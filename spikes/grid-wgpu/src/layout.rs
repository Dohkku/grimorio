//! Disposición de la malla, virtualizada.
//!
//! La clave: nunca se recorre la biblioteca entera para pintar. Las filas se
//! calculan una vez (una pasada por las proporciones) y a partir de ahí saber
//! qué se ve es una búsqueda binaria sobre las alturas acumuladas.

use grimorio_core::QueryHit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modo {
    /// Celdas cuadradas iguales, la imagen se recorta para llenar.
    Cuadricula,
    /// Filas justificadas al ancho, cada imagen conserva su proporción.
    Justificado,
}

impl Modo {
    pub fn alternar(self) -> Modo {
        match self {
            Modo::Cuadricula => Modo::Justificado,
            Modo::Justificado => Modo::Cuadricula,
        }
    }
    pub fn nombre(self) -> &'static str {
        match self {
            Modo::Cuadricula => "cuadrícula",
            Modo::Justificado => "justificado",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Celda {
    pub indice: u32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Una fila: dónde empieza en la lista, cuántos elementos tiene y su altura.
#[derive(Debug, Clone, Copy)]
struct Fila {
    inicio: u32,
    n: u32,
    y: f32,
    alto: f32,
}

pub struct Disposicion {
    modo: Modo,
    ancho: f32,
    margen: f32,
    hueco: f32,
    /// Lado objetivo de la celda; el usuario lo cambia con la rueda o +/-.
    objetivo: f32,
    filas: Vec<Fila>,
    alto_total: f32,
    columnas: u32,
}

impl Disposicion {
    pub fn nueva(modo: Modo, objetivo: f32, margen: f32, hueco: f32) -> Self {
        Disposicion {
            modo,
            ancho: 0.0,
            margen,
            hueco,
            objetivo,
            filas: Vec::new(),
            alto_total: 0.0,
            columnas: 1,
        }
    }

    pub fn modo(&self) -> Modo {
        self.modo
    }
    pub fn objetivo(&self) -> f32 {
        self.objetivo
    }
    pub fn alto_total(&self) -> f32 {
        self.alto_total
    }
    pub fn filas(&self) -> usize {
        self.filas.len()
    }

    pub fn set_modo(&mut self, modo: Modo, hits: &[QueryHit]) {
        self.modo = modo;
        self.recalcular(hits);
    }

    pub fn set_objetivo(&mut self, objetivo: f32, hits: &[QueryHit]) {
        self.objetivo = objetivo.clamp(80.0, 420.0);
        self.recalcular(hits);
    }

    pub fn set_ancho(&mut self, ancho: f32, hits: &[QueryHit]) {
        if (ancho - self.ancho).abs() > 0.5 {
            self.ancho = ancho;
            self.recalcular(hits);
        }
    }

    /// Una pasada por la lista. Con 100.000 elementos son unos pocos
    /// milisegundos y solo ocurre al cambiar tamaño, modo o zoom.
    pub fn recalcular(&mut self, hits: &[QueryHit]) {
        self.filas.clear();
        let util = (self.ancho - self.margen * 2.0).max(50.0);
        let mut y = self.margen;

        match self.modo {
            Modo::Cuadricula => {
                let cols = ((util + self.hueco) / (self.objetivo + self.hueco))
                    .floor()
                    .max(1.0) as u32;
                self.columnas = cols;
                let lado = (util - self.hueco * (cols - 1) as f32) / cols as f32;
                let n = hits.len() as u32;
                let mut i = 0u32;
                while i < n {
                    let quedan = (n - i).min(cols);
                    self.filas.push(Fila {
                        inicio: i,
                        n: quedan,
                        y,
                        alto: lado,
                    });
                    y += lado + self.hueco;
                    i += cols;
                }
            }
            Modo::Justificado => {
                self.columnas = 0;
                let mut inicio = 0u32;
                let mut suma_prop = 0.0f32;
                for (i, h) in hits.iter().enumerate() {
                    let prop = h.aspect().clamp(0.35, 3.2);
                    suma_prop += prop;
                    let n = i as u32 - inicio + 1;
                    let huecos = self.hueco * (n.saturating_sub(1)) as f32;
                    let alto = (util - huecos) / suma_prop;
                    // Cerramos la fila cuando la altura resultante baja del
                    // objetivo: así todas las filas quedan parecidas de alto.
                    if alto <= self.objetivo || n as f32 * self.objetivo > util * 1.6 {
                        self.filas.push(Fila { inicio, n, y, alto });
                        y += alto + self.hueco;
                        inicio = i as u32 + 1;
                        suma_prop = 0.0;
                    }
                }
                // Última fila incompleta: se deja a la altura objetivo en vez de
                // estirarla, que es lo que hace que las galerías se vean rotas.
                if inicio < hits.len() as u32 {
                    let n = hits.len() as u32 - inicio;
                    self.filas.push(Fila {
                        inicio,
                        n,
                        y,
                        alto: self.objetivo,
                    });
                    y += self.objetivo + self.hueco;
                }
            }
        }
        self.alto_total = (y - self.hueco + self.margen).max(0.0);
    }

    /// Celdas visibles para un desplazamiento y alto de ventana dados.
    /// `margen_extra` pide filas de más arriba y abajo para precargar.
    pub fn visibles(
        &self,
        hits: &[QueryHit],
        scroll: f32,
        alto_ventana: f32,
        margen_extra: f32,
        salida: &mut Vec<Celda>,
    ) {
        salida.clear();
        if self.filas.is_empty() {
            return;
        }
        let y0 = scroll - margen_extra;
        let y1 = scroll + alto_ventana + margen_extra;

        // Búsqueda binaria de la primera fila visible: el coste no depende del
        // tamaño de la biblioteca.
        let mut lo = 0usize;
        let mut hi = self.filas.len();
        while lo < hi {
            let mid = (lo + hi) / 2;
            if self.filas[mid].y + self.filas[mid].alto < y0 {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }

        for fila in &self.filas[lo..] {
            if fila.y > y1 {
                break;
            }
            let mut x = self.margen;
            for k in 0..fila.n {
                let idx = fila.inicio + k;
                let hit = &hits[idx as usize];
                let w = match self.modo {
                    Modo::Cuadricula => fila.alto,
                    Modo::Justificado => (fila.alto * hit.aspect().clamp(0.35, 3.2)).max(24.0),
                };
                salida.push(Celda {
                    indice: idx,
                    x,
                    y: fila.y - scroll,
                    w,
                    h: fila.alto,
                });
                x += w + self.hueco;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hits(n: usize) -> Vec<QueryHit> {
        (0..n)
            .map(|i| QueryHit {
                id: format!("{i:0>26}"),
                name: format!("n{i}"),
                ext: "jpg".into(),
                size: 1000,
                width: if i % 3 == 0 { 1600 } else { 900 },
                height: if i % 3 == 0 { 900 } else { 1600 },
                stars: 0,
                thumb_off: None,
                thumb_len: None,
                dominant: None,
                imported_at_ms: 0,
                kind: Default::default(),
                duracion_s: None,
                ruta_ref: None,
                adulto: false,
            })
            .collect()
    }

    #[test]
    fn la_cuadricula_llena_el_ancho() {
        let h = hits(100);
        let mut d = Disposicion::nueva(Modo::Cuadricula, 200.0, 20.0, 10.0);
        d.set_ancho(1000.0, &h);
        let mut v = Vec::new();
        d.visibles(&h, 0.0, 800.0, 0.0, &mut v);
        assert!(!v.is_empty());
        let ultima = v.iter().take(d.columnas as usize).last().unwrap();
        assert!(ultima.x + ultima.w <= 1000.0 - 20.0 + 0.5);
    }

    #[test]
    fn justificado_respeta_proporciones() {
        let h = hits(200);
        let mut d = Disposicion::nueva(Modo::Justificado, 220.0, 20.0, 10.0);
        d.set_ancho(1400.0, &h);
        let mut v = Vec::new();
        d.visibles(&h, 0.0, 900.0, 0.0, &mut v);
        for c in &v {
            let esperado = h[c.indice as usize].aspect().clamp(0.35, 3.2);
            assert!((c.w / c.h - esperado).abs() < 0.02, "celda {:?}", c);
        }
    }

    #[test]
    fn solo_devuelve_lo_que_se_ve() {
        let h = hits(100_000);
        let mut d = Disposicion::nueva(Modo::Cuadricula, 160.0, 20.0, 10.0);
        d.set_ancho(1600.0, &h);
        let mut v = Vec::new();
        d.visibles(&h, 500_000.0, 1000.0, 200.0, &mut v);
        assert!(v.len() < 200, "devolvió {} celdas", v.len());
        assert!(v.iter().all(|c| c.y > -400.0 && c.y < 1400.0));
    }

    #[test]
    fn el_scroll_al_final_no_se_sale() {
        let h = hits(1000);
        let mut d = Disposicion::nueva(Modo::Justificado, 200.0, 20.0, 10.0);
        d.set_ancho(1200.0, &h);
        let mut v = Vec::new();
        d.visibles(&h, d.alto_total() - 100.0, 800.0, 0.0, &mut v);
        assert!(v.iter().any(|c| c.indice as usize == h.len() - 1));
    }
}
