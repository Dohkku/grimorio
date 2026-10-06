//! Archivos de diseño y tipografías: sacarles una imagen.
//!
//! Ninguno se decodifica aquí. Cada formato tiene ya quien lo sabe leer, y
//! reescribir un lector de PSD para una miniatura sería meses de trabajo para
//! hacerlo peor:
//!
//!   psd, psb        ImageMagick, la capa 0 (la imagen ya compuesta)
//!   xcf             ImageMagick, todas las capas aplanadas
//!   ai              pdftoppm: un .ai moderno es un PDF por dentro
//!   eps             Ghostscript, recortado a su caja
//!   kra, ora        son zips con `mergedimage.png` dentro
//!   sketch          zip con `previews/preview.png`
//!   ttf, otf, ttc   una muestra de texto dibujada con la propia letra
//!
//! Sin la herramienta, el elemento entra igual, a ciegas, como antes.

use crate::externo::{correr, correr_leyendo, Fallo, Temporal};
use image::DynamicImage;
use std::path::Path;
use std::time::Duration;

/// Las extensiones de diseño que entran como imágenes.
pub const EXTENSIONES: &[&str] = &["psd", "psb", "xcf", "kra", "ora", "sketch", "ai", "eps"];

/// El lado mayor de lo que sale. Lo bastante para la previsualización de
/// 1024 px y para acercarse un poco en el visor.
const LADO: u32 = 2048;
const TOPE: Duration = Duration::from_secs(30);

pub fn es_diseno(ext: &str) -> bool {
    EXTENSIONES.contains(&ext.to_ascii_lowercase().as_str())
}

fn leer_png(ruta: &Path) -> Result<DynamicImage, String> {
    let bytes = std::fs::read(ruta).map_err(|e| format!("no salió imagen: {e}"))?;
    crate::image_ops::decode(&bytes).map_err(|e| e.to_string())
}

fn fallo(quien: &str, f: Fallo) -> String {
    match f {
        Fallo::NoEsta => format!("falta {quien} para dibujarlo"),
        otro => format!("{quien}: {otro}"),
    }
}

/// Saca un archivo de dentro de un zip (kra, ora, sketch).
///
/// Antes se llamaba a `unzip`, que está en cualquier escritorio Linux pero no
/// en Windows. Leer una entrada de un zip son cuarenta líneas y el inflado ya
/// venía dentro (`miniz_oxide`, el mismo del PNG), así que se hace aquí y deja
/// de depender de nada de fuera.
fn de_zip(ruta: &Path, dentro: &[&str]) -> Result<DynamicImage, String> {
    for nombre in dentro {
        if let Some(bytes) = entrada_zip(ruta, nombre)? {
            if let Ok(img) = crate::image_ops::decode(&bytes) {
                return Ok(img);
            }
        }
    }
    Err("el archivo no trae vista previa dentro".into())
}

