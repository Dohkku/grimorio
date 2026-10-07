//! De qué está hecho un archivo, y cómo sacarle una imagen.
//!
//! Hasta M2 todo lo que entraba era una imagen que el crate `image` sabía
//! decodificar. Una biblioteca de referencias real tiene también vídeo, PDF,
//! tipografías, RAW y audio, y cada uno se abre de una forma distinta.
//!
//! Esta capa existe para que el resto del núcleo no se entere. La importación
//! pide un [`Lienzo`] y recibe siempre lo mismo: unas medidas, a veces una
//! imagen de la que sacar miniatura, paleta y huella, y a veces un dato extra
//! —la duración de un vídeo, las páginas de un PDF—.
//!
//! Regla de la casa: **un archivo que no sabemos dibujar entra igual**. Se
//! queda con su color de marcador y sus datos, y el día que sepamos dibujarlo
//! se le genera la miniatura sin volver a importar nada. Perder un archivo
//! porque todavía no tenemos el decodificador sería el peor de los dos males.

use crate::error::Result;
use crate::externo::{self, Temporal};
use crate::image_ops;
use image::DynamicImage;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

/// La familia de un archivo. Es lo que decide cómo se abre y cómo se enseña.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Familia {
    #[default]
    Imagen,
    /// Negativo digital. Lleva dentro un JPEG de vista previa que sí sabemos leer.
    Raw,
    Video,
    Audio,
    /// PDF por ahora; algún día PSD y compañía.
    Documento,
    Tipografia,
    /// Modelos 3D: piezas de impresión, escenas de Blender, mallas de juego.
    Modelo,
}

impl Familia {
    pub fn as_str(&self) -> &'static str {
        match self {
            Familia::Imagen => "imagen",
            Familia::Raw => "raw",
            Familia::Video => "video",
            Familia::Audio => "audio",
            Familia::Documento => "documento",
            Familia::Tipografia => "tipografia",
            Familia::Modelo => "modelo",
        }
    }

    /// Acepta también los nombres en inglés y el singular/plural que sale solo
    /// al escribir en el buscador: `tipo:videos` no puede ser un error.
    pub fn parse(s: &str) -> Option<Familia> {
        let s = s.trim().to_ascii_lowercase();
        let s = s.strip_suffix('s').unwrap_or(&s);
        match s {
            "imagen" | "image" | "foto" | "img" => Some(Familia::Imagen),
            "raw" | "negativo" => Some(Familia::Raw),
            "video" | "vídeo" | "peli" | "clip" => Some(Familia::Video),
            "audio" | "sonido" | "música" | "musica" => Some(Familia::Audio),
            "documento" | "document" | "doc" | "pdf" => Some(Familia::Documento),
            "tipografia" | "tipografía" | "fuente" | "font" => Some(Familia::Tipografia),
            "modelo" | "model" | "3d" | "malla" | "pieza" => Some(Familia::Modelo),
            _ => None,
        }
    }
}

/// Extensiones por familia, todas en minúsculas y sin punto.
///
/// Es una tabla y no una consulta a `file(1)`: mirar el contenido de cada
/// archivo antes de decidir cuesta una lectura extra por archivo, y con
/// cincuenta mil archivos eso se nota más que acertar en el caso raro de una
/// extensión mentirosa. Donde sí se mira dentro es al abrirlo.
const TABLA: &[(Familia, &[&str])] = &[
    (
        Familia::Imagen,
        &[
            "jpg", "jpeg", "png", "gif", "webp", "bmp", "tif", "tiff", "ico", "tga", "avif", "svg",
            // Los de diseño entran como imágenes: lo que se mira de ellos es
            // cómo quedan. La imagen la saca diseno.rs con su herramienta.
            "psd", "psb", "xcf", "kra", "ora", "sketch", "ai", "eps",
        ],
    ),
    (
        Familia::Raw,
        &[
            "cr2", "cr3", "nef", "arw", "dng", "orf", "rw2", "raf", "srw", "pef", "3fr", "erf",
        ],
    ),
    (
        Familia::Video,
        &[
            "mp4", "mov", "mkv", "webm", "avi", "m4v", "mpg", "mpeg", "wmv", "flv", "ogv",
        ],
    ),
    (
        Familia::Audio,
        &[
            "mp3", "flac", "wav", "ogg", "oga", "m4a", "aac", "opus", "aiff", "wma",
        ],
    ),
    (Familia::Documento, &["pdf"]),
    (Familia::Tipografia, &["ttf", "otf", "ttc", "woff", "woff2"]),
    (
        Familia::Modelo,
        &["stl", "obj", "ply", "glb", "gltf", "3mf", "blend", "fbx"],
    ),
];

