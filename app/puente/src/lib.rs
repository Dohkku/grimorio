//! Puente entre `grimorio-core` y la interfaz.
//!
//! Tiene dos caminos, y la separación es deliberada:
//!
//!   * **El bus de comandos.** JSON en las dos direcciones, asíncrono, sobre un
//!     hilo propio que es el único dueño de la `Library`. Baja frecuencia: son
//!     acciones de una persona. Que sea JSON lo hace inspeccionable y deja la
//!     puerta abierta a que un plugin hable el mismo idioma.
//!   * **El camino caliente.** La malla lee la vista actual por índice, sin
//!     copias y sin serializar nada: los bytes de una miniatura son un puntero
//!     dentro del pack mapeado en memoria. Doscientas celdas por fotograma no
//!     pueden pasar por un canal.
//!
//! Reglas de esta capa:
//!   * Ningún tipo de Rust cruza la frontera. Enteros, punteros y bytes.
//!   * Ninguna llamada de la interfaz toca el disco ni SQLite: una consulta
//!     sobre 100.000 elementos tarda 85 ms, que son quince fotogramas perdidos.
//!   * Nada de `panic!` cruzando la frontera: todo error se vuelve un evento.

use grimorio_core::{Error, Library, Query, Result};
use std::ffi::{c_char, c_void, CStr};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

mod comandos;
mod derivar;
mod eventos;
mod vista;

pub use comandos::Comando;
pub use vista::Vista;

// ------------------------------------------------------------------ callback

/// El puntero de vuelta a C++. Se llama desde el hilo trabajador, así que quien
/// esté al otro lado tiene que reenviarlo a su hilo de interfaz.
pub type EventoFn = extern "C" fn(usuario: *mut c_void, json: *const u8, len: u32);

#[derive(Clone, Copy)]
struct Aviso {
    f: EventoFn,
    usuario: *mut c_void,
}

// SAFETY: `usuario` es un puntero opaco que solo se devuelve tal cual a quien lo
// dio. El contrato con C++ es que ese objeto viva hasta `grim_parar` y que su
// callback sea seguro de llamar desde otro hilo.
unsafe impl Send for Aviso {}

impl Aviso {
    fn manda(&self, json: &[u8]) {
        (self.f)(self.usuario, json.as_ptr(), json.len() as u32);
    }
}

// -------------------------------------------------------------------- núcleo

enum Peticion {
    Cmd { id: u64, cmd: Comando },
    Parar,
}

pub struct Nucleo {
    tx: Sender<Peticion>,
    hilo: Option<std::thread::JoinHandle<()>>,
    /// El vigía de las carpetas vigiladas, y cómo decirle que pare.
    vigia: Option<std::thread::JoinHandle<()>>,
    parar_vigia: Arc<std::sync::atomic::AtomicBool>,
    /// La vista publicada. El hilo trabajador la sustituye entera; la interfaz
    /// se lleva una referencia contada y la suelta cuando ha terminado, así que
    /// nunca lee una que se esté rehaciendo.
    vista: Arc<Mutex<Arc<Vista>>>,
    siguiente_id: AtomicU64,
    /// Copia del callback, para poder avisar de un comando mal escrito sin
    /// pasar por el hilo.
    aviso: Aviso,
}

impl Nucleo {
    fn arrancar(raiz: PathBuf, aviso: Aviso) -> Result<Nucleo> {
        // Abrir la biblioteca aquí y no en el hilo: si la ruta no vale, quien
        // llama se entera enseguida y con un error, no con un evento tardío.
        let mut lib = Library::open(&raiz)?;
        let vista_inicial = Arc::new(Vista::desde(&mut lib, &Query::default())?);
        let vista = Arc::new(Mutex::new(vista_inicial));

        let (tx, rx) = std::sync::mpsc::channel::<Peticion>();
        let parar_vigia = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let vigia = {
            let tx = tx.clone();
            let raiz = raiz.clone();
            let parar = Arc::clone(&parar_vigia);
            std::thread::Builder::new()
                .name("grimorio-vigia".into())
                .spawn(move || vigilar(raiz, tx, parar))
                .ok()
        };
        let vista_hilo = Arc::clone(&vista);
        let aviso_hilo = aviso;
        let hilo = std::thread::Builder::new()
            .name("grimorio-nucleo".into())
            .spawn(move || trabajar(lib, rx, vista_hilo, aviso_hilo))
            .map_err(|e| Error::Invalid(format!("no pude arrancar el hilo del núcleo: {e}")))?;

        Ok(Nucleo {
            tx,
            hilo: Some(hilo),
            vigia,
            parar_vigia,
            vista,
            siguiente_id: AtomicU64::new(1),
            aviso,
        })
    }

    fn vista_actual(&self) -> Arc<Vista> {
        match self.vista.lock() {
            Ok(v) => Arc::clone(&v),
            // Un `poison` significa que el hilo trabajador se cayó a mitad de
            // publicar. La vista de dentro sigue siendo válida —es inmutable—
            // así que se devuelve igual en vez de tirar la interfaz.
            Err(envenenado) => Arc::clone(&envenenado.into_inner()),
        }
    }
}

impl Drop for Nucleo {
    fn drop(&mut self) {
        self.parar_vigia.store(true, Ordering::Relaxed);
        if let Some(h) = self.vigia.take() {
            let _ = h.join();
        }
        let _ = self.tx.send(Peticion::Parar);
        if let Some(h) = self.hilo.take() {
            let _ = h.join();
        }
    }
}

/// El vigía: cada pocos segundos mira las carpetas vigiladas y, cuando hay algo
/// nuevo **y ha dejado de crecer**, lo manda importar por la misma cola que
/// cualquier otra orden.
///
/// «Dejado de crecer» es el mismo tamaño durante dos segundos: una descarga a
/// medias o un vídeo que se está exportando no se importan cortados. Y se
/// espera a que todo lo nuevo de una carpeta esté quieto antes de mandar nada,
/// porque la marca de «visto hasta aquí» es una sola fecha por carpeta: si se
/// importara lo quieto y se marcara, lo que aún crecía quedaría por detrás de
/// la marca y no entraría nunca.
fn vigilar(raiz: PathBuf, tx: Sender<Peticion>, parar: Arc<std::sync::atomic::AtomicBool>) {
    use std::collections::HashMap;
    use std::time::{Duration, Instant};
    const CADA: Duration = Duration::from_secs(3);
    const QUIETO: Duration = Duration::from_secs(2);
    // Lo visto en la pasada anterior: tamaño y desde cuándo lo tiene.
    let mut tamanos: HashMap<PathBuf, (u64, Instant)> = HashMap::new();
    // Lo mandado a importar y todavía no marcado: id -> hasta.
    let mut en_camino: HashMap<String, u64> = HashMap::new();
    'fuera: loop {
        let espera = Instant::now();
        while espera.elapsed() < CADA {
            if parar.load(Ordering::Relaxed) {
                break 'fuera;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let Ok(todas) = grimorio_core::vigiladas::cargar(&raiz) else {
            continue;
        };
        for v in todas {
            if let Some(hasta) = en_camino.get(&v.id) {
                if v.desde_ms < *hasta {
                    continue; // la importación anterior aún no ha terminado
                }
                en_camino.remove(&v.id);
            }
            let inicio = grimorio_core::time::now_ms();
            let nuevos = grimorio_core::vigiladas::nuevos(&v);
            if nuevos.is_empty() {
                continue;
            }
            let mut listos = Vec::new();
            let mut crece = false;
            for (p, len) in &nuevos {
                match tamanos.get(p) {
                    Some((l, t)) if *l == *len && t.elapsed() >= QUIETO => listos.push(p.clone()),
                    Some((l, _)) if *l == *len => crece = true,
                    _ => {
                        tamanos.insert(p.clone(), (*len, Instant::now()));
                        crece = true;
                    }
                }
            }
            if crece || listos.is_empty() {
                continue;
            }
            for p in &listos {
                tamanos.remove(p);
            }
            let cmd = Comando::ImportarVigilada {
                id: v.id.clone(),
                rutas: listos.iter().map(|p| p.to_string_lossy().into_owned()).collect(),
                hasta: inicio,
            };
            if tx.send(Peticion::Cmd { id: 0, cmd }).is_err() {
                break 'fuera;
            }
            en_camino.insert(v.id.clone(), inicio);
        }
    }
}

/// El hilo trabajador: dueño único de la `Library` y de todo lo que toca disco.
fn trabajar(mut lib: Library, rx: Receiver<Peticion>, vista: Arc<Mutex<Arc<Vista>>>, aviso: Aviso) {
    eventos::listo(&aviso, &lib);
    eventos::carpetas(&aviso, &lib);
    eventos::grupos(&aviso, &lib);
    eventos::vigiladas(&aviso, &lib);
    eventos::vista(&aviso, 0, &vista_len(&vista));

    while let Ok(p) = rx.recv() {
        let (id, cmd) = match p {
            Peticion::Parar => break,
            Peticion::Cmd { id, cmd } => (id, cmd),
        };
        protegido(&aviso, id, || {
            comandos::ejecutar(&mut lib, &cmd, id, &aviso, &vista)
        });
    }
}

