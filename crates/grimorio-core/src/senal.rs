//! La forma de un sonido: su onda en el tiempo y su espectro en frecuencia.
//!
//! Se calcula una vez por archivo y se guarda (ver [`crate::derivado`]): el
//! visor solo lee y pinta. La decodificación la hace ffmpeg a un archivo de
//! muestras `f32` sueltas, y aquí se recorre ese archivo **por trozos**: una
//! hora de audio a 44,1 kHz son seiscientos megas en `f32`, y no hace falta
//! tenerlos en memoria para sacar cuatro mil columnas de dibujo.
//!
//! La FFT es la de toda la vida —radix 2, en sitio—: cuarenta líneas que no
//! justifican una dependencia, como dice la política del proyecto.

use std::f32::consts::PI;
use std::io::{Read, Write};

/// Muestras por pareja de picos de la onda. Con 44,1 kHz son 5,8 ms: de sobra
/// para ver un golpe de caja aunque se acerque mucho la vista.
pub const HOP_PICOS: u32 = 256;
/// Tamaño de la ventana del espectro. 2048 muestras a 44,1 kHz separan las
/// frecuencias de 21,5 en 21,5 Hz: bastante para ver notas graves.
pub const VENTANA: usize = 2048;
/// Columnas del espectro, como mucho. Más que píxeles tiene una pantalla.
pub const COLUMNAS_MAX: usize = 4096;
/// Filas del espectro, en escala logarítmica de frecuencia.
pub const FILAS: usize = 256;
pub const F_MIN: f32 = 30.0;
/// Por debajo de esto el espectro se pinta negro: es ruido de fondo.
pub const DB_SUELO: f32 = -90.0;

/// Lo que el visor necesita para pintar un sonido.
#[derive(Debug, Clone, Default)]
pub struct Analisis {
    pub tasa: u32,
    pub muestras: u64,
    /// (mínimo, máximo) de cada tramo de [`HOP_PICOS`] muestras.
    pub picos: Vec<[f32; 2]>,
    /// Muestras entre una columna del espectro y la siguiente.
    pub hop_espectro: u32,
    pub columnas: u32,
    /// Por columnas: `FILAS` bytes cada una, de grave a agudo. 0 es el suelo
    /// de [`DB_SUELO`] y 255 el máximo posible.
    pub espectro: Vec<u8>,
    pub f_max: f32,
}

/// Transformada rápida de Fourier, compleja y en sitio. `re.len()` tiene que
/// ser potencia de dos.
pub fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut largo = 2;
    while largo <= n {
        let ang = -2.0 * PI / largo as f32;
        let (wr, wi) = (ang.cos(), ang.sin());
        for ini in (0..n).step_by(largo) {
            let (mut cr, mut ci) = (1.0f32, 0.0f32);
            for k in 0..largo / 2 {
                let (a, b) = (ini + k, ini + k + largo / 2);
                let tr = re[b] * cr - im[b] * ci;
                let ti = re[b] * ci + im[b] * cr;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
                let nr = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = nr;
            }
        }
        largo <<= 1;
    }
}

/// Qué cubetas de la FFT caen en cada fila logarítmica.
fn filas_de(tasa: u32) -> (Vec<(usize, usize)>, f32) {
    let nyquist = tasa as f32 / 2.0;
    let por_cubeta = tasa as f32 / VENTANA as f32;
    let (lo, hi) = (F_MIN.ln(), nyquist.ln());
    let filas = (0..FILAS)
        .map(|r| {
            let f0 = (lo + (hi - lo) * r as f32 / FILAS as f32).exp();
            let f1 = (lo + (hi - lo) * (r + 1) as f32 / FILAS as f32).exp();
            let a = (f0 / por_cubeta).floor() as usize;
            let b = ((f1 / por_cubeta).ceil() as usize).max(a + 1);
            (a.min(VENTANA / 2), b.min(VENTANA / 2 + 1))
        })
        .collect();
    (filas, nyquist)
}

/// Analiza muestras en memoria. Para archivos de verdad, [`analizar_archivo`].
pub fn analizar(muestras: &[f32], tasa: u32) -> Analisis {
    analizar_con(muestras.len() as u64, tasa, |desde, buf| {
        let d = desde as usize;
        for (k, x) in buf.iter_mut().enumerate() {
            *x = muestras.get(d + k).copied().unwrap_or(0.0);
        }
    })
}

/// Lee en una posición sin depender de dónde quedó el cursor. En Unix es
/// `pread`; Windows tiene su equivalente con otro nombre, que además mueve el
/// cursor, cosa que aquí da igual porque nadie más lee de este `File`.
#[cfg(unix)]
fn leer_en(f: &std::fs::File, buf: &mut [u8], pos: u64) -> std::io::Result<usize> {
    std::os::unix::fs::FileExt::read_at(f, buf, pos)
}