/// La familia de una extensión, o `None` si no es nada que sepamos guardar.
///
/// Devolver `None` y no una familia «otros» es deliberado: arrastrar una
/// carpeta entera no puede meter en la biblioteca los `.o`, los `.log` y los
/// `.DS_Store` que hubiera dentro.
pub fn familia_de(ext: &str) -> Option<Familia> {
    let e = ext.trim_start_matches('.').to_ascii_lowercase();
    TABLA
        .iter()
        .find(|(_, exts)| exts.contains(&e.as_str()))
        .map(|(f, _)| *f)
}

pub fn se_puede_guardar(ext: &str) -> bool {
    familia_de(ext).is_some()
}

/// Lo que la importación necesita de un archivo, venga de donde venga.
#[derive(Debug, Default)]
pub struct Lienzo {
    /// De aquí salen la miniatura, la paleta y la huella perceptual.
    ///
    /// `None` quiere decir «todavía no sabemos dibujar esto». El elemento entra
    /// igual y se queda con su color de marcador.
    pub imagen: Option<DynamicImage>,
    /// Medidas del original, que no siempre son las de la imagen: la vista
    /// previa de un RAW es más pequeña que el negativo.
    pub ancho: u32,
    pub alto: u32,
    pub duracion_ms: Option<u64>,
    pub paginas: Option<u32>,
    /// Triángulos, si es un modelo 3D.
    pub triangulos: Option<u64>,
    /// Ancho, fondo y alto en milímetros, si el modelo dice sus unidades.
    pub medidas_mm: Option<[f32; 3]>,
    /// Por qué no hay imagen, para poder decirlo en el informe.
    pub sin_imagen: Option<String>,
}

impl Lienzo {
    fn de_imagen(img: DynamicImage) -> Lienzo {
        use image::GenericImageView;
        let (ancho, alto) = img.dimensions();
        Lienzo {
            imagen: Some(img),
            ancho,
            alto,
            ..Default::default()
        }
    }

    fn a_ciegas(motivo: impl Into<String>) -> Lienzo {
        Lienzo {
            sin_imagen: Some(motivo.into()),
            ..Default::default()
        }
    }
}

/// Abre un archivo ya leído en memoria y devuelve con qué trabajar.
///
/// Nunca falla por no saber dibujar algo: eso devuelve un lienzo a ciegas con
/// el motivo dentro. Solo falla si el archivo dice ser una imagen y no lo es,
/// porque entonces está roto y merece salir en el informe.
/// Cuánto se le deja a cada herramienta antes de darla por colgada.
///
/// Generosos pero finitos: un vídeo de dos horas en un disco lento tarda; un
/// vídeo roto no termina nunca. La diferencia entre los dos es este número.
const TOPE_VIDEO: Duration = Duration::from_secs(30);
const TOPE_PDF: Duration = Duration::from_secs(20);
const TOPE_INFORME: Duration = Duration::from_secs(10);

/// Cuánto de un archivo necesita [`abrir`] en memoria.
///
/// Existe por los archivos grandes. La importación leía cada archivo entero
/// antes de mirar qué era, y en paralelo: ocho hilos con ocho vídeos de cuatro
/// gigas eran treinta y dos gigas de memoria para sacar, al final, un
/// fotograma que `ffmpeg` lee del disco por su cuenta. Así la importación
/// solo carga lo que de verdad se decodifica aquí dentro.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lectura {
    /// Nada: lo que se dibuja sale de la ruta (vídeo, PDF, tipografía,
    /// formatos de diseño, `.blend`) o no se sabe dibujar todavía.
    Nada,
    /// Solo el principio, hasta tantos bytes. Es el audio: la carátula va en
    /// la etiqueta del principio (ID3v2, bloques de FLAC), y un WAV de una
    /// hora no tiene por qué entrar entero en memoria para encontrarla.
    Principio(usize),
    /// El archivo entero: imágenes, modelos que se leen aquí y RAW.
    Entero,
}