/// Lee una entrada de un zip sin cargar el zip entero: un `.kra` puede pesar
/// cientos de megas y la vista previa son unos pocos. Se busca el índice del
/// final, se localiza la entrada por su nombre y se lee solo ese trozo. Sin
/// zip64 ni cifrado, que ningún programa de dibujo usa para esto.
///
/// `Ok(None)` es que el zip está bien pero no la trae; `Err`, que no es un zip.
fn entrada_zip(ruta: &Path, buscada: &str) -> Result<Option<Vec<u8>>, String> {
    use std::io::{Read, Seek, SeekFrom};
    let no_zip = || "el archivo no es un zip".to_string();
    let u16_le = |b: &[u8], o: usize| u16::from_le_bytes([b[o], b[o + 1]]) as usize;
    let u32_le =
        |b: &[u8], o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) as usize;
    let mut f = std::fs::File::open(ruta).map_err(|e| e.to_string())?;
    let largo = f.metadata().map_err(|e| e.to_string())?.len();
    // El final del índice está en los últimos 22 bytes, más un comentario de
    // hasta 64 KiB.
    let cola = largo.min(22 + 65_535);
    let mut b = vec![0u8; cola as usize];
    f.seek(SeekFrom::Start(largo - cola))
        .map_err(|e| e.to_string())?;
    f.read_exact(&mut b).map_err(|e| e.to_string())?;
    let fin = (0..b.len().saturating_sub(21))
        .rev()
        .find(|&o| u32_le(&b, o) == 0x0605_4b50)
        .ok_or_else(no_zip)?;
    let n = u16_le(&b, fin + 10);
    let tam_indice = u32_le(&b, fin + 12);
    let ini_indice = u32_le(&b, fin + 16) as u64;
    if ini_indice + tam_indice as u64 > largo {
        return Err(no_zip());
    }
    let mut indice = vec![0u8; tam_indice];
    f.seek(SeekFrom::Start(ini_indice))
        .map_err(|e| e.to_string())?;
    f.read_exact(&mut indice).map_err(|e| e.to_string())?;
    let mut o = 0;
    for _ in 0..n {
        if o + 46 > indice.len() || u32_le(&indice, o) != 0x0201_4b50 {
            return Err("el índice del zip está roto".into());
        }
        let metodo = u16_le(&indice, o + 10);
        let comprimido = u32_le(&indice, o + 20);
        let ln = u16_le(&indice, o + 28);
        let extra = u16_le(&indice, o + 30) + u16_le(&indice, o + 32);
        let local = u32_le(&indice, o + 42) as u64;
        let nombre = indice.get(o + 46..o + 46 + ln).unwrap_or_default();
        o += 46 + ln + extra;
        if nombre != buscada.as_bytes() {
            continue;
        }
        if local + 30 + comprimido as u64 > largo {
            return Err("el zip está cortado".into());
        }
        let mut cab = [0u8; 30];
        f.seek(SeekFrom::Start(local)).map_err(|e| e.to_string())?;
        f.read_exact(&mut cab).map_err(|e| e.to_string())?;
        let saltar = (u16_le(&cab, 26) + u16_le(&cab, 28)) as i64;
        f.seek(SeekFrom::Current(saltar))
            .map_err(|e| e.to_string())?;
        let mut datos = vec![0u8; comprimido];
        f.read_exact(&mut datos)
            .map_err(|_| "el zip está cortado".to_string())?;
        return match metodo {
            0 => Ok(Some(datos)),
            8 => miniz_oxide::inflate::decompress_to_vec(&datos)
                .map(Some)
                .map_err(|_| format!("no pude inflar {buscada}")),
            m => Err(format!("zip con compresión {m}, que no conozco")),
        };
    }
    Ok(None)
}

/// ImageMagick, con el nombre que tenga en este sistema.
///
/// La versión 7 se llama `magick`; la 6, que es la que traen muchas
/// distribuciones, solo `convert`. En Windows nunca se prueba `convert`: ese
/// nombre es `C:\Windows\System32\convert.exe`, la herramienta que pasa un
/// disco de FAT a NTFS, y no hay que lanzarla ni por error.
fn imagemagick() -> &'static str {
    if cfg!(windows) || crate::externo::hay("magick") {
        "magick"
    } else {
        "convert"
    }
}