#[cfg(windows)]
fn leer_en(f: &std::fs::File, buf: &mut [u8], pos: u64) -> std::io::Result<usize> {
    std::os::windows::fs::FileExt::seek_read(f, buf, pos)
}

/// Analiza un archivo de `f32` little endian, mono, sin cabecera: lo que
/// escribe `ffmpeg -f f32le -ac 1`.
pub fn analizar_archivo(ruta: &std::path::Path, tasa: u32) -> std::io::Result<Analisis> {
    let f = std::fs::File::open(ruta)?;
    let n = f.metadata()?.len() / 4;
    let mut bytes = Vec::new();
    Ok(analizar_con(n, tasa, |desde, buf| {
        bytes.resize(buf.len() * 4, 0);
        let leidos = leer_en(&f, &mut bytes, desde * 4).unwrap_or(0);
        for (k, x) in buf.iter_mut().enumerate() {
            *x = if k * 4 + 4 <= leidos {
                f32::from_le_bytes([
                    bytes[k * 4],
                    bytes[k * 4 + 1],
                    bytes[k * 4 + 2],
                    bytes[k * 4 + 3],
                ])
            } else {
                0.0
            };
        }
    }))
}

/// El análisis, leyendo por trozos con `leer(desde, búfer)`.
fn analizar_con(n: u64, tasa: u32, mut leer: impl FnMut(u64, &mut [f32])) -> Analisis {
    // Los picos, de una pasada y por bloques grandes.
    let mut picos = Vec::with_capacity((n / HOP_PICOS as u64 + 1) as usize);
    let bloque = HOP_PICOS as usize * 1024;
    let mut buf = vec![0f32; bloque];
    let mut desde = 0u64;
    while desde < n {
        let cuantos = ((n - desde) as usize).min(bloque);
        leer(desde, &mut buf[..cuantos]);
        for tramo in buf[..cuantos].chunks(HOP_PICOS as usize) {
            let (mut lo, mut hi) = (0f32, 0f32);
            for &x in tramo {
                lo = lo.min(x);
                hi = hi.max(x);
            }
            picos.push([lo.max(-1.0), hi.min(1.0)]);
        }
        desde += cuantos as u64;
    }

    // El espectro: una ventana de Hann centrada en cada columna.
    let columnas = (n as usize / 256).clamp(1, COLUMNAS_MAX);
    let hop = ((n as usize).max(1) as f64 / columnas as f64).max(1.0);
    let (filas, f_max) = filas_de(tasa);
    let hann: Vec<f32> = (0..VENTANA)
        .map(|i| 0.5 - 0.5 * (2.0 * PI * i as f32 / (VENTANA - 1) as f32).cos())
        .collect();
    // Una sinusoide a escala completa con ventana de Hann da N/4 de amplitud:
    // eso es el 0 dB.
    let referencia = VENTANA as f32 / 4.0;
    let mut espectro = Vec::with_capacity(columnas * FILAS);
    let (mut re, mut im) = (vec![0f32; VENTANA], vec![0f32; VENTANA]);
    let mut ventana = vec![0f32; VENTANA];
    let mut mag = vec![0f32; VENTANA / 2 + 1];
    for c in 0..columnas {
        let centro = (c as f64 * hop + hop / 2.0) as i64;
        let ini = centro - VENTANA as i64 / 2;
        if ini < 0 {
            let falta = (-ini) as usize;
            ventana[..falta].fill(0.0);
            leer(0, &mut ventana[falta..]);
        } else {
            leer(ini as u64, &mut ventana);
        }
        for i in 0..VENTANA {
            re[i] = ventana[i] * hann[i];
            im[i] = 0.0;
        }
        fft(&mut re, &mut im);
        for (k, m) in mag.iter_mut().enumerate() {
            *m = (re[k] * re[k] + im[k] * im[k]).sqrt() / referencia;
        }
        for &(a, b) in &filas {
            let m = mag[a..b].iter().copied().fold(0.0f32, f32::max);
            let db = 20.0 * m.max(1e-9).log10();
            let v = ((db - DB_SUELO) / -DB_SUELO).clamp(0.0, 1.0);
            espectro.push((v * 255.0).round() as u8);
        }
    }

    Analisis {
        tasa,
        muestras: n,
        picos,
        hop_espectro: hop.round() as u32,
        columnas: columnas as u32,
        espectro,
        f_max,
    }
}