/// Hasta dónde se lee un audio buscando su carátula. Las etiquetas con
/// portada rondan los cientos de kilobytes; treinta y dos megas cubren hasta
/// las portadas a resolución de impresión sin cargar el sonido.
const PRINCIPIO_AUDIO: usize = 32 << 20;

/// Qué parte del archivo necesita [`abrir`] para una extensión.
pub fn que_leer(ext: &str) -> Lectura {
    let e = ext.trim_start_matches('.').to_ascii_lowercase();
    match familia_de(&e) {
        Some(Familia::Imagen) if crate::diseno::es_diseno(&e) => Lectura::Nada,
        Some(Familia::Imagen) if !image_ops::is_supported(&e) => Lectura::Nada,
        Some(Familia::Imagen) | Some(Familia::Raw) => Lectura::Entero,
        Some(Familia::Modelo) if crate::modelo3d::PROPIAS.contains(&e.as_str()) => {
            Lectura::Entero
        }
        Some(Familia::Audio) => Lectura::Principio(PRINCIPIO_AUDIO),
        _ => Lectura::Nada,
    }
}

/// El archivo hace falta en disco, y no solo sus bytes, porque las
/// herramientas de fuera trabajan con rutas. Los bytes se pasan igual para lo
/// que se decodifica aquí dentro y no tener que volver a leer el archivo.
///
/// Basta con pasar lo que pide [`que_leer`]; con más no cambia nada.
pub fn abrir(ruta: &Path, ext: &str, bytes: &[u8]) -> Result<Lienzo> {
    match familia_de(ext) {
        Some(Familia::Imagen) if crate::diseno::es_diseno(ext) => {
            Ok(match crate::diseno::previa(ruta, ext) {
                Ok(img) => Lienzo::de_imagen(img),
                Err(e) => Lienzo::a_ciegas(e),
            })
        }
        // La muestra de una tipografía no tiene medidas que contar: «1024 ×
        // 640» en el panel no diría nada de la letra. Se quedan a cero.
        Some(Familia::Tipografia) => Ok(match crate::diseno::muestra_tipografia(ruta) {
            Ok(img) => Lienzo {
                imagen: Some(img),
                ..Default::default()
            },
            Err(e) => Lienzo::a_ciegas(e),
        }),
        Some(Familia::Imagen) => {
            if !image_ops::is_supported(ext) {
                // Un formato de la familia que este núcleo todavía no decodifica
                // —SVG, AVIF— entra igual.
                return Ok(Lienzo::a_ciegas(format!("todavía no sé dibujar .{ext}")));
            }
            Ok(Lienzo::de_imagen(image_ops::decode(bytes)?))
        }
        Some(Familia::Video) => Ok(de_video(ruta)),
        Some(Familia::Audio) => Ok(de_audio(ruta, bytes)),
        Some(Familia::Documento) => Ok(de_pdf(ruta)),
        Some(Familia::Modelo) => Ok(de_modelo(ruta, ext, bytes)),
        Some(Familia::Raw) => match crate::raw::previa(bytes) {
            // Las medidas son las de la vista previa, no las del negativo: son
            // las únicas que conocemos sin revelarlo, y decir un número
            // inventado sería peor que decir el que se puede comprobar.
            Some(jpeg) => Ok(Lienzo::de_imagen(image_ops::decode(jpeg)?)),
            None => Ok(Lienzo::a_ciegas(
                "el negativo no lleva vista previa dentro".to_string(),
            )),
        },
        None => Ok(Lienzo::a_ciegas(format!("extensión desconocida: .{ext}"))),
    }
}