/// Ejecuta un comando de forma que ni su error ni su pánico se lleven el hilo.
///
/// Sin esto, un pánico en cualquier comando —un `unwrap` olvidado en el núcleo,
/// una imagen rara— desenrollaba `trabajar` entero: el hilo moría, el canal se
/// quedaba sin nadie al otro lado y la interfaz seguía pintando la última vista
/// sin que ningún botón volviera a hacer nada, ni un mensaje que lo explicara.
/// Aquí el pánico se convierte en un evento de error más, como hace
/// `grim_derivar`, y el hilo sigue con el siguiente comando.
///
/// `AssertUnwindSafe` es honesto en este caso: lo único que comparte el cierre
/// es la `Library`, cuyo estado en disco lo protege SQLite (una transacción a
/// medias se deshace sola), y la vista publicada, que solo se sustituye entera.
/// Lo peor que queda es una vista algo vieja, que el siguiente comando rehace.
///
/// Ojo: en el perfil `release` el pánico es `abort` y esto no llega a actuar;
/// el proceso cae entero, que al menos no es silencioso. Protege las pruebas,
/// `bench-fast` y cualquier compilación con `panic = "unwind"`.
fn protegido(aviso: &Aviso, id: u64, f: impl FnOnce() -> Result<()>) {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(Ok(())) => {}
        Ok(Err(e)) => eventos::error(aviso, id, &e.to_string()),
        Err(panico) => {
            let motivo = panico
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| panico.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "sin detalle".into());
            eventos::error(
                aviso,
                id,
                &format!("el núcleo falló con este comando ({motivo}); lo demás sigue en pie"),
            );
        }
    }
}

fn vista_len(vista: &Arc<Mutex<Arc<Vista>>>) -> usize {
    match vista.lock() {
        Ok(v) => v.len(),
        Err(e) => e.into_inner().len(),
    }
}

// ---------------------------------------------------------------- frontera C

/// Arranca el núcleo sobre una biblioteca. Devuelve nulo si la ruta no lo es.
///
/// # Safety
/// `ruta` debe ser una cadena C válida. `usuario` debe seguir vivo hasta
/// `grim_parar`, y `cb` debe poder llamarse desde otro hilo.
#[no_mangle]
pub unsafe extern "C" fn grim_iniciar(
    ruta: *const c_char,
    cb: EventoFn,
    usuario: *mut c_void,
) -> *mut Nucleo {
    if ruta.is_null() {
        return std::ptr::null_mut();
    }
    let Ok(ruta) = CStr::from_ptr(ruta).to_str() else {
        return std::ptr::null_mut();
    };
    match Nucleo::arrancar(PathBuf::from(ruta), Aviso { f: cb, usuario }) {
        Ok(n) => Box::into_raw(Box::new(n)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// # Safety
/// `n` debe venir de `grim_iniciar` y no haberse parado ya.
#[no_mangle]
pub unsafe extern "C" fn grim_parar(n: *mut Nucleo) {
    if !n.is_null() {
        drop(Box::from_raw(n));
    }
}

/// Encola un comando en JSON. Devuelve su número, que vuelve en el evento de
/// respuesta; 0 si el JSON no se entiende.
///
/// # Safety
/// `n` debe venir de `grim_iniciar`; `json` debe apuntar a `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn grim_mandar(n: *mut Nucleo, json: *const u8, len: u32) -> u64 {
    if n.is_null() || json.is_null() {
        return 0;
    }
    let nucleo = &*n;
    let bytes = std::slice::from_raw_parts(json, len as usize);
    let cmd: Comando = match serde_json::from_slice(bytes) {
        Ok(c) => c,
        Err(e) => {
            // El error del comando mal formado se avisa igual: si no, un fallo
            // de escritura en la interfaz se convierte en un botón que no hace
            // nada y nadie sabe por qué.
            eventos::error(&nucleo.aviso, 0, &format!("comando ilegible: {e}"));
            return 0;
        }
    };
    let id = nucleo.siguiente_id.fetch_add(1, Ordering::Relaxed);
    match nucleo.tx.send(Peticion::Cmd { id, cmd }) {
        Ok(()) => id,
        Err(_) => 0,
    }
}

// ------------------------------------------------------------- camino rápido

/// Toma una referencia a la vista actual. Hay que soltarla con
/// `grim_vista_soltar`; mientras esté tomada, sus punteros son válidos.
///
/// # Safety
/// `n` debe venir de `grim_iniciar`.
#[no_mangle]
pub unsafe extern "C" fn grim_vista(n: *const Nucleo) -> *const Vista {
    if n.is_null() {
        return std::ptr::null();
    }
    Arc::into_raw((*n).vista_actual())
}

/// Otra referencia a la misma vista. Lo usa el proveedor de imágenes: la vista
/// puede cambiar mientras una miniatura viaja por la cola de decodificación, y
/// sin una referencia propia los bytes se irían de debajo.
///
/// # Safety
/// `v` debe venir de `grim_vista` o de esta misma función.
#[no_mangle]
pub unsafe extern "C" fn grim_vista_clonar(v: *const Vista) -> *const Vista {
    if v.is_null() {
        return std::ptr::null();
    }
    Arc::increment_strong_count(v);
    v
}

/// # Safety
/// `v` debe venir de `grim_vista` y soltarse una sola vez.
#[no_mangle]
pub unsafe extern "C" fn grim_vista_soltar(v: *const Vista) {
    if !v.is_null() {
        drop(Arc::from_raw(v));
    }
}

macro_rules! campo {
    ($v:expr, $i:expr, $default:expr) => {{
        if $v.is_null() {
            return $default;
        }
        match (&*$v).hit($i) {
            Some(h) => h,
            None => return $default,
        }
    }};
}

/// # Safety
/// `v` debe venir de `grim_vista`.
#[no_mangle]
pub unsafe extern "C" fn grim_vista_n(v: *const Vista) -> usize {
    if v.is_null() {
        0
    } else {
        (*v).len()
    }
}

/// # Safety
/// `v` debe venir de `grim_vista`.
#[no_mangle]
pub unsafe extern "C" fn grim_vista_ancho(v: *const Vista, i: usize) -> u32 {
    campo!(v, i, 0).width
}

/// # Safety
/// `v` debe venir de `grim_vista`.
#[no_mangle]
pub unsafe extern "C" fn grim_vista_alto(v: *const Vista, i: usize) -> u32 {
    campo!(v, i, 0).height
}

/// # Safety
/// `v` debe venir de `grim_vista`.
#[no_mangle]
pub unsafe extern "C" fn grim_vista_estrellas(v: *const Vista, i: usize) -> u8 {
    campo!(v, i, 0).stars
}

/// La extensión, apuntando dentro de la vista. No copia.
///
/// # Safety
/// `v` debe venir de `grim_vista`; el puntero vale mientras la vista viva.
#[no_mangle]
pub unsafe extern "C" fn grim_vista_ext(v: *const Vista, i: usize, len: *mut u32) -> *const u8 {
    let vista = match v.as_ref() {
        Some(x) => x,
        None => return std::ptr::null(),
    };
    match vista.hit(i) {
        Some(h) => {
            if !len.is_null() {
                *len = h.ext.len() as u32;
            }
            h.ext.as_ptr()
        }
        None => std::ptr::null(),
    }
}

/// La familia del elemento, como número, para que la celda sepa qué dibujar.
///
/// Va como número y no como cadena porque esto es el camino caliente: doscientas
/// celdas por fotograma preguntando, y una cadena por celda serían doscientas
/// asignaciones de memoria que no hacen falta. El orden es el del enum de Rust,
/// y `grimorio.h` lo repite para que el lado de C++ no lo adivine.
///
/// # Safety
/// `v` debe venir de `grim_vista`.
#[no_mangle]
pub unsafe extern "C" fn grim_vista_familia(v: *const Vista, i: usize) -> u8 {
    use grimorio_core::Familia;
    match campo!(v, i, 0).kind {
        Familia::Imagen => 0,
        Familia::Raw => 1,
        Familia::Video => 2,
        Familia::Audio => 3,
        Familia::Documento => 4,
        Familia::Tipografia => 5,
        Familia::Modelo => 6,
    }
}

/// Si el elemento está marcado como contenido adulto.
///
/// # Safety
/// `v` debe venir de `grim_vista`.
#[no_mangle]
pub unsafe extern "C" fn grim_vista_adulto(v: *const Vista, i: usize) -> u8 {
    if v.is_null() {
        return 0;
    }
    match (*v).hit(i) {
        Some(h) if h.adulto => 1,
        _ => 0,
    }
}

/// Duración en segundos, o 0 si el archivo no dura.
///
/// # Safety
/// `v` debe venir de `grim_vista`.
#[no_mangle]
pub unsafe extern "C" fn grim_vista_duracion(v: *const Vista, i: usize) -> u32 {
    campo!(v, i, 0).duracion_s.unwrap_or(0)
}

/// Peso del original en bytes. Lo pide la vista en lista, que lo enseña en
/// cada fila sin cargar ningún `item.json`.
///
/// # Safety
/// `v` debe venir de `grim_vista`.
#[no_mangle]
pub unsafe extern "C" fn grim_vista_peso(v: *const Vista, i: usize) -> u64 {
    campo!(v, i, 0).size
}

/// Cuándo se importó, en milisegundos Unix.
///
/// # Safety
/// `v` debe venir de `grim_vista`.
#[no_mangle]
pub unsafe extern "C" fn grim_vista_importado(v: *const Vista, i: usize) -> u64 {
    campo!(v, i, 0).imported_at_ms
}

/// Color dominante en 0x00RRGGBB, o 0xFFFFFFFF si el elemento no tiene paleta.
///
/// # Safety
/// `v` debe venir de `grim_vista`.
#[no_mangle]
pub unsafe extern "C" fn grim_vista_dominante(v: *const Vista, i: usize) -> u32 {
    const SIN_COLOR: u32 = 0xFFFF_FFFF;
    let h = campo!(v, i, SIN_COLOR);
    match h.dominant {
        Some(c) => ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | c[2] as u32,
        None => SIN_COLOR,
    }
}

/// # Safety
/// `v` debe venir de `grim_vista`. El puntero vale hasta soltarla.
#[no_mangle]
pub unsafe extern "C" fn grim_vista_nombre(v: *const Vista, i: usize, len: *mut u32) -> *const u8 {
    if !len.is_null() {
        *len = 0;
    }
    let h = campo!(v, i, std::ptr::null());
    if !len.is_null() {
        *len = h.name.len() as u32;
    }
    h.name.as_ptr()
}

/// # Safety
/// `v` debe venir de `grim_vista`. El puntero vale hasta soltarla.
#[no_mangle]
pub unsafe extern "C" fn grim_vista_id(v: *const Vista, i: usize, len: *mut u32) -> *const u8 {
    if !len.is_null() {
        *len = 0;
    }
    let h = campo!(v, i, std::ptr::null());
    if !len.is_null() {
        *len = h.id.len() as u32;
    }
    h.id.as_ptr()
}

/// Bytes JPEG de la miniatura, apuntando dentro del pack mapeado. Cero copias.
///
/// # Safety
/// `v` debe venir de `grim_vista`. El puntero vale hasta soltarla y no se libera.
#[no_mangle]
pub unsafe extern "C" fn grim_vista_thumb(v: *const Vista, i: usize, len: *mut u32) -> *const u8 {
    if !len.is_null() {
        *len = 0;
    }
    if v.is_null() {
        return std::ptr::null();
    }
    match (*v).thumb(i) {
        Some(bytes) => {
            if !len.is_null() {
                *len = bytes.len() as u32;
            }
            bytes.as_ptr()
        }
        None => std::ptr::null(),
    }
}

/// Posición de un id dentro de la vista, o `usize::MAX` si no está.
///
/// # Safety
/// `v` debe venir de `grim_vista`; `id` debe apuntar a `len` bytes UTF-8.
#[no_mangle]
pub unsafe extern "C" fn grim_vista_indice_de(v: *const Vista, id: *const u8, len: u32) -> usize {
    if v.is_null() || id.is_null() {
        return usize::MAX;
    }
    let bytes = std::slice::from_raw_parts(id, len as usize);
    match std::str::from_utf8(bytes) {
        Ok(s) => (*v).indice_de(s).unwrap_or(usize::MAX),
        Err(_) => usize::MAX,
    }
}

/// Escribe en `salida` la ruta de la previsualización de 1024 px del elemento
/// `i`, y devuelve cuántos bytes ocupa. Si no cabe, devuelve 0.
///
/// Escribe en el buffer de quien llama en vez de devolver un puntero porque la
/// ruta se calcula al vuelo: guardar 100.000 rutas en memoria para que una sola
/// se use en el visor sería tirar seis megas por nada.
///
/// # Safety
/// `v` debe venir de `grim_vista`; `salida` debe apuntar a `cap` bytes escribibles.
#[no_mangle]
pub unsafe extern "C" fn grim_vista_previa(
    v: *const Vista,
    i: usize,
    salida: *mut u8,
    cap: u32,
) -> u32 {
    if v.is_null() || salida.is_null() {
        return 0;
    }
    let Some(ruta) = (*v).ruta_previa(i) else {
        return 0;
    };
    let bytes = ruta.as_bytes();
    if bytes.len() > cap as usize {
        return 0;
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), salida, bytes.len());
    bytes.len() as u32
}

/// La ruta del archivo original, para abrirlo o reproducirlo.
///
/// # Safety
/// `v` debe venir de `grim_vista`; `salida` tiene que tener `cap` bytes.
#[no_mangle]
pub unsafe extern "C" fn grim_vista_ruta(
    v: *const Vista,
    i: usize,
    salida: *mut u8,
    cap: u32,
) -> u32 {
    if v.is_null() || salida.is_null() {
        return 0;
    }
    let Some(ruta) = (*v).ruta_original(i) else {
        return 0;
    };
    let bytes = ruta.as_bytes();
    if bytes.len() > cap as usize {
        return 0;
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), salida, bytes.len());
    bytes.len() as u32
}