/// Una imagen de un archivo de diseño.
pub fn previa(ruta: &Path, ext: &str) -> Result<DynamicImage, String> {
    let ruta_s = ruta.to_str().ok_or("ruta no es UTF-8")?;
    let ext = ext.to_ascii_lowercase();
    let salida = Temporal::nuevo("diseno").map_err(|e| e.to_string())?;
    let png = salida.con_extension("png");
    let png_s = png.to_str().ok_or("ruta temporal no es UTF-8")?;
    let tam = format!("{LADO}x{LADO}>");
    match ext.as_str() {
        "psd" | "psb" => {
            let capa0 = format!("{ruta_s}[0]");
            correr(imagemagick(), &[&capa0, "-resize", &tam, png_s], TOPE)
                .map_err(|f| fallo("ImageMagick", f))?;
            leer_png(&png)
        }
        "xcf" => {
            correr(
                imagemagick(),
                &[ruta_s, "-background", "white", "-flatten", "-resize", &tam, png_s],
                TOPE,
            )
            .map_err(|f| fallo("ImageMagick", f))?;
            leer_png(&png)
        }
        "ai" => {
            // pdftoppm añade la extensión él solo.
            let base = salida.ruta.to_str().ok_or("ruta temporal no es UTF-8")?;
            let lado = LADO.to_string();
            correr(
                "pdftoppm",
                &["-png", "-singlefile", "-f", "1", "-l", "1", "-scale-to", &lado, ruta_s, base],
                TOPE,
            )
            .map_err(|f| fallo("pdftoppm", f))?;
            leer_png(&png)
        }
        "eps" => {
            let destino = format!("-sOutputFile={png_s}");
            // En Windows Ghostscript no se llama `gs`: el de consola, el que no
            // abre ventana, es `gswin64c`.
            let gs = if cfg!(windows) { "gswin64c" } else { "gs" };
            correr(
                gs,
                &["-q", "-dSAFER", "-dBATCH", "-dNOPAUSE", "-dEPSCrop", "-sDEVICE=png16m",
                  "-r150", "-dTextAlphaBits=4", "-dGraphicsAlphaBits=4", &destino, ruta_s],
                TOPE,
            )
            .map_err(|f| fallo("Ghostscript", f))?;
            leer_png(&png)
        }
        "kra" | "ora" => de_zip(ruta, &["mergedimage.png", "Thumbnails/thumbnail.png", "preview.png"]),
        "sketch" => de_zip(ruta, &["previews/preview.png"]),
        _ => Err(format!("todavía no sé dibujar .{ext}")),
    }
}

/// Si los primeros bytes son los de una fuente que se puede leer: la firma de
/// TrueType, OpenType, colección o WOFF y, en las que tienen directorio de
/// tablas, un número de tablas que quepa en lo que mide el archivo.
fn parece_tipografia(cabeza: &[u8], largo: u64) -> bool {
    if cabeza.len() < 12 {
        return false;
    }
    match &cabeza[0..4] {
        b"wOFF" | b"wOF2" | b"ttcf" => true,
        [0, 1, 0, 0] | b"OTTO" | b"true" | b"typ1" => {
            let tablas = u16::from_be_bytes([cabeza[4], cabeza[5]]) as u64;
            tablas > 0 && largo >= 12 + 16 * tablas
        }
        _ => false,
    }
}