/// Lado de la miniatura de un modelo. Es la previsualización de 1024 la que
/// sale de aquí, así que se dibuja a ese tamaño.
const LADO_MODELO: u32 = crate::THUMB_PREVIEW;

/// Un modelo 3D dibujado desde arriba a la derecha, con sus medidas.
///
/// Los que este núcleo sabe leer se dibujan aquí mismo. `.blend` y `.fbx` se
/// convierten con Blender y se dibujan igual que los demás, para que una pieza
/// se vea igual venga del formato que venga. Sin Blender, entra a ciegas.
fn de_modelo(ruta: &Path, ext: &str, bytes: &[u8]) -> Lienzo {
    let ext = ext.to_ascii_lowercase();
    let leida = if crate::modelo3d::PROPIAS.contains(&ext.as_str()) {
        crate::modelo3d::leer(bytes, &ext, Some(ruta))
    } else {
        crate::derivado::leer_modelo(ruta, &ext)
    };
    // La miniatura que el `.blend` lleva dentro es de reserva, no la primera
    // opción: Blender la guarda a 128 px y con la luz de la escena, que en una
    // celda de 320 sale borrosa y casi siempre a oscuras. Se usa si Blender no
    // está o no supo exportar la escena.
    if leida.is_err() && ext == "blend" {
        if let Some(img) = crate::derivado::miniatura_blend(ruta) {
            return Lienzo::de_imagen(img);
        }
    }
    match leida {
        Ok(m) => {
            let img = crate::modelo3d::dibujar(&m, LADO_MODELO);
            Lienzo {
                ancho: img.width(),
                alto: img.height(),
                imagen: Some(DynamicImage::ImageRgb8(img)),
                triangulos: Some(m.triangulos() as u64),
                medidas_mm: m.medidas_mm(),
                ..Default::default()
            }
        }
        Err(e) => Lienzo::a_ciegas(e.to_string()),
    }
}

/// Un fotograma del vídeo y sus datos, con `ffmpeg`.
///
/// Nunca devuelve error: si `ffmpeg` no está o el archivo está roto, el vídeo
/// entra igual y se queda sin cara. Perder el archivo por no tener instalado un
/// programa sería el peor de los dos males.
fn de_video(ruta: &Path) -> Lienzo {
    let (ancho, alto, duracion_ms) = datos_de_video(ruta).unwrap_or((0, 0, None));

    if !externo::hay("ffmpeg") {
        return Lienzo {
            ancho,
            alto,
            duracion_ms,
            sin_imagen: Some("hace falta ffmpeg para ver los vídeos".into()),
            ..Default::default()
        };
    }

    // Un décimo de la duración, no el primer fotograma: los vídeos empiezan en
    // negro, con una claqueta o con un fundido más veces de las que uno cree, y
    // una malla llena de rectángulos negros no sirve de nada.
    let salto = duracion_ms.map(|d| (d / 10).max(500)).unwrap_or(1000);
    let segundos = format!("{}.{:03}", salto / 1000, salto % 1000);
    let tmp = match Temporal::nuevo("fotograma") {
        Ok(t) => t,
        Err(e) => return Lienzo::a_ciegas(e.to_string()),
    };
    let destino = tmp.con_extension("jpg");

    let r = externo::correr_y_leer(
        "ffmpeg",
        &[
            "-v",
            "error",
            // El `-ss` antes del `-i` salta por índice en vez de decodificando:
            // en un archivo de una hora la diferencia es de minutos.
            "-ss",
            &segundos,
            "-i",
            &ruta.to_string_lossy(),
            "-frames:v",
            "1",
            "-f",
            "image2",
            "-y",
            &destino.to_string_lossy(),
        ],
        TOPE_VIDEO,
        &destino,
    );

    match r.ok().and_then(|b| image_ops::decode(&b).ok()) {
        Some(img) => {
            use image::GenericImageView;
            let (w, h) = img.dimensions();
            Lienzo {
                imagen: Some(img),
                // Las medidas buenas son las que dijo ffprobe; el fotograma
                // puede venir escalado.
                ancho: if ancho > 0 { ancho } else { w },
                alto: if alto > 0 { alto } else { h },
                duracion_ms,
                ..Default::default()
            }
        }
        None => Lienzo {
            ancho,
            alto,
            duracion_ms,
            sin_imagen: Some("no pude sacar un fotograma".into()),
            ..Default::default()
        },
    }
}