/// La onda dibujada, para la miniatura de un sonido que no trae carátula.
///
/// Una celda gris que dice «audio» no se distingue de la de al lado; la forma
/// del sonido sí: un golpe seco, una voz, una canción entera se ven distintos.
/// Barras simétricas en tono claro sobre fondo oscuro, como la miniatura de un
/// modelo: es contenido, no interfaz, y tiene que leerse en cualquier tema.
pub fn dibujar_onda(a: &Analisis, ancho: u32, alto: u32) -> image::RgbImage {
    let mut img = image::RgbImage::from_pixel(ancho, alto, image::Rgb([30, 31, 34]));
    let n = a.picos.len();
    if n == 0 {
        return img;
    }
    let margen = alto as f32 * 0.12;
    let medio = alto as f32 / 2.0;
    let alcance = medio - margen;
    // Normalizada al pico más alto: un archivo grabado flojo también se ve.
    let tope = a
        .picos
        .iter()
        .map(|p| p[0].abs().max(p[1].abs()))
        .fold(0.0f32, f32::max)
        .max(1e-4);
    let barra = 3u32;
    let paso = barra + 2;
    let columnas = (ancho / paso).max(1);
    for c in 0..columnas {
        let i0 = c as usize * n / columnas as usize;
        let i1 = ((c as usize + 1) * n / columnas as usize)
            .max(i0 + 1)
            .min(n);
        let v = a.picos[i0..i1]
            .iter()
            .map(|p| p[0].abs().max(p[1].abs()))
            .fold(0.0f32, f32::max)
            / tope;
        let h = (v * alcance).max(1.0);
        let y0 = (medio - h).max(0.0) as u32;
        let y1 = ((medio + h) as u32).min(alto - 1);
        let x0 = c * paso + (ancho - columnas * paso) / 2;
        for x in x0..(x0 + barra).min(ancho) {
            for y in y0..=y1 {
                // Más claro en el centro que en las puntas: da volumen sin
                // necesitar un segundo color.
                let d = ((y as f32 - medio).abs() / alcance.max(1.0)).min(1.0);
                let k = 1.0 - 0.35 * d;
                img.put_pixel(
                    x,
                    y,
                    image::Rgb([(224.0 * k) as u8, (218.0 * k) as u8, (208.0 * k) as u8]),
                );
            }
        }
    }
    img
}

impl Analisis {
    /// Formato del archivo `.onda` (little endian):
    /// ```text
    /// "GRIMONDA" · u32 versión (1) · u32 tasa · u64 muestras
    /// u32 hop_picos · u32 n_picos · u32 hop_espectro · u32 columnas · u32 filas
    /// f32 f_min · f32 f_max
    /// f32 picos[n_picos·2]  (mínimo, máximo)
    /// u8 espectro[columnas·filas]  (por columnas, de grave a agudo)
    /// ```
    pub fn escribir(&self, w: &mut impl Write) -> std::io::Result<()> {
        let mut b = Vec::with_capacity(56 + self.picos.len() * 8 + self.espectro.len());
        b.extend_from_slice(b"GRIMONDA");
        for x in [1u32, self.tasa] {
            b.extend_from_slice(&x.to_le_bytes());
        }
        b.extend_from_slice(&self.muestras.to_le_bytes());
        for x in [
            HOP_PICOS,
            self.picos.len() as u32,
            self.hop_espectro,
            self.columnas,
            FILAS as u32,
        ] {
            b.extend_from_slice(&x.to_le_bytes());
        }
        b.extend_from_slice(&F_MIN.to_le_bytes());
        b.extend_from_slice(&self.f_max.to_le_bytes());
        for p in &self.picos {
            b.extend_from_slice(&p[0].to_le_bytes());
            b.extend_from_slice(&p[1].to_le_bytes());
        }
        b.extend_from_slice(&self.espectro);
        w.write_all(&b)
    }

