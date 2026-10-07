//! Decodificación de miniaturas fuera del hilo de dibujo.
//!
//! El detalle importante no es que haya hilos: es la **cancelación**. Cada
//! fotograma el hilo principal reemplaza la lista de deseos por lo que se ve
//! ahora mismo, en orden de prioridad. Lo que salió de pantalla desaparece de
//! la cola sin que nadie tenga que cancelarlo: un scroll violento genera trabajo
//! constante, no proporcional a la distancia recorrida.

use grimorio_core::thumbs::{PackReader, ThumbRef};
use std::collections::VecDeque;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

pub struct Decodificada {
    pub indice: u32,
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

struct Cola {
    deseados: VecDeque<u32>,
    fin: bool,
}

pub struct Pool {
    estado: Arc<(Mutex<Cola>, Condvar)>,
    /// En `Option` para poder cerrarlo antes de esperar a los hilos: al cerrar
    /// el receptor, cualquier `send` bloqueado falla y el hilo sale solo.
    rx: Option<Receiver<Decodificada>>,
    hilos: Vec<JoinHandle<()>>,
    decodificadas: std::cell::Cell<u64>,
}

impl Pool {
    pub fn nuevo(pack: Arc<PackReader>, refs: Arc<Vec<Option<ThumbRef>>>, n_hilos: usize) -> Pool {
        let estado = Arc::new((
            Mutex::new(Cola {
                deseados: VecDeque::new(),
                fin: false,
            }),
            Condvar::new(),
        ));
        // Canal acotado: la memoria de una miniatura decodificada son ~400 KB.
        // Sin tope, los hilos decodifican mucho más rápido de lo que el hilo de
        // dibujo consume y la cola se come la RAM (23 GB en una prueba de 15 s).
        // Bloquear al productor es exactamente lo que queremos: si nadie mira
        // esa miniatura todavía, que el hilo espere o coja otra tarea.
        let (tx, rx) = sync_channel(n_hilos * 2);
        let mut hilos = Vec::with_capacity(n_hilos);
        for _ in 0..n_hilos {
            let estado = Arc::clone(&estado);
            let pack = Arc::clone(&pack);
            let refs = Arc::clone(&refs);
            let tx: SyncSender<Decodificada> = tx.clone();
            hilos.push(std::thread::spawn(move || {
                bucle(estado, pack, refs, tx);
            }));
        }
        drop(tx);
        Pool {
            estado,
            rx: Some(rx),
            hilos,
            decodificadas: std::cell::Cell::new(0),
        }
    }

    /// Reemplaza la lista de deseos. El orden es el orden de prioridad.
    pub fn pedir(&self, lista: impl IntoIterator<Item = u32>) {
        let (m, cv) = &*self.estado;
        let mut cola = m.lock().unwrap();
        cola.deseados.clear();
        cola.deseados.extend(lista);
        if !cola.deseados.is_empty() {
            cv.notify_all();
        }
    }

    pub fn pendientes(&self) -> usize {
        self.estado.0.lock().unwrap().deseados.len()
    }

    pub fn recibidas(&self) -> u64 {
        self.decodificadas.get()
    }

    /// Todo lo que haya llegado desde el fotograma anterior, sin bloquear.
    pub fn cosechar(&self, max: usize) -> Vec<Decodificada> {
        let mut out = Vec::new();
        while out.len() < max {
            match self.rx.as_ref().expect("receptor vivo").try_recv() {
                Ok(d) => out.push(d),
                Err(_) => break,
            }
        }
        self.decodificadas
            .set(self.decodificadas.get() + out.len() as u64);
        out
    }

    /// Espera activa acotada: solo la usa el modo captura, nunca la interfaz.
    pub fn esperar(&self, ms: u64) -> Vec<Decodificada> {
        let hasta = std::time::Instant::now() + std::time::Duration::from_millis(ms);
        let mut out = Vec::new();
        while std::time::Instant::now() < hasta {
            match self
                .rx
                .as_ref()
                .expect("receptor vivo")
                .recv_timeout(std::time::Duration::from_millis(5))
            {
                Ok(d) => out.push(d),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if self.pendientes() == 0 {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        self.decodificadas
            .set(self.decodificadas.get() + out.len() as u64);
        out
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        {
            let (m, cv) = &*self.estado;
            let mut cola = m.lock().unwrap();
            cola.fin = true;
            cv.notify_all();
        }
        // Cerrar el canal desbloquea a quien estuviera esperando a enviar.
        self.rx.take();
        for h in self.hilos.drain(..) {
            let _ = h.join();
        }
    }
}

fn bucle(
    estado: Arc<(Mutex<Cola>, Condvar)>,
    pack: Arc<PackReader>,
    refs: Arc<Vec<Option<ThumbRef>>>,
    tx: SyncSender<Decodificada>,
) {
    loop {
        let indice = {
            let (m, cv) = &*estado;
            let mut cola = m.lock().unwrap();
            loop {
                if cola.fin {
                    return;
                }
                if let Some(i) = cola.deseados.pop_front() {
                    break i;
                }
                cola = cv.wait(cola).unwrap();
            }
        };

        let Some(Some(tref)) = refs.get(indice as usize).copied() else {
            continue;
        };
        let Some(bytes) = pack.get(tref) else {
            continue;
        };
        // Decodificar desde el mmap: sin `open`, sin `read`, sin copia previa.
        let Ok(img) = image::load_from_memory(bytes) else {
            continue;
        };
        let rgba = img.to_rgba8();
        let (w, h) = (rgba.width(), rgba.height());
        if tx
            .send(Decodificada {
                indice,
                w,
                h,
                rgba: rgba.into_raw(),
            })
            .is_err()
        {
            return;
        }
    }
}