/// La carátula que el archivo lleve dentro, y su duración.
///
/// La carátula se saca aquí, sin herramientas de fuera: está en la etiqueta del
/// archivo y leerla es seguir longitudes explícitas. La duración sí necesita
/// ffprobe, y si no está, el archivo entra con su portada y sin el dato.
fn de_audio(ruta: &Path, bytes: &[u8]) -> Lienzo {
    let duracion_ms = duracion_de(ruta);
    let caratula = crate::audio::caratula(bytes).and_then(|c| image_ops::decode(c).ok());
    // A 22 kHz y no a menos: a 8 kHz se pierde todo lo que pasa de 4 kHz
    // —platillos, sibilantes— y la miniatura mentía.
    let onda = crate::derivado::analizar_audio(ruta, 22_050).ok();
    let imagen = match (onda, caratula) {
        (Some(a), caratula) => Some(DynamicImage::ImageRgb8(miniatura_de_sonido(&a, caratula))),
        (None, Some(c)) => Some(c),
        (None, None) => None,
    };
    match imagen {
        Some(img) => {
            use image::GenericImageView;
            let (w, h) = img.dimensions();
            Lienzo {
                imagen: Some(img),
                ancho: w,
                alto: h,
                duracion_ms,
                ..Default::default()
            }
        }
        None => Lienzo {
            duracion_ms,
            sin_imagen: Some("el archivo no lleva carátula dentro y no pude leer su sonido".into()),
            ..Default::default()
        },
    }
}

/// La miniatura de un sonido: su onda en el tiempo, y si trae carátula, la
/// carátula a la izquierda.
///
/// La onda va siempre, también con carátula: en la malla lo que distingue un
/// golpe de caja de una canción entera es su forma, y una carátula —la del
/// disco— es la misma para doce pistas. Pero quitarla del todo sería perder lo
/// que el archivo trae, así que se queda en un cuadrado a la izquierda.
fn miniatura_de_sonido(
    a: &crate::senal::Analisis,
    caratula: Option<DynamicImage>,
) -> image::RgbImage {
    const ANCHO: u32 = 1024;
    const ALTO: u32 = 576;
    match caratula {
        None => crate::senal::dibujar_onda(a, ANCHO, ALTO),
        Some(c) => {
            let mut lienzo = image::RgbImage::new(ANCHO, ALTO);
            let cuadro = image::imageops::resize(
                &image_ops::make_thumb(&c, ALTO),
                ALTO,
                ALTO,
                image::imageops::FilterType::Triangle,
            );
            image::imageops::replace(&mut lienzo, &cuadro, 0, 0);
            let onda = crate::senal::dibujar_onda(a, ANCHO - ALTO, ALTO);
            image::imageops::replace(&mut lienzo, &onda, ALTO as i64, 0);
            lienzo
        }
    }
}

/// Solo la duración, para lo que no tiene imagen que mirar.
fn duracion_de(ruta: &Path) -> Option<u64> {
    let dijo = externo::correr_leyendo(
        "ffprobe",
        &[
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=nw=1:nk=1",
            &ruta.to_string_lossy(),
        ],
        TOPE_INFORME,
    )
    .ok()?;
    dijo.lines()
        .next()
        .and_then(|l| l.trim().parse::<f64>().ok())
        .filter(|d| d.is_finite() && *d > 0.0)
        .map(|d| (d * 1000.0) as u64)
}