/// Decodifica un archivo de imagen entero a RGBA de 8 bits, para el zoom del
/// visor y el cuentagotas.
///
/// Es la vía de reserva: Qt abre primero lo que sabe, y aquí llega lo que no
/// —WebP sin el paquete de formatos de Qt, que en esta máquina no está y era
/// casi toda la biblioteca—. Un RAW da su vista previa, que es lo único que
/// hay sin revelar.
///
/// Devuelve los píxeles (liberar con `grim_pixeles_soltar`) y rellena ancho y
/// alto; null si no se pudo.
///
/// # Safety
/// `ruta` tiene que apuntar a `len` bytes de UTF-8; `ancho` y `alto`, a
/// enteros donde escribir.
#[no_mangle]
pub unsafe extern "C" fn grim_decodificar(
    ruta: *const u8,
    len: u32,
    ancho: *mut u32,
    alto: *mut u32,
) -> *mut u8 {
    if ruta.is_null() || ancho.is_null() || alto.is_null() {
        return std::ptr::null_mut();
    }
    let Ok(ruta) = std::str::from_utf8(std::slice::from_raw_parts(ruta, len as usize)) else {
        return std::ptr::null_mut();
    };
    let ruta = std::path::Path::new(ruta);
    let Ok(bytes) = std::fs::read(ruta) else {
        return std::ptr::null_mut();
    };
    let ext = ruta
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    // Los de diseño, con su herramienta (y a un máximo de 2048 px).
    if grimorio_core::diseno::es_diseno(&ext) {
        let Ok(img) = grimorio_core::diseno::previa(ruta, &ext) else {
            return std::ptr::null_mut();
        };
        let rgba = img.to_rgba8();
        *ancho = rgba.width();
        *alto = rgba.height();
        return Box::into_raw(rgba.into_raw().into_boxed_slice()) as *mut u8;
    }
    let datos: &[u8] = if grimorio_core::medio::familia_de(&ext)
        == Some(grimorio_core::medio::Familia::Raw)
    {
        match grimorio_core::raw::previa(&bytes) {
            Some(j) => j,
            None => return std::ptr::null_mut(),
        }
    } else {
        &bytes
    };
    let Ok(img) = grimorio_core::image_ops::decode(datos) else {
        return std::ptr::null_mut();
    };
    let rgba = img.to_rgba8();
    *ancho = rgba.width();
    *alto = rgba.height();
    let caja = rgba.into_raw().into_boxed_slice();
    Box::into_raw(caja) as *mut u8
}

/// Crea una biblioteca en `ruta` —o se asegura de que ya hay una— para poder
/// abrirla. El nombre visible es el de la carpeta. Devuelve 1 si se puede
/// abrir, 0 si no.
///
/// # Safety
/// `ruta` tiene que apuntar a `len` bytes de UTF-8.
#[no_mangle]
pub unsafe extern "C" fn grim_crear_biblioteca(ruta: *const u8, len: u32) -> i32 {
    if ruta.is_null() {
        return 0;
    }
    let Ok(ruta) = std::str::from_utf8(std::slice::from_raw_parts(ruta, len as usize)) else {
        return 0;
    };
    let ruta = std::path::Path::new(ruta);
    let nombre = ruta
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.trim_end_matches(".grimorio").to_string())
        .unwrap_or_else(|| "biblioteca".to_string());
    i32::from(Library::open_or_create(ruta, &nombre).is_ok())
}