/// Una muestra de la tipografía: su nombre, letras grandes, una frase y los
/// números. Clara sobre fondo claro, que es como se mira una letra.
///
/// El nombre de la familia lo da `fc-scan`, que en Windows no existe: allí se
/// queda en el nombre del archivo, que casi siempre basta.
pub fn muestra_tipografia(ruta: &Path) -> Result<DynamicImage, String> {
    let ruta_s = ruta.to_str().ok_or("ruta no es UTF-8")?;
    // Antes de dibujar, que sea una fuente de verdad. Con una rota, ImageMagick
    // no falla: pone su letra por defecto y la muestra sale igual, con lo que
    // un archivo corrupto parecía una tipografía sana. En Linux no se notaba
    // porque las máquinas sin ImageMagick no sacaban nada; en Windows sí.
    let cabeza = {
        use std::io::Read;
        let mut b = Vec::with_capacity(4096);
        std::fs::File::open(ruta)
            .and_then(|f| f.take(4096).read_to_end(&mut b))
            .map_err(|e| e.to_string())?;
        b
    };
    if !parece_tipografia(&cabeza, std::fs::metadata(ruta).map(|m| m.len()).unwrap_or(0)) {
        return Err("no es una tipografía que se pueda leer".into());
    }
    let familia = correr_leyendo("fc-scan", &["--format", "%{family[0]} %{style[0]}", ruta_s], TOPE)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            ruta.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string()
        });
    let salida = Temporal::nuevo("letra").map_err(|e| e.to_string())?;
    let png = salida.con_extension("png");
    let png_s = png.to_str().ok_or("ruta temporal no es UTF-8")?;
    // El nombre va con la letra de la propia fuente: si no tiene esos glifos,
    // ImageMagick pone otros, y eso también dice algo de la fuente.
    let titulo = familia.replace('%', "%%");
    correr(
        imagemagick(),
        &[
            "-size", "1024x640", "xc:#f2f0ec",
            "-font", ruta_s, "-fill", "#8a8179", "-pointsize", "34", "-annotate", "+56+80", &titulo,
            "-fill", "#1a1a1a", "-pointsize", "170", "-annotate", "+50+290", "Aa Bb Cc",
            "-pointsize", "50", "-annotate", "+56+420", "El veloz murciélago hindú",
            "-annotate", "+56+500", "comía feliz cardillo y kiwi.",
            "-fill", "#8a8179", "-annotate", "+56+590", "0123456789 &?!",
            png_s,
        ],
        TOPE,
    )
    .map_err(|f| fallo("ImageMagick", f))?;
    leer_png(&png)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hay(p: &str) -> bool {
        std::process::Command::new(p)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
    }

    #[test]
    fn un_psd_sale_con_su_imagen() {
        if !hay(imagemagick()) {
            return;
        }
        let d = tempfile::tempdir().unwrap();
        let psd = d.path().join("cartel.psd");
        let ok = std::process::Command::new(imagemagick())
            .args(["-size", "120x80", "xc:#e04040"])
            .arg(&psd)
            .status()
            .unwrap()
            .success();
        if !ok {
            return;
        }
        let img = previa(&psd, "psd").unwrap();
        assert_eq!((img.width(), img.height()), (120, 80));
    }

    /// Un zip mínimo escrito a mano, sin depender de `zip` (que en Windows no
    /// está). Cada entrada va desinflada o guardada tal cual, para pasar por
    /// los dos métodos. El CRC va a cero porque el lector no lo mira.
    fn zip_a_mano(entradas: &[(&str, &[u8], bool)]) -> Vec<u8> {
        let mut z = Vec::new();
        let mut indice = Vec::new();
        for (nombre, datos, desinflar) in entradas {
            let (metodo, cuerpo) = if *desinflar {
                (8u16, miniz_oxide::deflate::compress_to_vec(datos, 6))
            } else {
                (0u16, datos.to_vec())
            };
            let local = z.len() as u32;
            let comun = |v: &mut Vec<u8>| {
                v.extend_from_slice(&20u16.to_le_bytes()); // versión necesaria
                v.extend_from_slice(&0u16.to_le_bytes()); // banderas
                v.extend_from_slice(&metodo.to_le_bytes());
                v.extend_from_slice(&[0; 4]); // hora y fecha
                v.extend_from_slice(&0u32.to_le_bytes()); // crc
                v.extend_from_slice(&(cuerpo.len() as u32).to_le_bytes());
                v.extend_from_slice(&(datos.len() as u32).to_le_bytes());
                v.extend_from_slice(&(nombre.len() as u16).to_le_bytes());
                v.extend_from_slice(&0u16.to_le_bytes()); // extra
            };
            z.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            comun(&mut z);
            z.extend_from_slice(nombre.as_bytes());
            z.extend_from_slice(&cuerpo);

            indice.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            indice.extend_from_slice(&20u16.to_le_bytes()); // versión que lo hizo
            comun(&mut indice);
            indice.extend_from_slice(&[0; 6]); // comentario, disco, atributos internos
            indice.extend_from_slice(&0u32.to_le_bytes()); // atributos externos
            indice.extend_from_slice(&local.to_le_bytes());
            indice.extend_from_slice(nombre.as_bytes());
        }
        let ini = z.len() as u32;
        z.extend_from_slice(&indice);
        z.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        z.extend_from_slice(&[0; 4]);
        z.extend_from_slice(&(entradas.len() as u16).to_le_bytes());
        z.extend_from_slice(&(entradas.len() as u16).to_le_bytes());
        z.extend_from_slice(&(indice.len() as u32).to_le_bytes());
        z.extend_from_slice(&ini.to_le_bytes());
        z.extend_from_slice(&0u16.to_le_bytes());
        z
    }

    fn png(ancho: u32, alto: u32) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(ancho, alto, image::Rgb([10, 200, 90]));
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn un_kra_saca_la_imagen_que_lleva_dentro() {
        let d = tempfile::tempdir().unwrap();
        let kra = d.path().join("dibujo.kra");
        let z = zip_a_mano(&[
            ("mimetype", b"application/x-krita", false),
            ("mergedimage.png", &png(40, 30), true),
        ]);
        std::fs::write(&kra, z).unwrap();
        let sacada = previa(&kra, "kra").unwrap();
        assert_eq!((sacada.width(), sacada.height()), (40, 30));
    }

    /// El mismo, con un zip de verdad si está `zip` a mano: el escrito a mano
    /// prueba el lector, este que el lector entiende lo que hay ahí fuera.
    #[test]
    fn un_kra_hecho_con_zip_tambien_sale() {
        if !hay("zip") {
            return;
        }
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("mergedimage.png"), png(40, 30)).unwrap();
        let kra = d.path().join("dibujo.kra");
        let ok = std::process::Command::new("zip")
            .current_dir(d.path())
            .args(["-q", kra.to_str().unwrap(), "mergedimage.png"])
            .status()
            .unwrap()
            .success();
        if !ok {
            return;
        }
        let sacada = previa(&kra, "kra").unwrap();
        assert_eq!((sacada.width(), sacada.height()), (40, 30));
    }

    #[test]
    fn un_sketch_guardado_sin_comprimir_tambien_sale() {
        let d = tempfile::tempdir().unwrap();
        let sketch = d.path().join("pantalla.sketch");
        let z = zip_a_mano(&[("previews/preview.png", &png(12, 7), false)]);
        std::fs::write(&sketch, z).unwrap();
        let sacada = previa(&sketch, "sketch").unwrap();
        assert_eq!((sacada.width(), sacada.height()), (12, 7));
    }

    #[test]
    fn un_zip_sin_la_vista_previa_lo_dice() {
        let d = tempfile::tempdir().unwrap();
        let ora = d.path().join("capas.ora");
        std::fs::write(&ora, zip_a_mano(&[("stack.xml", b"<image/>", true)])).unwrap();
        assert!(previa(&ora, "ora").is_err());
    }

    #[test]
    fn una_fuente_sale_con_su_muestra() {
        let fuente = Path::new("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf");
        if !hay(imagemagick()) || !fuente.exists() {
            return;
        }
        let img = muestra_tipografia(fuente).unwrap();
        assert_eq!((img.width(), img.height()), (1024, 640));
    }

    #[test]
    fn sin_vista_previa_lo_dice() {
        let d = tempfile::tempdir().unwrap();
        let roto = d.path().join("vacio.sketch");
        std::fs::write(&roto, b"no soy un zip").unwrap();
        assert!(previa(&roto, "sketch").is_err());
        assert!(previa(&roto, "xyz").is_err());
    }

    #[test]
    fn una_fuente_rota_no_se_toma_por_buena() {
        // La firma sola no basta: "OTTO" con nada detrás no es una fuente.
        assert!(!parece_tipografia(b"OTTO...", 7));
        assert!(!parece_tipografia(b"no soy una letra", 16));
        // Una cabecera OpenType con dos tablas cabe en 44 bytes.
        let mut sana = b"OTTO".to_vec();
        sana.extend_from_slice(&[0, 2, 0, 0, 0, 0, 0, 0]);
        assert!(parece_tipografia(&sana, 44));
        assert!(!parece_tipografia(&sana, 20));
        assert!(parece_tipografia(b"wOF2\0\0\0\0\0\0\0\0", 12));
    }
}