fn datos_de_video(ruta: &Path) -> Option<(u32, u32, Option<u64>)> {
    let dijo = externo::correr_leyendo(
        "ffprobe",
        &[
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-show_entries",
            "format=duration",
            "-of",
            "default=nw=1:nk=1",
            &ruta.to_string_lossy(),
        ],
        TOPE_INFORME,
    )
    .ok()?;
    let mut lineas = dijo.lines();
    let w = lineas.next()?.trim().parse().unwrap_or(0);
    let h = lineas.next()?.trim().parse().unwrap_or(0);
    let dur = lineas
        .next()
        .and_then(|l| l.trim().parse::<f64>().ok())
        .filter(|d| d.is_finite() && *d > 0.0)
        .map(|d| (d * 1000.0) as u64);
    Some((w, h, dur))
}

/// La primera página del PDF, con `pdftoppm`.
fn de_pdf(ruta: &Path) -> Lienzo {
    let paginas = paginas_de_pdf(ruta);
    if !externo::hay("pdftoppm") {
        return Lienzo {
            paginas,
            sin_imagen: Some("hace falta poppler-utils para ver los PDF".into()),
            ..Default::default()
        };
    }
    let tmp = match Temporal::nuevo("pagina") {
        Ok(t) => t,
        Err(e) => return Lienzo::a_ciegas(e.to_string()),
    };
    // `pdftoppm` le pone la extensión él, así que se le da el prefijo.
    let destino = tmp.con_extension("jpg");
    let r = externo::correr_y_leer(
        "pdftoppm",
        &[
            "-jpeg",
            // 96 puntos por pulgada: una A4 sale a unos 800x1120, que da de
            // sobra para una miniatura de 320 y para la previa de 1024.
            "-r",
            "96",
            "-f",
            "1",
            "-l",
            "1",
            "-singlefile",
            &ruta.to_string_lossy(),
            &tmp.ruta.to_string_lossy(),
        ],
        TOPE_PDF,
        &destino,
    );
    match r.ok().and_then(|b| image_ops::decode(&b).ok()) {
        Some(img) => {
            use image::GenericImageView;
            let (w, h) = img.dimensions();
            Lienzo {
                imagen: Some(img),
                ancho: w,
                alto: h,
                paginas,
                ..Default::default()
            }
        }
        None => Lienzo {
            paginas,
            sin_imagen: Some("no pude dibujar la primera página".into()),
            ..Default::default()
        },
    }
}