/// Si en `ruta` hay una biblioteca de Grimorio.
///
/// # Safety
/// `ruta` tiene que apuntar a `len` bytes de UTF-8.
#[no_mangle]
pub unsafe extern "C" fn grim_es_biblioteca(ruta: *const u8, len: u32) -> i32 {
    if ruta.is_null() {
        return 0;
    }
    let Ok(ruta) = std::str::from_utf8(std::slice::from_raw_parts(ruta, len as usize)) else {
        return 0;
    };
    i32::from(Library::open(std::path::Path::new(ruta)).is_ok())
}

/// Suelta lo que dio `grim_decodificar`.
///
/// # Safety
/// `p` tiene que venir de `grim_decodificar` y `bytes` ser ancho × alto × 4.
#[no_mangle]
pub unsafe extern "C" fn grim_pixeles_soltar(p: *mut u8, bytes: usize) {
    if p.is_null() {
        return;
    }
    drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(p, bytes)));
}

/// Cuántos bytes ocupa el pack mapeado. Solo para el informe del banco.
///
/// # Safety
/// `v` debe venir de `grim_vista`.
#[no_mangle]
pub unsafe extern "C" fn grim_vista_bytes_pack(v: *const Vista) -> u64 {
    if v.is_null() {
        0
    } else {
        (*v).bytes_pack()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grimorio_core::item::{Item, OriginMode};
    use std::ffi::CString;
    use std::time::{Duration, Instant};

    /// El otro lado de la frontera, fingido: apunta todo lo que llega.
    struct Recolector {
        eventos: Mutex<Vec<serde_json::Value>>,
    }

    extern "C" fn recoge(usuario: *mut c_void, json: *const u8, len: u32) {
        // SAFETY: `usuario` es el `Recolector` que pasamos a `grim_iniciar`, y
        // vive más que el núcleo porque está en la pila de la prueba.
        let r = unsafe { &*(usuario as *const Recolector) };
        let bytes = unsafe { std::slice::from_raw_parts(json, len as usize) };
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(bytes) {
            r.eventos.lock().unwrap().push(v);
        } else {
            panic!("el núcleo mandó algo que no es JSON");
        }
    }

    impl Recolector {
        fn nuevo() -> Recolector {
            Recolector {
                eventos: Mutex::new(Vec::new()),
            }
        }

        /// Espera a que llegue un evento de ese tipo. El bus es asíncrono, así
        /// que una prueba que no espera es una prueba que falla a veces.
        fn espera(&self, tipo: &str) -> serde_json::Value {
            let hasta = Instant::now() + Duration::from_secs(5);
            loop {
                if let Some(v) = self
                    .eventos
                    .lock()
                    .unwrap()
                    .iter()
                    .rev()
                    .find(|v| v["tipo"] == tipo)
                {
                    return v.clone();
                }
                if Instant::now() > hasta {
                    let vistos: Vec<String> = self
                        .eventos
                        .lock()
                        .unwrap()
                        .iter()
                        .map(|v| v.to_string())
                        .collect();
                    panic!("no llegó ningún «{tipo}»; llegaron: {vistos:#?}");
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        }

        fn olvida(&self) {
            self.eventos.lock().unwrap().clear();
        }
    }

    fn biblioteca_con(n: usize) -> (tempfile::TempDir, PathBuf, Vec<String>) {
        let dir = tempfile::tempdir().unwrap();
        let raiz = dir.path().join("p.grimorio");
        let mut lib = Library::create(&raiz, "p").unwrap();
        let mut ids = Vec::new();
        for i in 0..n {
            let item = Item::new(
                grimorio_core::id::new_id(),
                format!("foto {i}"),
                "jpg".into(),
                100,
                OriginMode::Copy,
            );
            ids.push(item.id.clone());
            std::fs::create_dir_all(lib.item_dir(&item.id)).unwrap();
            lib.save_item(&item).unwrap();
        }
        (dir, raiz, ids)
    }

    fn manda(n: *mut Nucleo, json: &str) -> u64 {
        unsafe { grim_mandar(n, json.as_ptr(), json.len() as u32) }
    }

    #[test]
    fn un_comando_que_entra_en_panico_se_vuelve_un_error_y_no_tumba_nada() {
        let r = Recolector::nuevo();
        let aviso = Aviso {
            f: recoge,
            usuario: &r as *const Recolector as *mut c_void,
        };
        protegido(&aviso, 42, || panic!("una imagen rara"));
        let e = r.espera("error");
        assert_eq!(e["cmd"], 42);
        assert!(e["mensaje"].as_str().unwrap().contains("una imagen rara"));

        // Y después del pánico, el siguiente comando se ejecuta con normalidad.
        r.olvida();
        let mut corrio = false;
        protegido(&aviso, 43, || {
            corrio = true;
            Ok(())
        });
        assert!(corrio);
        assert!(r.eventos.lock().unwrap().is_empty());
    }

    #[test]
    fn al_arrancar_cuenta_lo_que_hay() {
        let (_d, raiz, _) = biblioteca_con(3);
        let r = Recolector::nuevo();
        let ruta = CString::new(raiz.to_str().unwrap()).unwrap();
        let n = unsafe {
            grim_iniciar(
                ruta.as_ptr(),
                recoge,
                &r as *const Recolector as *mut c_void,
            )
        };
        assert!(!n.is_null());
        assert_eq!(r.espera("listo")["total"], 3);
        assert_eq!(r.espera("vista")["n"], 3);
        unsafe { grim_parar(n) };
    }

    #[test]
    fn una_ruta_que_no_es_biblioteca_no_arranca_nada() {
        let dir = tempfile::tempdir().unwrap();
        let r = Recolector::nuevo();
        let ruta = CString::new(dir.path().to_str().unwrap()).unwrap();
        let n = unsafe {
            grim_iniciar(
                ruta.as_ptr(),
                recoge,
                &r as *const Recolector as *mut c_void,
            )
        };
        assert!(n.is_null());
        unsafe { assert!(grim_iniciar(std::ptr::null(), recoge, std::ptr::null_mut()).is_null()) };
    }

    #[test]
    fn crear_una_carpeta_y_meter_elementos_dentro() {
        let (_d, raiz, ids) = biblioteca_con(3);
        let r = Recolector::nuevo();
        let ruta = CString::new(raiz.to_str().unwrap()).unwrap();
        let n = unsafe {
            grim_iniciar(
                ruta.as_ptr(),
                recoge,
                &r as *const Recolector as *mut c_void,
            )
        };
        r.espera("vista");
        r.olvida();

        manda(n, r#"{"cmd":"crear_carpeta","nombre":"Referencias"}"#);
        let carpeta = r.espera("hecho")["id"].as_str().unwrap().to_string();
        assert!(!carpeta.is_empty(), "crear devuelve el id de la carpeta");
        r.olvida();

        let cmd = format!(
            r#"{{"cmd":"carpetas_de_elementos","ids":["{}","{}"],"anadir":["{}"]}}"#,
            ids[0], ids[1], carpeta
        );
        manda(n, &cmd);
        let arbol = r.espera("carpetas");
        assert_eq!(arbol["conteos"][&carpeta], 2);
        r.olvida();

        // Y la vista filtrada por esa carpeta enseña justo esos dos.
        let cmd = format!(r#"{{"cmd":"consulta","q":{{"folder":"{carpeta}"}}}}"#);
        manda(n, &cmd);
        assert_eq!(r.espera("vista")["n"], 2);

        let v = unsafe { grim_vista(n) };
        assert_eq!(unsafe { grim_vista_n(v) }, 2);
        unsafe { grim_vista_soltar(v) };
        unsafe { grim_parar(n) };
    }

    /// Mover de una carpeta a otra es **una** operación, no dos.
    ///
    /// Con dos, el primer deshacer dejaba el elemento en las dos carpetas a la
    /// vez y hacía falta un segundo para terminar de volver. Un movimiento se
    /// deshace de una.
    #[test]
    fn mover_de_una_carpeta_a_otra_se_deshace_de_una_vez() {
        let (_d, raiz, ids) = biblioteca_con(2);
        let r = Recolector::nuevo();
        let ruta = CString::new(raiz.to_str().unwrap()).unwrap();
        let n = unsafe {
            grim_iniciar(
                ruta.as_ptr(),
                recoge,
                &r as *const Recolector as *mut c_void,
            )
        };
        r.espera("vista");
        r.olvida();

        manda(n, r#"{"cmd":"crear_carpeta","nombre":"Antes"}"#);
        let antes = r.espera("hecho")["id"].as_str().unwrap().to_string();
        r.olvida();
        manda(n, r#"{"cmd":"crear_carpeta","nombre":"Después"}"#);
        let despues = r.espera("hecho")["id"].as_str().unwrap().to_string();
        r.olvida();

        manda(
            n,
            &format!(
                r#"{{"cmd":"carpetas_de_elementos","ids":["{}"],"anadir":["{}"]}}"#,
                ids[0], antes
            ),
        );
        r.espera("carpetas");
        r.olvida();

        // El movimiento: entra en una y sale de la otra de una sola vez.
        manda(
            n,
            &format!(
                r#"{{"cmd":"carpetas_de_elementos","ids":["{}"],"anadir":["{}"],"quitar":["{}"]}}"#,
                ids[0], despues, antes
            ),
        );
        let arbol = r.espera("carpetas");
        assert_eq!(arbol["conteos"][&despues], 1, "tiene que estar en la nueva");
        assert!(
            arbol["conteos"][&antes].is_null() || arbol["conteos"][&antes] == 0,
            "y haber salido de la vieja"
        );
        let registro = r.espera("registro");
        assert_eq!(registro["deshacer"], "mover de carpeta");
        r.olvida();

        manda(n, r#"{"cmd":"deshacer"}"#);
        let arbol = r.espera("carpetas");
        assert_eq!(arbol["conteos"][&antes], 1, "un solo deshacer lo devuelve");
        assert!(
            arbol["conteos"][&despues].is_null() || arbol["conteos"][&despues] == 0,
            "y no lo deja en las dos a la vez"
        );

        unsafe { grim_parar(n) };
    }

    #[test]
    fn poner_estrellas_avisa_de_ese_elemento_y_no_de_los_demas() {
        let (_d, raiz, ids) = biblioteca_con(3);
        let r = Recolector::nuevo();
        let ruta = CString::new(raiz.to_str().unwrap()).unwrap();
        let n = unsafe {
            grim_iniciar(
                ruta.as_ptr(),
                recoge,
                &r as *const Recolector as *mut c_void,
            )
        };
        r.espera("vista");
        r.olvida();

        manda(
            n,
            &format!(r#"{{"cmd":"estrellas","ids":["{}"],"valor":4}}"#, ids[0]),
        );
        let ev = r.espera("item");
        assert_eq!(ev["id"], ids[0]);
        r.espera("hecho");

        // La vista publicada es de antes del cambio, a propósito: quien pinta
        // decide cuándo recogerla. Pero una consulta nueva sí lo trae.
        r.olvida();
        manda(n, r#"{"cmd":"consulta","q":{"min_stars":4}}"#);
        assert_eq!(r.espera("vista")["n"], 1);
        unsafe { grim_parar(n) };
    }

    #[test]
    fn un_comando_que_no_se_entiende_contesta_con_un_error() {
        let (_d, raiz, _) = biblioteca_con(1);
        let r = Recolector::nuevo();
        let ruta = CString::new(raiz.to_str().unwrap()).unwrap();
        let n = unsafe {
            grim_iniciar(
                ruta.as_ptr(),
                recoge,
                &r as *const Recolector as *mut c_void,
            )
        };
        r.espera("vista");
        r.olvida();

        assert_eq!(manda(n, r#"{"cmd":"formatear_disco"}"#), 0);
        assert!(r.espera("error")["mensaje"]
            .as_str()
            .unwrap()
            .contains("ilegible"));
        r.olvida();

        // Un comando bien escrito pero imposible también contesta.
        manda(n, r#"{"cmd":"borrar_carpeta","id":"no-existe"}"#);
        assert!(r.espera("error")["mensaje"]
            .as_str()
            .unwrap()
            .contains("no existe"));
        unsafe { grim_parar(n) };
    }

    #[test]
    fn una_carpeta_no_puede_meterse_dentro_de_su_nieta() {
        let (_d, raiz, _) = biblioteca_con(1);
        let r = Recolector::nuevo();
        let ruta = CString::new(raiz.to_str().unwrap()).unwrap();
        let n = unsafe {
            grim_iniciar(
                ruta.as_ptr(),
                recoge,
                &r as *const Recolector as *mut c_void,
            )
        };
        r.espera("vista");

        r.olvida();
        manda(n, r#"{"cmd":"crear_carpeta","nombre":"padre"}"#);
        let padre = r.espera("hecho")["id"].as_str().unwrap().to_string();
        r.olvida();
        manda(
            n,
            &format!(r#"{{"cmd":"crear_carpeta","nombre":"hija","padre":"{padre}"}}"#),
        );
        let hija = r.espera("hecho")["id"].as_str().unwrap().to_string();
        r.olvida();

        manda(
            n,
            &format!(r#"{{"cmd":"mover_carpeta","id":"{padre}","padre":"{hija}"}}"#),
        );
        assert!(r.espera("error")["mensaje"]
            .as_str()
            .unwrap()
            .contains("dentro de sí misma"));
        unsafe { grim_parar(n) };
    }

    #[test]
    fn colocar_una_carpeta_entre_sus_hermanas_llega_por_el_bus() {
        let (_dir, raiz, _) = biblioteca_con(0);
        let ruta = std::ffi::CString::new(raiz.to_str().unwrap()).unwrap();
        let r = Recolector::nuevo();
        let n = unsafe {
            grim_iniciar(
                ruta.as_ptr(),
                recoge,
                &r as *const Recolector as *mut c_void,
            )
        };
        r.espera("vista");

        let mut ids = Vec::new();
        for nombre in ["A", "B", "C"] {
            r.olvida();
            manda(
                n,
                &format!(r#"{{"cmd":"crear_carpeta","nombre":"{nombre}"}}"#),
            );
            ids.push(r.espera("hecho")["id"].as_str().unwrap().to_string());
        }
        // El árbol viaja como está guardado, sin ordenar: quien lo pinta lo
        // ordena por `pos`. La prueba hace lo mismo, que es justo lo que se
        // quiere comprobar.
        let nombres = |r: &Recolector| -> Vec<String> {
            let mut v: Vec<(i64, String)> = r.espera("carpetas")["arbol"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| {
                    (
                        c["pos"].as_i64().unwrap(),
                        c["nombre"].as_str().unwrap().to_string(),
                    )
                })
                .collect();
            v.sort();
            v.into_iter().map(|(_, n)| n).collect()
        };

        r.olvida();
        manda(
            n,
            &format!(
                r#"{{"cmd":"mover_carpeta","id":"{}","antes_de":"{}"}}"#,
                ids[2], ids[0]
            ),
        );
        r.espera("hecho");
        assert_eq!(nombres(&r), ["C", "A", "B"]);

        // Sin `antes_de`, la última: es lo que hace soltar sobre una carpeta en
        // vez de entre dos, y tiene que seguir funcionando igual.
        r.olvida();
        manda(
            n,
            &format!(r#"{{"cmd":"mover_carpeta","id":"{}"}}"#, ids[2]),
        );
        r.espera("hecho");
        assert_eq!(nombres(&r), ["A", "B", "C"]);

        // Y se deshace, como todo lo que toca el árbol.
        r.olvida();
        manda(n, r#"{"cmd":"deshacer"}"#);
        r.espera("hecho");
        assert_eq!(nombres(&r), ["C", "A", "B"]);
        unsafe { grim_parar(n) };
    }

    #[test]
    fn importar_mete_las_imagenes_y_avisa_del_progreso() {
        let (dir, raiz, _) = biblioteca_con(0);
        // Tres imágenes de verdad en disco: es lo que llega al soltar archivos
        // sobre la ventana.
        let fuente = dir.path().join("sueltas");
        std::fs::create_dir_all(&fuente).unwrap();
        for n in 0..3u8 {
            let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_fn(64, 48, |x, y| {
                image::Rgb([x as u8, y as u8, n.wrapping_mul(80)])
            }));
            let jpeg = grimorio_core::image_ops::encode_jpeg(&img.to_rgb8(), 80).unwrap();
            std::fs::write(fuente.join(format!("foto{n}.jpg")), jpeg).unwrap();
        }

        let r = Recolector::nuevo();
        let ruta = CString::new(raiz.to_str().unwrap()).unwrap();
        let n = unsafe {
            grim_iniciar(
                ruta.as_ptr(),
                recoge,
                &r as *const Recolector as *mut c_void,
            )
        };
        r.espera("vista");
        r.olvida();

        manda(n, r#"{"cmd":"crear_carpeta","nombre":"Entrada"}"#);
        let carpeta = r.espera("hecho")["id"].as_str().unwrap().to_string();
        r.olvida();

        let cmd = format!(
            r#"{{"cmd":"importar","rutas":[{}],"carpeta":"{}","etiquetas":["soltadas"]}}"#,
            texto_json(&fuente),
            carpeta
        );
        manda(n, &cmd);

        let hecho = r.espera("hecho");
        assert!(
            hecho["mensaje"].as_str().unwrap().contains("3 importadas")
                || hecho["mensaje"].as_str().unwrap().contains("3 importados"),
            "salió {}",
            hecho["mensaje"]
        );
        assert_eq!(r.espera("vista")["n"], 3, "la malla se rehace sola");
        assert_eq!(r.espera("carpetas")["conteos"][&carpeta], 3);
        assert!(r.espera("progreso")["total"].as_u64().unwrap() >= 1);

        // Y las miniaturas están donde la malla las va a buscar.
        let v = unsafe { grim_vista(n) };
        let mut len = 0u32;
        assert!(!unsafe { grim_vista_thumb(v, 0, &mut len) }.is_null());
        assert!(len > 0, "el elemento importado tiene que traer miniatura");
        unsafe { grim_vista_soltar(v) };
        unsafe { grim_parar(n) };
    }

    #[test]
    fn lo_pegado_se_mueve_y_recuerda_de_donde_viene() {
        // Lo que llega por el portapapeles o la red es un temporal: se mueve,
        // no se copia, y la URL queda en el elemento.
        let (dir, raiz, _) = biblioteca_con(0);
        let temporal = dir.path().join("pegado.jpg");
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_fn(32, 32, |x, y| {
            image::Rgb([x as u8 * 8, y as u8 * 8, 90])
        }));
        let jpeg = grimorio_core::image_ops::encode_jpeg(&img.to_rgb8(), 80).unwrap();
        std::fs::write(&temporal, jpeg).unwrap();

        let r = Recolector::nuevo();
        let ruta = CString::new(raiz.to_str().unwrap()).unwrap();
        let n = unsafe {
            grim_iniciar(
                ruta.as_ptr(),
                recoge,
                &r as *const Recolector as *mut c_void,
            )
        };
        r.espera("vista");
        r.olvida();
        let cmd = format!(
            r#"{{"cmd":"importar","rutas":[{}],"origen":"https://ejemplo.org/obra","mover":true}}"#,
            texto_json(&temporal)
        );
        manda(n, &cmd);
        r.espera("hecho");
        unsafe { grim_parar(n) };

        assert!(!temporal.exists(), "el temporal tenía que moverse, no copiarse");
        // El único item.json de la biblioteca: está en items/<a>/<b>/<id>/.
        let mut fichas = Vec::new();
        for a in std::fs::read_dir(raiz.join("items")).unwrap().flatten() {
            for b in std::fs::read_dir(a.path()).unwrap().flatten() {
                for c in std::fs::read_dir(b.path()).unwrap().flatten() {
                    fichas.push(c.path().join("item.json"));
                }
            }
        }
        assert_eq!(fichas.len(), 1);
        let it: Item = serde_json::from_slice(&std::fs::read(&fichas[0]).unwrap()).unwrap();
        assert_eq!(it.source.as_deref(), Some("https://ejemplo.org/obra"));
    }

    #[test]
    fn importar_a_una_carpeta_que_no_existe_no_importa_nada() {
        let (_d, raiz, _) = biblioteca_con(0);
        let r = Recolector::nuevo();
        let ruta = CString::new(raiz.to_str().unwrap()).unwrap();
        let n = unsafe {
            grim_iniciar(
                ruta.as_ptr(),
                recoge,
                &r as *const Recolector as *mut c_void,
            )
        };
        r.espera("vista");
        r.olvida();
        manda(
            n,
            r#"{"cmd":"importar","rutas":["/tmp"],"carpeta":"inventada"}"#,
        );
        assert!(r.espera("error")["mensaje"]
            .as_str()
            .unwrap()
            .contains("no existe la carpeta"));
        unsafe { grim_parar(n) };
    }

    #[test]
    fn parar_con_comandos_en_la_cola_no_cuelga() {
        let (_d, raiz, _) = biblioteca_con(50);
        let r = Recolector::nuevo();
        let ruta = CString::new(raiz.to_str().unwrap()).unwrap();
        let n = unsafe {
            grim_iniciar(
                ruta.as_ptr(),
                recoge,
                &r as *const Recolector as *mut c_void,
            )
        };
        for _ in 0..50 {
            manda(n, r#"{"cmd":"consulta"}"#);
        }
        // `grim_parar` tiene que volver: si el hilo se quedara esperando algo,
        // cerrar la ventana dejaría el proceso colgado.
        unsafe { grim_parar(n) };
    }

    #[test]
    fn la_frontera_aguanta_punteros_nulos() {
        unsafe {
            assert_eq!(grim_mandar(std::ptr::null_mut(), b"x".as_ptr(), 1), 0);
            assert_eq!(grim_vista_n(std::ptr::null()), 0);
            assert_eq!(grim_vista_ancho(std::ptr::null(), 0), 0);
            assert_eq!(grim_vista_dominante(std::ptr::null(), 0), 0xFFFF_FFFF);
            let mut len = 9u32;
            assert!(grim_vista_thumb(std::ptr::null(), 0, &mut len).is_null());
            assert_eq!(len, 0);
            assert_eq!(
                grim_vista_indice_de(std::ptr::null(), b"x".as_ptr(), 1),
                usize::MAX
            );
            grim_vista_soltar(std::ptr::null());
            grim_parar(std::ptr::null_mut());
        }
    }

    /// Arranca el núcleo y espera a que la primera vista esté publicada.
    fn arranca(raiz: &std::path::Path, r: &Recolector) -> *mut Nucleo {
        let ruta = CString::new(raiz.to_str().unwrap()).unwrap();
        let n =
            unsafe { grim_iniciar(ruta.as_ptr(), recoge, r as *const Recolector as *mut c_void) };
        assert!(!n.is_null());
        r.espera("vista");
        n
    }

    /// Una ruta como cadena JSON, entre comillas y escapada. Metida a mano
    /// entre comillas, las barras de Windows (`C:\Users\…`) rompían el JSON.
    fn texto_json(ruta: &std::path::Path) -> String {
        serde_json::to_string(&ruta.to_string_lossy()).unwrap()
    }

    fn json(ids: &[String]) -> String {
        serde_json::to_string(ids).unwrap()
    }

    #[test]
    fn poner_estrellas_se_deshace_y_se_rehace() {
        let (_d, raiz, ids) = biblioteca_con(4);
        let r = Recolector::nuevo();
        let n = arranca(&raiz, &r);

        manda(
            n,
            &format!(r#"{{"cmd":"estrellas","ids":{},"valor":4}}"#, json(&ids)),
        );
        assert_eq!(r.espera("item")["estrellas"], 4);
        // El menú tiene que enterarse de que ahora hay algo que deshacer, y de
        // cómo se llama, sin tener que preguntarlo.
        assert_eq!(r.espera("registro")["deshacer"], "poner 4 estrellas");

        r.olvida();
        manda(n, r#"{"cmd":"deshacer"}"#);
        assert_eq!(r.espera("item")["estrellas"], 0);
        let reg = r.espera("registro");
        assert!(reg["deshacer"].is_null());
        assert_eq!(reg["rehacer"], "poner 4 estrellas");

        r.olvida();
        manda(n, r#"{"cmd":"rehacer"}"#);
        assert_eq!(r.espera("item")["estrellas"], 4);
        unsafe { grim_parar(n) };
    }

    #[test]
    fn reordenar_pone_uno_entre_sus_vecinos_y_se_deshace() {
        let (_d, raiz, _) = biblioteca_con(4);
        let orden = || -> Vec<String> {
            let lib = Library::open(&raiz).unwrap();
            let q = Query {
                sort: grimorio_core::SortBy::Manual,
                limit: 0,
                ..Default::default()
            };
            lib.index().search(&q).unwrap().into_iter().map(|h| h.id).collect()
        };
        let antes = orden();
        let r = Recolector::nuevo();
        let n = arranca(&raiz, &r);
        r.espera("vista");

        // El último, al principio.
        r.olvida();
        manda(
            n,
            &format!(r#"{{"cmd":"reordenar","id":"{}","despues":"{}"}}"#, antes[3], antes[0]),
        );
        r.espera("hecho");
        assert_eq!(orden(), vec![antes[3].clone(), antes[0].clone(), antes[1].clone(), antes[2].clone()]);

        // El primero (que ahora es antes[3]) entre el segundo y el tercero.
        r.olvida();
        manda(
            n,
            &format!(
                r#"{{"cmd":"reordenar","id":"{}","antes":"{}","despues":"{}"}}"#,
                antes[3], antes[0], antes[1]
            ),
        );
        r.espera("hecho");
        assert_eq!(orden(), vec![antes[0].clone(), antes[3].clone(), antes[1].clone(), antes[2].clone()]);

        r.olvida();
        manda(n, r#"{"cmd":"deshacer"}"#);
        r.espera("registro");
        r.olvida();
        manda(n, r#"{"cmd":"deshacer"}"#);
        r.espera("registro");
        assert_eq!(orden(), antes, "deshacer dos veces deja el orden de siempre");
        unsafe { grim_parar(n) };
    }

    #[test]
    fn muchos_movimientos_seguidos_con_empates_siguen_el_orden_esperado() {
        // Importados de una vez: todos con el mismo milisegundo, el peor caso
        // para el punto medio. Se mueven 60 veces a sitios distintos y el
        // orden del índice tiene que ser siempre el de la lista de aquí.
        let (_d, raiz, _) = biblioteca_con(40);
        let orden = || -> Vec<String> {
            let lib = Library::open(&raiz).unwrap();
            let q = Query {
                sort: grimorio_core::SortBy::Manual,
                limit: 0,
                ..Default::default()
            };
            lib.index().search(&q).unwrap().into_iter().map(|h| h.id).collect()
        };
        let mut esperado = orden();
        let r = Recolector::nuevo();
        let n = arranca(&raiz, &r);
        r.espera("vista");
        let mut semilla = 7u64;
        for _ in 0..60 {
            semilla = semilla.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let de = (semilla >> 33) as usize % esperado.len();
            semilla = semilla.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let id = esperado.remove(de);
            let a = (semilla >> 33) as usize % (esperado.len() + 1);
            let antes = if a > 0 { format!(r#","antes":"{}""#, esperado[a - 1]) } else { String::new() };
            let despues = if a < esperado.len() { format!(r#","despues":"{}""#, esperado[a]) } else { String::new() };
            esperado.insert(a, id.clone());
            r.olvida();
            manda(n, &format!(r#"{{"cmd":"reordenar","id":"{id}"{antes}{despues}}}"#));
            r.espera("hecho");
            assert_eq!(orden(), esperado);
        }
        unsafe { grim_parar(n) };
    }

    #[test]
    fn una_carpeta_vigilada_importa_lo_que_hay_y_luego_lo_nuevo() {
        let (dir, raiz, _) = biblioteca_con(0);
        let disco = dir.path().join("descargas");
        std::fs::create_dir_all(&disco).unwrap();
        let foto = |nombre: &str, tono: u8| {
            let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_fn(30, 20, |x, y| {
                image::Rgb([tono, x as u8 * 8, y as u8 * 9])
            }));
            let jpeg = grimorio_core::image_ops::encode_jpeg(&img.to_rgb8(), 80).unwrap();
            std::fs::write(disco.join(nombre), jpeg).unwrap();
        };
        foto("ya-estaba.jpg", 10);

        let r = Recolector::nuevo();
        let n = arranca(&raiz, &r);
        r.espera("vista");
        manda(
            n,
            &format!(r#"{{"cmd":"vigilar_carpeta","ruta":{}}}"#, texto_json(&disco)),
        );
        let cuantos = || {
            let mut c = 0;
            if let Ok(a) = std::fs::read_dir(raiz.join("items")) {
                for a in a.flatten() {
                    for b in std::fs::read_dir(a.path()).unwrap().flatten() {
                        c += std::fs::read_dir(b.path()).unwrap().count();
                    }
                }
            }
            c
        };
        let espera_a = |n: usize| {
            let hasta = Instant::now() + Duration::from_secs(20);
            while cuantos() < n {
                assert!(Instant::now() < hasta, "no llegó a {n}; hay {}", cuantos());
                std::thread::sleep(Duration::from_millis(100));
            }
        };
        espera_a(1);
        // El original se queda donde estaba: se copia, no se mueve.
        assert!(disco.join("ya-estaba.jpg").exists());

        foto("nueva.jpg", 200);
        espera_a(2);
        std::thread::sleep(Duration::from_secs(4));
        assert_eq!(cuantos(), 2, "lo ya importado no vuelve a entrar");
        unsafe { grim_parar(n) };
    }

    #[test]
    fn renombrar_una_etiqueta_la_cambia_en_todos_y_fusiona_sin_repetir() {
        let (_d, raiz, ids) = biblioteca_con(3);
        let r = Recolector::nuevo();
        let n = arranca(&raiz, &r);
        r.espera("carpetas");
        // 0 y 1 llevan «gato»; 1 y 2 llevan «felino».
        for (i, t) in [(0, "gato"), (1, "gato"), (1, "felino"), (2, "felino")] {
            r.olvida();
            manda(
                n,
                &format!(r#"{{"cmd":"etiquetar","ids":["{}"],"anadir":["{t}"]}}"#, ids[i]),
            );
            r.espera("hecho");
        }
        r.olvida();
        manda(n, r#"{"cmd":"renombrar_etiqueta","vieja":"gato","nueva":"felino"}"#);
        r.espera("hecho");
        let lib = Library::open(&raiz).unwrap();
        for id in &ids {
            assert_eq!(lib.load_item(id).unwrap().tags, vec!["felino".to_string()], "{id}");
        }
        drop(lib);

        r.olvida();
        manda(n, r#"{"cmd":"deshacer"}"#);
        r.espera("registro");
        let lib = Library::open(&raiz).unwrap();
        assert!(lib.load_item(&ids[0]).unwrap().tags.contains(&"gato".to_string()));
        drop(lib);

        r.olvida();
        manda(n, r#"{"cmd":"borrar_etiqueta","nombre":"felino"}"#);
        r.espera("hecho");
        let lib = Library::open(&raiz).unwrap();
        assert!(!lib.load_item(&ids[2]).unwrap().tags.contains(&"felino".to_string()));
        unsafe { grim_parar(n) };
    }

    #[test]
    fn renombrar_en_lote_numera_en_el_orden_dado_y_se_deshace() {
        let (_d, raiz, ids) = biblioteca_con(3);
        let r = Recolector::nuevo();
        let n = arranca(&raiz, &r);
        r.espera("carpetas");
        r.olvida();
        // Al revés a propósito: el orden lo pone quien llama (la vista).
        let orden = vec![ids[2].clone(), ids[0].clone(), ids[1].clone()];
        manda(
            n,
            &format!(
                r#"{{"cmd":"renombrar_en_lote","ids":{},"patron":"ref-{{n:2}}","inicio":5}}"#,
                json(&orden)
            ),
        );
        assert!(r.espera("hecho")["mensaje"].as_str().unwrap().starts_with("3 renombrados"));
        let lib = Library::open(&raiz).unwrap();
        assert_eq!(lib.load_item(&ids[2]).unwrap().name, "ref-05");
        assert_eq!(lib.load_item(&ids[0]).unwrap().name, "ref-06");
        assert_eq!(lib.load_item(&ids[1]).unwrap().name, "ref-07");
        let antes = lib.load_item(&ids[0]).unwrap().name;
        drop(lib);

        r.olvida();
        manda(n, r#"{"cmd":"deshacer"}"#);
        r.espera("registro");
        let lib = Library::open(&raiz).unwrap();
        assert_ne!(lib.load_item(&ids[0]).unwrap().name, antes);
        unsafe { grim_parar(n) };
    }

    #[test]
    fn quitar_copias_exactas_deja_la_primera_y_se_deshace() {
        let (dir, raiz, _) = biblioteca_con(0);
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_fn(40, 30, |x, y| {
            image::Rgb([x as u8 * 6, y as u8 * 8, 70])
        }));
        let jpeg = grimorio_core::image_ops::encode_jpeg(&img.to_rgb8(), 80).unwrap();
        let fuente = dir.path().join("copias");
        std::fs::create_dir_all(&fuente).unwrap();
        for n in 0..3 {
            std::fs::write(fuente.join(format!("copia{n}.jpg")), &jpeg).unwrap();
        }
        {
            let mut lib = Library::open(&raiz).unwrap();
            let opts = grimorio_core::import::ImportOptions {
                skip_duplicates: false,
                previews: false,
                ..Default::default()
            };
            let inf = grimorio_core::import::import(&mut lib, &[fuente], &opts, &|_, _| {}).unwrap();
            assert_eq!(inf.ids.len(), 3);
        }

        let r = Recolector::nuevo();
        let n = arranca(&raiz, &r);
        r.espera("carpetas");
        r.olvida();
        manda(n, r#"{"cmd":"quitar_copias_exactas"}"#);
        let hecho = r.espera("hecho");
        assert!(hecho["mensaje"].as_str().unwrap().starts_with("2 copias"), "{hecho}");
        assert_eq!(r.espera("carpetas")["papelera"], 2);

        r.olvida();
        manda(n, r#"{"cmd":"deshacer"}"#);
        assert_eq!(r.espera("carpetas")["papelera"], 0);
        unsafe { grim_parar(n) };
    }

    #[test]
    fn una_carpeta_inteligente_cuenta_lo_que_encaja_y_se_pone_al_dia() {
        let (_d, raiz, ids) = biblioteca_con(3);
        let r = Recolector::nuevo();
        let n = arranca(&raiz, &r);
        r.espera("carpetas");

        r.olvida();
        manda(
            n,
            r#"{"cmd":"guardar_busqueda","nombre":"buenas","consulta":"estrellas:>=4"}"#,
        );
        let lista = r.espera("busquedas")["lista"].clone();
        assert_eq!(lista[0]["nombre"], "buenas");
        assert_eq!(lista[0]["cuenta"], 0);

        // Poner estrellas cambia cuántas encajan, y se dice sin preguntar.
        r.olvida();
        manda(
            n,
            &format!(r#"{{"cmd":"estrellas","ids":{},"valor":4}}"#, json(&ids[..2])),
        );
        assert_eq!(r.espera("busquedas")["lista"][0]["cuenta"], 2);

        let id = lista[0]["id"].as_str().unwrap().to_string();
        r.olvida();
        manda(n, &format!(r#"{{"cmd":"borrar_busqueda","id":"{id}"}}"#));
        assert_eq!(r.espera("busquedas")["lista"].as_array().unwrap().len(), 0);
        unsafe { grim_parar(n) };
    }

    #[test]
    fn marcar_mas_18_publica_el_recuento_al_momento() {
        // El botón del modo seguro cuelga de este recuento: si no llega al
        // marcar, no aparece hasta reiniciar.
        let (_d, raiz, ids) = biblioteca_con(3);
        let r = Recolector::nuevo();
        let n = arranca(&raiz, &r);
        r.espera("carpetas");

        r.olvida();
        manda(
            n,
            &format!(r#"{{"cmd":"adulto","ids":{},"si":true}}"#, json(&ids[..2])),
        );
        assert_eq!(r.espera("item")["adulto"], true);
        assert_eq!(r.espera("carpetas")["adultos"], 2);

        // Y deshacerlo también lo cuenta.
        r.olvida();
        manda(n, r#"{"cmd":"deshacer"}"#);
        assert_eq!(r.espera("item")["adulto"], false);
        assert_eq!(r.espera("carpetas")["adultos"], 0);
        unsafe { grim_parar(n) };
    }

    #[test]
    fn deshacer_sin_nada_que_deshacer_contesta_en_vez_de_callarse() {
        let (_d, raiz, _) = biblioteca_con(2);
        let r = Recolector::nuevo();
        let n = arranca(&raiz, &r);
        r.olvida();
        manda(n, r#"{"cmd":"deshacer"}"#);
        let hecho = r.espera("hecho");
        assert!(hecho["mensaje"]
            .as_str()
            .unwrap()
            .contains("nada que deshacer"));
        unsafe { grim_parar(n) };
    }

    #[test]
    fn lo_que_va_a_la_papelera_sale_de_la_vista_y_vuelve_al_deshacer() {
        let (_d, raiz, ids) = biblioteca_con(5);
        let r = Recolector::nuevo();
        let n = arranca(&raiz, &r);

        r.olvida();
        manda(
            n,
            &format!(
                r#"{{"cmd":"papelera","ids":{},"dentro":true}}"#,
                json(&ids[..2])
            ),
        );
        assert_eq!(
            r.espera("vista")["n"],
            3,
            "la malla se queda con los que quedan"
        );

        r.olvida();
        manda(n, r#"{"cmd":"consulta","q":{"limit":0,"papelera":true}}"#);
        assert_eq!(
            r.espera("vista")["n"],
            2,
            "y la papelera enseña los otros dos"
        );

        r.olvida();
        manda(n, r#"{"cmd":"deshacer"}"#);
        assert_eq!(
            r.espera("vista")["n"],
            0,
            "que dejan de estar en la papelera"
        );
        unsafe { grim_parar(n) };
    }

    #[test]
    fn el_buscador_entiende_lo_que_se_le_escribe() {
        let (_d, raiz, ids) = biblioteca_con(6);
        let r = Recolector::nuevo();
        let n = arranca(&raiz, &r);

        manda(
            n,
            &format!(
                r#"{{"cmd":"etiquetar","ids":{},"anadir":["rótulo"]}}"#,
                json(&ids[..2])
            ),
        );
        r.espera("registro");

        r.olvida();
        manda(
            n,
            r#"{"cmd":"consulta","q":{"limit":0},"filtro":"etiqueta:rótulo"}"#,
        );
        assert_eq!(r.espera("vista")["n"], 2);

        r.olvida();
        manda(
            n,
            r#"{"cmd":"consulta","q":{"limit":0},"filtro":"estrellas:>=4"}"#,
        );
        assert_eq!(
            r.espera("vista")["n"],
            0,
            "nadie tiene cuatro estrellas todavía"
        );
        unsafe { grim_parar(n) };
    }

    #[test]
    fn las_etiquetas_llegan_solas_para_autocompletar() {
        let (_d, raiz, ids) = biblioteca_con(4);
        let r = Recolector::nuevo();
        let n = arranca(&raiz, &r);

        manda(
            n,
            &format!(
                r#"{{"cmd":"etiquetar","ids":{},"anadir":["tipografía"]}}"#,
                json(&ids)
            ),
        );
        // Sin pedirlas: etiquetar cambia la lista, así que la lista se manda.
        let lista = r.espera("etiquetas");
        assert_eq!(lista["lista"][0]["nombre"], "tipografía");
        assert_eq!(lista["lista"][0]["n"], 4);

        r.olvida();
        manda(n, r#"{"cmd":"etiquetas","prefijo":"tipo"}"#);
        assert_eq!(r.espera("etiquetas")["lista"][0]["nombre"], "tipografía");

        r.olvida();
        manda(n, r#"{"cmd":"etiquetas","prefijo":"zzz"}"#);
        assert_eq!(r.espera("etiquetas")["lista"].as_array().unwrap().len(), 0);
        unsafe { grim_parar(n) };
    }

    #[test]
    fn un_cambio_que_saca_al_elemento_del_filtro_rehace_la_vista() {
        let (_d, raiz, ids) = biblioteca_con(6);
        let r = Recolector::nuevo();
        let n = arranca(&raiz, &r);

        manda(
            n,
            &format!(
                r#"{{"cmd":"estrellas","ids":{},"valor":5}}"#,
                json(&ids[..3])
            ),
        );
        r.espera("registro");
        r.olvida();
        manda(
            n,
            r#"{"cmd":"consulta","q":{"limit":0},"filtro":"estrellas:>=4"}"#,
        );
        assert_eq!(r.espera("vista")["n"], 3);

        // Quitarle la estrella a uno lo saca del filtro. Avisar del elemento y
        // ya sería dejarlo en la malla incumpliéndolo.
        r.olvida();
        manda(
            n,
            &format!(
                r#"{{"cmd":"estrellas","ids":{},"valor":1}}"#,
                json(&ids[..1])
            ),
        );
        assert_eq!(r.espera("vista")["n"], 2);

        // Sin filtro de estrellas, en cambio, basta con repintar esa celda.
        r.olvida();
        manda(n, r#"{"cmd":"consulta","q":{"limit":0}}"#);
        r.espera("vista");
        r.olvida();
        manda(
            n,
            &format!(
                r#"{{"cmd":"estrellas","ids":{},"valor":2}}"#,
                json(&ids[..1])
            ),
        );
        assert_eq!(r.espera("item")["estrellas"], 2);
        assert!(
            !r.eventos
                .lock()
                .unwrap()
                .iter()
                .any(|v| v["tipo"] == "vista"),
            "consultar cien mil elementos por una estrella es lo que se está evitando"
        );
        unsafe { grim_parar(n) };
    }

    #[test]
    fn un_lote_grande_manda_progreso_en_vez_de_un_evento_por_elemento() {
        let (_d, raiz, ids) = biblioteca_con(200);
        let r = Recolector::nuevo();
        let n = arranca(&raiz, &r);

        r.olvida();
        manda(
            n,
            &format!(r#"{{"cmd":"estrellas","ids":{},"valor":3}}"#, json(&ids)),
        );
        r.espera("vista");
        let eventos = r.eventos.lock().unwrap();
        let items = eventos.iter().filter(|v| v["tipo"] == "item").count();
        let progresos = eventos.iter().filter(|v| v["tipo"] == "progreso").count();
        assert_eq!(items, 0, "doscientos eventos ahogarían el hilo de interfaz");
        assert!(progresos > 0, "pero algo tiene que decir mientras tanto");
        drop(eventos);
        unsafe { grim_parar(n) };
    }
}

/// Pide un derivado —proxy de vídeo, onda, página de PDF, malla 3D— y espera
/// a que esté. Bloquea, y puede tardar minutos: se llama desde un hilo propio,
/// nunca desde el de la interfaz ni el del bus.
///
/// Escribe la respuesta JSON en `salida` y devuelve cuántos bytes ocupa. Si no
/// cabe no escribe nada y devuelve lo que haría falta: quien llama vuelve a
/// pedir con un búfer de ese tamaño. Como el derivado ya está en disco, la
/// segunda vez es inmediata.
///
/// # Safety
/// `peticion` apunta a `len` bytes válidos; `salida` a `cap` bytes escribibles.
#[no_mangle]
pub unsafe extern "C" fn grim_derivar(
    peticion: *const u8,
    len: u32,
    salida: *mut u8,
    cap: u32,
) -> u32 {
    if peticion.is_null() {
        return 0;
    }
    let bytes = std::slice::from_raw_parts(peticion, len as usize);
    // Un pánico no puede cruzar la frontera C: se vuelve una respuesta más.
    let r = std::panic::catch_unwind(|| derivar::responder(bytes)).unwrap_or_else(
        |_| serde_json::json!({ "ok": false, "error": "el núcleo falló preparando esto" }),
    );
    let texto = r.to_string();
    let b = texto.as_bytes();
    if b.len() <= cap as usize && !salida.is_null() {
        std::ptr::copy_nonoverlapping(b.as_ptr(), salida, b.len());
    }
    b.len() as u32
}