    /// Lo contrario de [`escribir`], para las pruebas y para el CLI.
    pub fn leer(r: &mut impl Read) -> std::io::Result<Analisis> {
        let mut b = Vec::new();
        r.read_to_end(&mut b)?;
        let mal = || std::io::Error::new(std::io::ErrorKind::InvalidData, "no es un .onda");
        if b.len() < 52 || &b[..8] != b"GRIMONDA" {
            return Err(mal());
        }
        let u = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
        let f = |o: usize| f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
        let tasa = u(12);
        let muestras = u64::from_le_bytes(b[16..24].try_into().unwrap());
        let n_picos = u(28) as usize;
        let hop_espectro = u(32);
        let columnas = u(36);
        let filas = u(40) as usize;
        let f_max = f(48);
        let o = 52;
        let fin_picos = o + n_picos * 8;
        if b.len() < fin_picos + columnas as usize * filas {
            return Err(mal());
        }
        let picos = (0..n_picos)
            .map(|k| [f(o + k * 8), f(o + k * 8 + 4)])
            .collect();
        Ok(Analisis {
            tasa,
            muestras,
            picos,
            hop_espectro,
            columnas,
            espectro: b[fin_picos..fin_picos + columnas as usize * filas].to_vec(),
            f_max,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seno(f: f32, amp: f32, tasa: u32, segundos: f32) -> Vec<f32> {
        (0..(tasa as f32 * segundos) as usize)
            .map(|i| amp * (2.0 * PI * f * i as f32 / tasa as f32).sin())
            .collect()
    }

    #[test]
    fn la_fft_encuentra_la_frecuencia_de_un_seno() {
        let n = 64;
        let mut re: Vec<f32> = (0..n)
            .map(|i| (2.0 * PI * 5.0 * i as f32 / n as f32).cos())
            .collect();
        let mut im = vec![0.0; n];
        fft(&mut re, &mut im);
        let mag: Vec<f32> = (0..n / 2).map(|k| re[k].hypot(im[k])).collect();
        let pico = mag
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap()
            .0;
        assert_eq!(pico, 5);
    }

    #[test]
    fn los_picos_de_la_onda_siguen_la_amplitud() {
        let a = analizar(&seno(440.0, 0.5, 44_100, 1.0), 44_100);
        assert_eq!(a.picos.len(), (44_100 + 255) / 256);
        let max = a.picos.iter().map(|p| p[1]).fold(0.0f32, f32::max);
        let min = a.picos.iter().map(|p| p[0]).fold(0.0f32, f32::min);
        assert!(
            (max - 0.5).abs() < 0.01 && (min + 0.5).abs() < 0.01,
            "{min} {max}"
        );
    }

    #[test]
    fn el_espectro_pone_un_tono_de_1_khz_en_su_fila() {
        let tasa = 44_100;
        let a = analizar(&seno(1000.0, 0.8, tasa, 2.0), tasa);
        let (filas, _) = filas_de(tasa);
        let c = a.columnas as usize / 2;
        let col = &a.espectro[c * FILAS..(c + 1) * FILAS];
        let fila = col.iter().enumerate().max_by_key(|(_, v)| **v).unwrap().0;
        let por_cubeta = tasa as f32 / VENTANA as f32;
        let (b0, b1) = filas[fila];
        let (f0, f1) = (b0 as f32 * por_cubeta, b1 as f32 * por_cubeta);
        assert!(
            f0 <= 1000.0 + por_cubeta && f1 >= 1000.0 - por_cubeta,
            "fila {fila}: {f0}..{f1}"
        );
        assert!(
            col[fila] > 200,
            "a -2 dB tiene que salir casi al máximo: {}",
            col[fila]
        );
    }

    #[test]
    fn la_miniatura_de_la_onda_sigue_al_sonido() {
        // Medio segundo de silencio y medio de tono: la mitad derecha tiene
        // que tener más luz que la izquierda.
        let mut s = vec![0.0f32; 4_000];
        s.extend(seno(440.0, 0.8, 8_000, 0.5));
        let img = dibujar_onda(&analizar(&s, 8_000), 200, 100);
        let luz = |x0: u32, x1: u32| -> u64 {
            (x0..x1)
                .flat_map(|x| (0..100).map(move |y| (x, y)))
                .map(|(x, y)| img.get_pixel(x, y).0[0] as u64)
                .sum()
        };
        assert!(luz(110, 190) > luz(10, 90) * 3);
    }

    #[test]
    fn el_silencio_es_el_suelo() {
        let a = analizar(&vec![0.0; 10_000], 44_100);
        assert!(a.espectro.iter().all(|v| *v == 0));
    }

    #[test]
    fn ida_y_vuelta_por_el_archivo() {
        let a = analizar(&seno(220.0, 0.3, 22_050, 0.5), 22_050);
        let mut b = Vec::new();
        a.escribir(&mut b).unwrap();
        let l = Analisis::leer(&mut b.as_slice()).unwrap();
        assert_eq!(l.picos, a.picos);
        assert_eq!(l.espectro, a.espectro);
        assert_eq!(
            (l.tasa, l.muestras, l.columnas),
            (a.tasa, a.muestras, a.columnas)
        );
    }

    #[test]
    fn leer_de_disco_da_lo_mismo_que_en_memoria() {
        let s = seno(330.0, 0.4, 8_000, 0.7);
        let tmp = crate::externo::Temporal::nuevo("pcm").unwrap();
        let mut b = Vec::new();
        for x in &s {
            b.extend_from_slice(&x.to_le_bytes());
        }
        std::fs::write(&tmp.ruta, b).unwrap();
        let a = analizar(&s, 8_000);
        let d = analizar_archivo(&tmp.ruta, 8_000).unwrap();
        assert_eq!(a.picos, d.picos);
        assert_eq!(a.espectro, d.espectro);
    }
}