fn paginas_de_pdf(ruta: &Path) -> Option<u32> {
    let dijo = externo::correr_leyendo(
        "pdfinfo",
        &["-enc", "UTF-8", &ruta.to_string_lossy()],
        TOPE_INFORME,
    )
    .ok()?;
    dijo.lines()
        .find_map(|l| l.strip_prefix("Pages:"))
        .and_then(|n| n.trim().parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cada_extension_cae_en_su_familia() {
        assert_eq!(familia_de("JPG"), Some(Familia::Imagen));
        assert_eq!(familia_de(".png"), Some(Familia::Imagen));
        assert_eq!(familia_de("cr2"), Some(Familia::Raw));
        assert_eq!(familia_de("mkv"), Some(Familia::Video));
        assert_eq!(familia_de("flac"), Some(Familia::Audio));
        assert_eq!(familia_de("pdf"), Some(Familia::Documento));
        assert_eq!(familia_de("woff2"), Some(Familia::Tipografia));
        assert_eq!(familia_de("STL"), Some(Familia::Modelo));
        assert_eq!(familia_de("blend"), Some(Familia::Modelo));
    }

    #[test]
    fn lo_que_no_es_de_la_casa_no_entra() {
        assert_eq!(familia_de("o"), None);
        assert_eq!(familia_de("log"), None);
        assert_eq!(familia_de(""), None);
        assert!(!se_puede_guardar("DS_Store"));
    }

    #[test]
    fn ninguna_extension_esta_en_dos_familias() {
        let mut vistas = std::collections::HashMap::new();
        for (f, exts) in TABLA {
            for e in *exts {
                if let Some(otra) = vistas.insert(*e, *f) {
                    panic!("«{e}» está en {} y en {}", otra.as_str(), f.as_str());
                }
            }
        }
    }

    #[test]
    fn el_buscador_entiende_los_nombres_de_familia() {
        assert_eq!(Familia::parse("videos"), Some(Familia::Video));
        assert_eq!(Familia::parse("Vídeo"), Some(Familia::Video));
        assert_eq!(Familia::parse("fuentes"), Some(Familia::Tipografia));
        assert_eq!(Familia::parse("cualquiera"), None);
    }

    #[test]
    fn ida_y_vuelta_por_el_nombre() {
        for (f, _) in TABLA {
            assert_eq!(Familia::parse(f.as_str()), Some(*f), "{}", f.as_str());
        }
    }

    #[test]
    fn un_archivo_que_no_sabemos_dibujar_no_es_un_error() {
        let l = abrir(Path::new("/no/existe.mp4"), "mp4", b"no es un mp4").unwrap();
        assert!(l.imagen.is_none());
        assert!(l.imagen.is_none(), "un mp4 que no existe no da imagen");
    }

    #[test]
    fn un_negativo_se_abre_por_la_previa_que_lleva_dentro() {
        use image::{ImageFormat, Rgb, RgbImage};
        // Un JPEG de verdad, metido donde una cámara lo metería.
        // Con ruido y de buen tamaño: un JPEG que comprime a menos de un kilo
        // no pasa el filtro contra falsos positivos, y una vista previa de
        // cámara nunca es tan pequeña.
        let mut im = RgbImage::new(320, 200);
        for (x, y, p) in im.enumerate_pixels_mut() {
            let n = (x * 7919 + y * 104_729) % 251;
            *p = Rgb([n as u8, (x % 256) as u8, (y % 256) as u8]);
        }
        let mut jpeg = std::io::Cursor::new(Vec::new());
        im.write_to(&mut jpeg, ImageFormat::Jpeg).unwrap();
        let jpeg = jpeg.into_inner();

        let mut cr2 = vec![0u8; 4096];
        cr2[0..4].copy_from_slice(b"II\x2a\x00");
        cr2.extend_from_slice(&jpeg);

        let l = abrir(Path::new("x.cr2"), "cr2", &cr2).unwrap();
        assert!(
            l.imagen.is_some(),
            "sale del rastreo aunque el TIFF sea falso"
        );
        assert_eq!((l.ancho, l.alto), (320, 200));
    }

    #[test]
    fn un_negativo_sin_previa_entra_a_ciegas_pero_entra() {
        let l = abrir(Path::new("x.nef"), "nef", &vec![0u8; 8000]).unwrap();
        assert!(l.imagen.is_none());
        assert!(l.sin_imagen.unwrap().contains("vista previa"));
    }

    #[test]
    fn una_imagen_rota_si_es_un_error() {
        assert!(abrir(Path::new("x.png"), "png", b"esto no es un png").is_err());
    }
}

/// Pruebas con las herramientas de fuera de verdad.
///
/// Se saltan solas si la herramienta no está instalada: la promesa del núcleo
/// es que se puede probar sin escritorio, no que todo el mundo tenga ffmpeg.
/// Lo que **sí** se comprueba siempre es que no tenerlas no rompe nada.
#[cfg(test)]
mod tests_fuera {
    use super::*;

    #[test]
    fn un_video_de_verdad_da_su_fotograma_y_su_duracion() {
        if !externo::hay("ffmpeg") {
            eprintln!("sin ffmpeg: prueba saltada");
            return;
        }
        let tmp = Temporal::nuevo("prueba").unwrap();
        let video = tmp.con_extension("mp4");
        // `testsrc` es el patrón de barras que trae ffmpeg dentro: no hace
        // falta ningún archivo de muestra en el repositorio.
        let hecho = externo::correr(
            "ffmpeg",
            &[
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "testsrc=duration=3:size=320x240:rate=10",
                "-pix_fmt",
                "yuv420p",
                "-y",
                &video.to_string_lossy(),
            ],
            Duration::from_secs(30),
        );
        assert!(
            hecho.is_ok(),
            "no pude generar el vídeo de prueba: {hecho:?}"
        );

        let l = abrir(&video, "mp4", &[]).unwrap();
        assert!(l.imagen.is_some(), "un vídeo tiene que dar un fotograma");
        assert_eq!(
            (l.ancho, l.alto),
            (320, 240),
            "las medidas son las del vídeo"
        );
        let d = l.duracion_ms.expect("y su duración");
        assert!((2500..3500).contains(&d), "duró {d} ms");
        let _ = std::fs::remove_file(&video);
    }

    #[test]
    fn un_video_roto_no_cuelga_la_importacion() {
        let tmp = Temporal::nuevo("roto").unwrap();
        let video = tmp.con_extension("mp4");
        std::fs::write(&video, "esto no es un video, ni de lejos".as_bytes()).unwrap();
        let t = std::time::Instant::now();
        let l = abrir(&video, "mp4", &[]).unwrap();
        assert!(l.imagen.is_none());
        assert!(l.sin_imagen.is_some(), "y dice por qué");
        assert!(
            t.elapsed() < Duration::from_secs(15),
            "sin quedarse esperando"
        );
        let _ = std::fs::remove_file(&video);
    }

    /// Un PDF de una página, escrito a mano.
    ///
    /// Es más corto que traerse una dependencia para generarlo, y de paso deja
    /// claro lo poco que hace falta para que poppler lo dé por bueno.
    fn pdf_de_una_pagina() -> Vec<u8> {
        let objetos: [&str; 5] = [
            "<</Type/Catalog/Pages 2 0 R>>",
            "<</Type/Pages/Kids[3 0 R]/Count 1>>",
            "<</Type/Page/Parent 2 0 R/MediaBox[0 0 200 100]/Contents 4 0 R\
              /Resources<</Font<</F1 5 0 R>>>>>>",
            "<</Length 42>>stream\nBT /F1 24 Tf 20 40 Td (Grimorio) Tj ET\nendstream",
            "<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>",
        ];
        let mut b = Vec::from(&b"%PDF-1.4\n"[..]);
        let mut donde = Vec::new();
        for (i, o) in objetos.iter().enumerate() {
            donde.push(b.len());
            b.extend_from_slice(format!("{} 0 obj{}endobj\n", i + 1, o).as_bytes());
        }
        // La tabla de referencias cruzadas: poppler sabe reconstruirla si está
        // mal, pero un archivo de prueba que solo funciona por la piedad del
        // lector no prueba nada.
        let xref = b.len();
        b.extend_from_slice(format!("xref\n0 {}\n", objetos.len() + 1).as_bytes());
        b.extend_from_slice(b"0000000000 65535 f \n");
        for d in &donde {
            b.extend_from_slice(format!("{d:010} 00000 n \n").as_bytes());
        }
        b.extend_from_slice(
            format!(
                "trailer<</Size {}/Root 1 0 R>>\nstartxref\n{}\n%%EOF\n",
                objetos.len() + 1,
                xref
            )
            .as_bytes(),
        );
        b
    }

    #[test]
    fn un_pdf_de_verdad_da_su_primera_pagina() {
        if !externo::hay("pdftoppm") {
            eprintln!("sin poppler-utils: prueba saltada");
            return;
        }
        let tmp = Temporal::nuevo("prueba").unwrap();
        let doc = tmp.con_extension("pdf");
        std::fs::write(&doc, pdf_de_una_pagina()).unwrap();

        let l = abrir(&doc, "pdf", &[]).unwrap();
        assert!(
            l.imagen.is_some(),
            "sin página dibujada: {:?}",
            l.sin_imagen
        );
        assert!(l.ancho > 100 && l.alto > 50, "salió {}x{}", l.ancho, l.alto);
        if externo::hay("pdfinfo") {
            assert_eq!(l.paginas, Some(1));
        }
        let _ = std::fs::remove_file(&doc);
    }

    #[test]
    fn un_pdf_roto_entra_igual_y_lo_cuenta() {
        let tmp = Temporal::nuevo("roto").unwrap();
        let doc = tmp.con_extension("pdf");
        std::fs::write(&doc, b"%PDF-1.4 y hasta aqui").unwrap();
        let l = abrir(&doc, "pdf", &[]).unwrap();
        assert!(l.imagen.is_none());
        assert!(l.sin_imagen.is_some());
        let _ = std::fs::remove_file(&doc);
    }
}
