//! Lo que se saca de un original para poder enseñarlo, bajo demanda.
//!
//! La miniatura y la previsualización se hacen al importar porque la malla las
//! necesita siempre. Esto otro solo hace falta cuando alguien abre el elemento,
//! y algunas cosas tardan —convertir un ProRes de diez minutos, analizar una
//! hora de audio—, así que se hace la primera vez que se pide y se guarda.
//!
//! Todo vive en `cache/` dentro de la biblioteca, con el mismo reparto en dos
//! niveles que `items/`. Es **derivado**: se puede borrar entero y se rehace
//! solo. Los originales no cambian nunca —su identidad es su contenido—, así
//! que la clave es el id y no hace falta mirar fechas.
//!
//! Nada de aquí toca el índice ni la biblioteca abierta: recibe rutas y
//! devuelve rutas. Por eso se puede llamar desde cualquier hilo, a la vez que
//! el bus de comandos sigue atendiendo, que es justo lo que hace la interfaz.

use crate::error::{Error, Result};
use crate::externo::{self, Fallo, Temporal};
use crate::{modelo3d, senal};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Convertir un vídeo entero puede llevar minutos. Una hora es el límite de
/// lo razonable; más que eso es que algo se ha colgado.
const TOPE_PROXY: Duration = Duration::from_secs(3600);
const TOPE_AUDIO: Duration = Duration::from_secs(600);
const TOPE_TIRA: Duration = Duration::from_secs(300);
const TOPE_PAGINA: Duration = Duration::from_secs(60);
/// Blender tarda en arrancar y una escena grande tarda en exportarse.
const TOPE_BLENDER: Duration = Duration::from_secs(180);

fn mal(e: impl std::fmt::Display) -> Error {
    Error::Invalid(e.to_string())
}

fn fallo(programa: &str, f: Fallo) -> Error {
    match f {
        Fallo::NoEsta => Error::Invalid(format!("hace falta {programa} y no está instalado")),
        otro => Error::Invalid(format!("{programa} {otro}")),
    }
}

/// Dónde vive un derivado. `nombre` va detrás del id: `01JK…-p3-1024.jpg`.
pub fn ruta(raiz: &Path, tipo: &str, id: &str, nombre: &str) -> PathBuf {
    let a = id.get(0..2).unwrap_or("00");
    let b = id.get(2..4).unwrap_or("00");
    raiz.join("cache")
        .join(tipo)
        .join(a)
        .join(b)
        .join(format!("{id}{nombre}"))
}

/// Un archivo a medio escribir al lado del definitivo.
///
/// Al lado y no en `/tmp`: `rename` solo es atómico dentro del mismo sistema
/// de archivos, y la biblioteca puede estar en otro disco. Atómico importa
/// porque dos hilos pueden pedir lo mismo a la vez, y el que llegue tarde no
/// puede ver un archivo a medias.
struct Parcial {
    ruta: PathBuf,
}

impl Parcial {
    fn para(destino: &Path) -> Result<Parcial> {
        if let Some(d) = destino.parent() {
            std::fs::create_dir_all(d).map_err(|source| Error::Io {
                path: d.to_path_buf(),
                source,
            })?;
        }
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let mut nombre = destino.file_name().unwrap_or_default().to_os_string();
        nombre.push(format!(
            ".{}-{}.parcial",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        Ok(Parcial {
            ruta: destino.with_file_name(nombre),
        })
    }

    /// La ruta del parcial con la extensión del destino al final, para las
    /// herramientas que deciden el formato por la extensión.
    fn con_ext(&self, destino: &Path) -> PathBuf {
        match destino.extension() {
            Some(e) => {
                let mut p = self.ruta.clone().into_os_string();
                p.push(".");
                p.push(e);
                PathBuf::from(p)
            }
            None => self.ruta.clone(),
        }
    }

    fn colocar(&self, hecho: &Path, destino: &Path) -> Result<()> {
        match std::fs::metadata(hecho) {
            Ok(m) if m.len() > 0 => {}
            _ => return Err(mal("la herramienta terminó sin escribir nada")),
        }
        std::fs::rename(hecho, destino).map_err(|source| Error::Io {
            path: destino.to_path_buf(),
            source,
        })
    }
}

impl Drop for Parcial {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.ruta);
        for ext in ["mp4", "jpg", "png", "glb", "onda", "malla"] {
            let mut p = self.ruta.clone().into_os_string();
            p.push(".");
            p.push(ext);
            let _ = std::fs::remove_file(p);
        }
    }
}

// ------------------------------------------------------------------ vídeo

/// Una copia del vídeo que cualquier reproductor sabe abrir.
///
/// Existe por los códecs de edición: ProRes, DNxHD, CineForm. ffmpeg los lee
/// todos, pero el GStreamer que hay debajo de Qt solo los decodifica si está
/// instalado `gstreamer1.0-libav`, y en Ubuntu no viene. Sin esto, el visor
/// dice «Internal data stream error» y se queda en la portada.
///
/// H.264 de 8 bits en 4:2:0 es lo único que todo decodificador entiende —el de
/// la tarjeta incluido—. Se limita a 1920 de ancho: es para mirar, no para
/// editar, y un 4K ProRes convertido a 4K tarda cuatro veces más para nada.
pub fn proxy_video(raiz: &Path, id: &str, original: &Path) -> Result<PathBuf> {
    let destino = ruta(raiz, "proxy", id, ".mp4");
    if destino.exists() {
        return Ok(destino);
    }
    let p = Parcial::para(&destino)?;
    let salida = p.con_ext(&destino);
    externo::correr(
        "ffmpeg",
        &[
            "-v",
            "error",
            "-y",
            "-i",
            &original.to_string_lossy(),
            "-map",
            "0:v:0",
            "-map",
            "0:a:0?",
            "-vf",
            "scale='trunc(min(1920,iw)/2)*2':-2",
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
            "-crf",
            "20",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            // El índice delante: el reproductor puede saltar sin leerlo entero.
            "-movflags",
            "+faststart",
            &salida.to_string_lossy(),
        ],
        TOPE_PROXY,
    )
    .map_err(|f| fallo("ffmpeg", f))?;
    p.colocar(&salida, &destino)?;
    Ok(destino)
}

/// Fotogramas del vídeo en rejilla, para enseñar dónde se va a saltar al pasar
/// el ratón por la barra.
///
/// Una sola imagen con todos y no un archivo por fotograma: se decodifica una
/// vez y luego enseñar uno es recortar, que no cuesta nada mientras el ratón
/// se mueve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tira {
    pub columnas: u32,
    pub filas: u32,
    pub n: u32,
    /// Segundos entre un fotograma y el siguiente.
    pub cada_s: f32,
}

pub const TIRA_ANCHO: u32 = 192;
/// Desde cuántos segundos la tira sale solo de los fotogramas clave.
const TIRA_SOLO_CLAVES_S: f32 = 90.0;

pub fn tira_video(
    raiz: &Path,
    id: &str,
    original: &Path,
    duracion_s: f32,
) -> Result<(PathBuf, Tira)> {
    if !(duracion_s.is_finite() && duracion_s > 0.0) {
        return Err(mal("el vídeo no dice cuánto dura"));
    }
    // Uno por segundo en los cortos, cien como mucho en los largos.
    let n = (duracion_s.ceil() as u32).clamp(1, 100);
    let columnas = 10.min(n);
    let filas = n.div_ceil(columnas);
    let tira = Tira {
        columnas,
        filas,
        n,
        cada_s: duracion_s / n as f32,
    };
    let destino = ruta(raiz, "tira", id, &format!("-{n}.jpg"));
    if destino.exists() {
        return Ok((destino, tira));
    }
    let p = Parcial::para(&destino)?;
    let salida = p.con_ext(&destino);
    let filtro = format!("fps={n}/{duracion_s:.3},scale={TIRA_ANCHO}:-2,tile={columnas}x{filas}");
    let orig = original.to_string_lossy();
    let sal = salida.to_string_lossy();
    let mut args: Vec<&str> = vec!["-v", "error", "-y"];
    // En un vídeo largo, solo los fotogramas clave. Decodificarlo entero se
    // comía todos los núcleos justo al abrirlo, mientras se pincha la barra:
    // medido con un vídeo de un minuto, 6,6 s de procesador contra 0,5. La
    // vista previa sale del clave más cercano, que para saber por dónde se
    // va basta. En uno corto se decodifica entero: es barato, y con claves
    // cada diez segundos la tira saldría casi vacía.
    if duracion_s > TIRA_SOLO_CLAVES_S {
        args.extend_from_slice(&["-skip_frame", "nokey"]);
    }
    args.extend_from_slice(&["-i", &orig, "-an", "-vf", &filtro, "-frames:v", "1", "-q:v", "4", &sal]);
    externo::correr("ffmpeg", &args, TOPE_TIRA).map_err(|f| fallo("ffmpeg", f))?;
    p.colocar(&salida, &destino)?;
    Ok((destino, tira))
}

// ----------------------------------------------------------------- tramos

/// Corta un tramo de un vídeo o de un sonido a un archivo nuevo en
/// `cache/versiones/`, de donde [`crate::Library::anadir_version`] lo mete en
/// la pila moviéndolo. Va aquí y no en la biblioteca porque puede tardar (el
/// corte exacto recodifica) y así se llama desde el hilo de los derivados,
/// sin parar el de editar.
///
/// Rápido por defecto: ffmpeg copia los paquetes del tramo sin decodificarlos
/// (`-c copy`), y tarda lo que tarda leerlos aunque el vídeo sea de una hora.
/// El archivo lleva desde el fotograma clave de antes, pero en MP4 y MOV una
/// lista de edición hace que se vea desde el punto pedido; otros contenedores
/// pueden enseñar ese trozo de más. Con `exacto` se recodifica y corta justo.
pub fn cortar_tramo(raiz: &Path, original: &Path, tramo: crate::Tramo) -> Result<PathBuf> {
    if tramo.hasta_ms <= tramo.desde_ms {
        return Err(mal("el tramo acaba antes de empezar"));
    }
    let ext = original
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp4")
        .to_ascii_lowercase();
    let dir = raiz.join("cache").join("versiones");
    std::fs::create_dir_all(&dir).map_err(|source| Error::Io {
        path: dir.clone(),
        source,
    })?;
    let destino = dir.join(format!("{}.{ext}", crate::id::new_id()));
    let desde = format!("{:.3}", tramo.desde_ms as f64 / 1000.0);
    let dura = format!("{:.3}", (tramo.hasta_ms - tramo.desde_ms) as f64 / 1000.0);
    let orig = original.to_string_lossy();
    let sal = destino.to_string_lossy();
    let mut args: Vec<&str> = vec!["-v", "error", "-y", "-ss", &desde, "-i", &orig, "-t", &dura];
    if tramo.exacto {
        // Que ffmpeg elija los códecs del contenedor; en MP4 y compañía,
        // H.264 casi sin pérdida y deprisa.
        args.extend_from_slice(&["-map", "0:v?", "-map", "0:a?"]);
        if matches!(ext.as_str(), "mp4" | "m4v" | "mov" | "mkv") {
            args.extend_from_slice(&["-c:v", "libx264", "-crf", "18", "-preset", "veryfast"]);
        }
    } else {
        // Sin `-avoid_negative_ts make_zero` a propósito: con él, lo que hay
        // desde el fotograma clave de antes se veía entero (un tramo de 8 s
        // salía de 15 con claves cada 10). Sin él, MP4 y MOV guardan una
        // lista de edición y empiezan a verse justo donde se pidió.
        args.extend_from_slice(&["-map", "0", "-c", "copy"]);
    }
    args.push(&sal);
    if let Err(f) = externo::correr("ffmpeg", &args, TOPE_PROXY) {
        let _ = std::fs::remove_file(&destino);
        return Err(fallo("ffmpeg", f));
    }
    match std::fs::metadata(&destino) {
        Ok(m) if m.len() > 0 => Ok(destino),
        _ => {
            let _ = std::fs::remove_file(&destino);
            Err(mal("ffmpeg no sacó nada de ese tramo"))
        }
    }
}

// ------------------------------------------------------------------ audio

/// Tasa a la que se analiza. 44,1 kHz llega hasta 22 kHz, lo que oye una
/// persona; analizar a 96 kHz un archivo de estudio duplicaría el trabajo para
/// enseñar frecuencias que nadie oye.
pub const TASA_ANALISIS: u32 = 44_100;

/// La onda y el espectro de un sonido, en un `.onda` (ver [`senal::Analisis`]).
///
/// Vale también para el audio de un vídeo: lo que se analiza es su primera
/// pista de sonido.
pub fn onda(raiz: &Path, id: &str, original: &Path) -> Result<PathBuf> {
    let destino = ruta(raiz, "onda", id, ".onda");
    if destino.exists() {
        return Ok(destino);
    }
    let a = analizar_audio(original, TASA_ANALISIS)?;
    let p = Parcial::para(&destino)?;
    let mut f = std::fs::File::create(&p.ruta).map_err(mal)?;
    a.escribir(&mut f).map_err(mal)?;
    drop(f);
    p.colocar(&p.ruta, &destino)?;
    Ok(destino)
}

/// Decodifica la primera pista de sonido a `tasa` y la analiza.
pub fn analizar_audio(original: &Path, tasa: u32) -> Result<senal::Analisis> {
    let pcm = Temporal::nuevo("pcm").map_err(mal)?;
    let tasa_txt = tasa.to_string();
    externo::correr(
        "ffmpeg",
        &[
            "-v",
            "error",
            "-y",
            "-i",
            &original.to_string_lossy(),
            "-map",
            "0:a:0",
            "-ac",
            "1",
            "-ar",
            &tasa_txt,
            "-f",
            "f32le",
            &pcm.ruta.to_string_lossy(),
        ],
        TOPE_AUDIO,
    )
    .map_err(|f| fallo("ffmpeg", f))?;
    let a = senal::analizar_archivo(&pcm.ruta, tasa).map_err(mal)?;
    if a.muestras == 0 {
        return Err(mal("el archivo no tiene sonido"));
    }
    Ok(a)
}

// -------------------------------------------------------------------- PDF

/// El tamaño de cada página, en puntos, para poder maquetar el documento
/// entero antes de dibujar ninguna.
pub fn medidas_pdf(original: &Path) -> Result<Vec<[f32; 2]>> {
    let dijo = externo::correr_leyendo(
        "pdfinfo",
        &["-f", "1", "-l", "100000", &original.to_string_lossy()],
        Duration::from_secs(30),
    )
    .map_err(|f| fallo("pdfinfo", f))?;
    // «Page    3 size: 595.276 x 841.89 pts (A4)»
    let mut out = Vec::new();
    for l in dijo.lines() {
        let Some(resto) = l.strip_prefix("Page") else {
            continue;
        };
        let Some(i) = resto.find("size:") else {
            continue;
        };
        let mut w = resto[i + 5..].split_ascii_whitespace();
        let a: Option<f32> = w.next().and_then(|s| s.parse().ok());
        let _x = w.next();
        let b: Option<f32> = w.next().and_then(|s| s.parse().ok());
        if let (Some(a), Some(b)) = (a, b) {
            out.push([a, b]);
        }
    }
    // El giro viene en otra línea, «Page 3 rot: 90», y el tamaño de arriba es
    // el de antes de girar: una página apaisada girada se ve vertical.
    for l in dijo.lines() {
        let Some(resto) = l.strip_prefix("Page") else {
            continue;
        };
        let Some(i) = resto.find("rot:") else {
            continue;
        };
        let n: Option<usize> = resto[..i].trim().parse().ok();
        let rot: Option<i32> = resto[i + 4..].trim().parse().ok();
        if let (Some(n), Some(rot)) = (n, rot) {
            if rot % 180 != 0 {
                if let Some(p) = out.get_mut(n.wrapping_sub(1)) {
                    p.swap(0, 1);
                }
            }
        }
    }
    if out.is_empty() {
        return Err(mal("pdfinfo no dijo el tamaño de ninguna página"));
    }
    Ok(out)
}

/// Una página dibujada a `ancho` píxeles. Uno basado, como las cuenta la gente.
pub fn pagina_pdf(
    raiz: &Path,
    id: &str,
    original: &Path,
    pagina: u32,
    ancho: u32,
) -> Result<PathBuf> {
    let ancho = ancho.clamp(64, 6000);
    let destino = ruta(raiz, "pdf", id, &format!("-p{pagina}-{ancho}.jpg"));
    if destino.exists() {
        return Ok(destino);
    }
    let p = Parcial::para(&destino)?;
    let n = pagina.to_string();
    let a = ancho.to_string();
    externo::correr(
        "pdftoppm",
        &[
            "-jpeg",
            "-jpegopt",
            "quality=90",
            "-f",
            &n,
            "-l",
            &n,
            "-scale-to-x",
            &a,
            "-scale-to-y",
            "-1",
            "-singlefile",
            &original.to_string_lossy(),
            &p.ruta.to_string_lossy(),
        ],
        TOPE_PAGINA,
    )
    .map_err(|f| fallo("pdftoppm", f))?;
    let hecho = p.con_ext(&destino);
    p.colocar(&hecho, &destino)?;
    Ok(destino)
}

// --------------------------------------------------------------------- 3D

/// Dónde está Blender, si está.
///
/// Primero `GRIMORIO_BLENDER`, luego el `blender` del `PATH`. Se resuelve el
/// enlace porque `blender-thumbnailer` vive al lado del binario de verdad, no
/// al lado del enlace de `/usr/local/bin`.
///
/// En Windows el instalador de Blender no lo pone en el `PATH`: se busca
/// también en `%ProgramFiles%\Blender Foundation\Blender X.Y\`, la versión más
/// alta primero. Allí no hay enlaces que resolver, y `canonicalize` devolvería
/// una ruta `\\?\C:\…` que algunos programas no entienden, así que no se toca.
pub fn blender() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("GRIMORIO_BLENDER") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    let nombre = format!("blender{}", std::env::consts::EXE_SUFFIX);
    let en_path = std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|d| d.join(&nombre))
            .find(|p| p.is_file())
    });
    if cfg!(windows) {
        return en_path.or_else(blender_instalado_windows);
    }
    en_path.map(|p| std::fs::canonicalize(&p).unwrap_or(p))
}

/// `%ProgramFiles%\Blender Foundation\Blender*\blender.exe`, la más nueva.
/// Las carpetas se llaman «Blender 4.2», «Blender 5.1»…: se comparan por los
/// números y no por el texto, para que la 4.10 vaya después de la 4.9.
fn blender_instalado_windows() -> Option<PathBuf> {
    let base = PathBuf::from(std::env::var_os("ProgramFiles")?).join("Blender Foundation");
    let version = |p: &Path| -> Vec<u32> {
        p.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
            .split(|c: char| !c.is_ascii_digit())
            .filter_map(|t| t.parse().ok())
            .collect()
    };
    let mut candidatos: Vec<PathBuf> = std::fs::read_dir(base)
        .ok()?
        .flatten()
        .map(|e| e.path().join("blender.exe"))
        .filter(|p| p.is_file())
        .collect();
    candidatos.sort_by_key(|p| p.parent().map(version).unwrap_or_default());
    candidatos.pop()
}

/// La miniatura que Blender guarda dentro de cada `.blend` al salvar.
///
/// La saca `blender-thumbnailer`, que viene con Blender y entiende los `.blend`
/// comprimidos con zstd, que son casi todos desde la 3.0. Arranca en
/// milisegundos: no abre Blender.
pub fn miniatura_blend(original: &Path) -> Option<image::DynamicImage> {
    let b = blender()?;
    let t = b.parent()?.join("blender-thumbnailer");
    if !t.is_file() {
        return None;
    }
    let tmp = Temporal::nuevo("blend").ok()?;
    let png = tmp.con_extension("png");
    let bytes = externo::correr_y_leer(
        &t.to_string_lossy(),
        &[&original.to_string_lossy(), &png.to_string_lossy()],
        Duration::from_secs(20),
        &png,
    )
    .ok()?;
    image::load_from_memory(&bytes).ok()
}

/// Lo que Blender ve, en triángulos sueltos.
///
/// No pasa por el exportador de glTF: con él, una escena de cuatrocientos mil
/// triángulos tardaba 26 s y una de ciento veinte mil, 41. Este guion pide a
/// Blender la geometría ya evaluada —con los modificadores aplicados, que es
/// lo que se ve en el viewport— y la vuelca de una vez con `foreach_get`: las
/// mismas dos escenas tardan 1,5 y 1,9 s, casi todo arrancar Blender.
///
/// Con la geometría van los materiales: el color de base de cada uno, si lo
/// fija o si sale de una imagen, y las coordenadas de textura.
///
/// Sale en el formato de [`modelo3d::leer_tris`].
const GUION_BLENDER: &str = r#"
import bpy, sys, os, struct, tempfile
import numpy as np
args = sys.argv[sys.argv.index('--') + 1:]
src, out, ext = args[0], args[1], args[2]
if ext == 'fbx':
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.fbx(filepath=src)
escena = bpy.context.scene
dg = bpy.context.evaluated_depsgraph_get()
mm = 1000.0 * (escena.unit_settings.scale_length or 1.0)

mats, por_nombre = [], {}
def mat_id(m):
    k = m.name if m else None
    if k not in por_nombre:
        por_nombre[k] = len(mats)
        mats.append(m)
    return por_nombre[k]

V, M, UV = [], [], []
for ob in escena.objects:
    if ob.type not in {'MESH', 'CURVE', 'SURFACE', 'META', 'FONT'} or not ob.visible_get():
        continue
    ev = ob.evaluated_get(dg)
    try:
        me = ev.to_mesh()
    except RuntimeError:
        continue
    if me is None:
        continue
    me.calc_loop_triangles()
    n = len(me.loop_triangles)
    if n:
        co = np.empty(len(me.vertices) * 3, np.float32)
        me.vertices.foreach_get('co', co)
        tri = np.empty(n * 3, np.int32)
        me.loop_triangles.foreach_get('vertices', tri)
        m = np.array(ev.matrix_world, np.float32)
        w = co.reshape(-1, 3) @ m[:3, :3].T + m[:3, 3]
        t = w[tri]
        lazos = np.empty(n * 3, np.int32)
        me.loop_triangles.foreach_get('loops', lazos)
        # Una escala negativa da la vuelta a las caras.
        if np.linalg.det(m[:3, :3]) < 0:
            t = t.reshape(-1, 3, 3)[:, [0, 2, 1]].reshape(-1, 3)
            lazos = lazos.reshape(-1, 3)[:, [0, 2, 1]].reshape(-1)
        V.append(t.astype('<f4'))
        mi = np.empty(n, np.int32)
        me.loop_triangles.foreach_get('material_index', mi)
        huecos = [mat_id(s.material) for s in ev.material_slots] or [mat_id(None)]
        M.append(np.array(huecos, np.uint32)[np.clip(mi, 0, len(huecos) - 1)])
        capa = me.uv_layers.active
        if capa is not None and len(capa.data):
            uv = np.empty(len(me.loops) * 2, np.float32)
            capa.data.foreach_get('uv', uv)
            uv = uv.reshape(-1, 2)[lazos]
            # En Blender el origen de la textura es abajo; en glTF, arriba.
            uv[:, 1] = 1.0 - uv[:, 1]
        else:
            uv = np.zeros((n * 3, 2), np.float32)
        UV.append(uv.astype('<f4'))
    ev.to_mesh_clear()

def srgb(x):
    x = min(max(x, 0.0), 1.0)
    return x * 12.92 if x <= 0.0031308 else 1.055 * x ** (1 / 2.4) - 0.055

def imagen_de(nodo, hondo=0):
    if nodo is None or hondo > 4:
        return None
    if nodo.type == 'TEX_IMAGE':
        return nodo.image
    for e in nodo.inputs:
        for l in e.links:
            i = imagen_de(l.from_node, hondo + 1)
            if i is not None:
                return i
    return None

def material(m):
    if m is None:
        return None, None, True
    dos = not getattr(m, 'use_backface_culling', False)
    color, img = None, None
    if m.use_nodes and m.node_tree:
        nodos = m.node_tree.nodes
        bsdf = next((x for x in nodos if x.type == 'BSDF_PRINCIPLED'), None) or \
               next((x for x in nodos if x.type == 'BSDF_DIFFUSE'), None)
        e = bsdf and (bsdf.inputs.get('Base Color') or bsdf.inputs.get('Color'))
        if e is not None:
            if e.links:
                img = imagen_de(e.links[0].from_node)
            else:
                c = tuple(e.default_value)
                # El gris por defecto de Blender no es un color elegido.
                if any(abs(a - b) > 1e-3 for a, b in zip(c, (0.8, 0.8, 0.8, 1.0))):
                    color = [srgb(c[0]), srgb(c[1]), srgb(c[2]), c[3]]
    else:
        c = tuple(m.diffuse_color)
        if any(abs(a - b) > 1e-3 for a, b in zip(c, (0.8, 0.8, 0.8, 1.0))):
            color = [srgb(c[0]), srgb(c[1]), srgb(c[2]), c[3]]
    if img is not None and color is None:
        color = [1.0, 1.0, 1.0, 1.0]
    return color, img, dos

FIRMAS = (b'\x89PNG', b'\xff\xd8\xff', b'RIFF', b'BM')
def bytes_de(img):
    try:
        if img.packed_file:
            d = bytes(img.packed_file.data)
        else:
            d = open(bpy.path.abspath(img.filepath, library=img.library), 'rb').read()
        if d[:4].startswith(FIRMAS) or d[:3] == b'\xff\xd8\xff':
            return d
    except Exception:
        pass
    # TGA, EXR, lo que sea: se pasa a PNG.
    try:
        tmp = os.path.join(tempfile.gettempdir(), 'grimorio-%d.png' % os.getpid())
        escena.render.image_settings.file_format = 'PNG'
        img.save_render(tmp, scene=escena)
        d = open(tmp, 'rb').read()
        os.remove(tmp)
        return d
    except Exception:
        return None

imagenes, por_imagen = [], {}
info = []
for m in mats:
    color, img, dos = material(m)
    t = -1
    if img is not None:
        if img.name not in por_imagen:
            d = bytes_de(img)
            por_imagen[img.name] = len(imagenes) if d else -1
            if d:
                imagenes.append(d)
        t = por_imagen[img.name]
    info.append((m.name if m else '', color, t, dos))

with open(out, 'wb') as f:
    n = sum(len(x) for x in M)
    f.write(b'GRIMTRI2')
    f.write(struct.pack('<fI', mm, n))
    for x in V:
        f.write(x.tobytes())
    for x in M:
        f.write(x.astype('<u4').tobytes())
    con_uv = any(t >= 0 for _, _, t, _ in info)
    f.write(struct.pack('<I', int(con_uv)))
    if con_uv:
        for x in UV:
            f.write(x.tobytes())
    f.write(struct.pack('<I', len(info)))
    for nombre, color, t, dos in info:
        b = nombre.encode('utf-8')
        f.write(struct.pack('<I', len(b)) + b)
        f.write(struct.pack('<4fiI', *(color or (1.0, 1.0, 1.0, 1.0)), t,
                            int(dos) | (0 if color else 2)))
    f.write(struct.pack('<I', len(imagenes)))
    for d in imagenes:
        f.write(struct.pack('<I', len(d)) + d)
"#;

pub fn blender_a_tris(original: &Path, ext: &str, destino: &Path) -> Result<()> {
    let b =
        blender().ok_or_else(|| mal("hace falta Blender para abrir este archivo, y no está"))?;
    let orig = original.to_string_lossy();
    let dest = destino.to_string_lossy();
    let mut args: Vec<&str> = vec!["-b"];
    if ext == "blend" {
        args.push(&orig);
    } else {
        args.push("--factory-startup");
    }
    args.extend_from_slice(&[
        "--python-exit-code",
        "1",
        "--python-expr",
        GUION_BLENDER,
        "--",
        &orig,
        &dest,
        ext,
    ]);
    externo::correr(&b.to_string_lossy(), &args, TOPE_BLENDER).map_err(|f| fallo("Blender", f))?;
    match std::fs::metadata(destino) {
        Ok(m) if m.len() > 12 => Ok(()),
        _ => Err(mal("Blender terminó sin volcar nada")),
    }
}

/// Lee un modelo, sea propio o vía Blender.
pub fn leer_modelo(original: &Path, ext: &str) -> Result<modelo3d::Malla> {
    let ext = ext.to_ascii_lowercase();
    if modelo3d::CON_BLENDER.contains(&ext.as_str()) {
        let tmp = Temporal::nuevo("modelo").map_err(mal)?;
        blender_a_tris(original, &ext, &tmp.ruta)?;
        let bytes = std::fs::read(&tmp.ruta).map_err(mal)?;
        return modelo3d::leer_tris(&bytes);
    }
    let bytes = std::fs::read(original).map_err(|source| Error::Io {
        path: original.to_path_buf(),
        source,
    })?;
    modelo3d::leer(&bytes, &ext, Some(original))
}

/// Lo que se sabe de un modelo después de leerlo.
#[derive(Debug, Clone, Copy)]
pub struct InfoMalla {
    pub triangulos: u64,
    pub medidas_mm: Option<[f32; 3]>,
}

/// La malla para el visor (ver [`modelo3d::escribir_malla`]).
pub fn malla(raiz: &Path, id: &str, original: &Path, ext: &str) -> Result<(PathBuf, InfoMalla)> {
    let destino = ruta(raiz, "malla", id, ".malla");
    if let Ok(b) = std::fs::read(&destino) {
        // Las de una versión anterior se rehacen: no traían los materiales.
        if b.len() >= 32
            && &b[..8] == b"GRIMALLA"
            && u32::from_le_bytes([b[8], b[9], b[10], b[11]]) == modelo3d::VERSION_MALLA
        {
            let f = |o: usize| f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
            let med = [f(20), f(24), f(28)];
            return Ok((
                destino,
                InfoMalla {
                    triangulos: u32::from_le_bytes([b[12], b[13], b[14], b[15]]) as u64,
                    medidas_mm: (med[0] >= 0.0).then_some(med),
                },
            ));
        }
    }
    let m = leer_modelo(original, ext)?;
    let info = InfoMalla {
        triangulos: m.triangulos() as u64,
        medidas_mm: m.medidas_mm(),
    };
    let p = Parcial::para(&destino)?;
    let mut f = std::io::BufWriter::new(std::fs::File::create(&p.ruta).map_err(mal)?);
    modelo3d::escribir_malla(&m, &mut f).map_err(mal)?;
    drop(f);
    p.colocar(&p.ruta, &destino)?;
    Ok((destino, info))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_ruta_reparte_en_dos_niveles_como_items() {
        let r = ruta(Path::new("/b"), "onda", "01JKXYZ", ".onda");
        assert_eq!(r, Path::new("/b/cache/onda/01/JK/01JKXYZ.onda"));
    }

    #[test]
    fn la_malla_se_escribe_una_vez_y_luego_se_lee_de_la_cache() {
        let dir = tempfile::tempdir().unwrap();
        let stl = dir.path().join("cubo.stl");
        std::fs::write(&stl, crate::modelo3d::tests::cubo_stl()).unwrap();
        let (r, info) = malla(dir.path(), "01ABCD", &stl, "stl").unwrap();
        assert_eq!(info.triangulos, 12);
        assert!(r.exists());
        // Borrar el original demuestra que la segunda vez no se relee.
        std::fs::remove_file(&stl).unwrap();
        let (r2, info2) = malla(dir.path(), "01ABCD", &stl, "stl").unwrap();
        assert_eq!(r, r2);
        assert_eq!(info2.triangulos, 12);
        assert_eq!(info2.medidas_mm.map(|m| m[0].round()), Some(10.0));
        // Y no queda ningún parcial por ahí.
        let sobra: Vec<_> = std::fs::read_dir(r.parent().unwrap())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("parcial"))
            .collect();
        assert!(sobra.is_empty());
    }

    #[test]
    fn la_tira_de_un_video_largo_sale_de_los_fotogramas_clave() {
        if !externo::hay("ffmpeg") {
            eprintln!("sin ffmpeg: prueba saltada");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let mp4 = dir.path().join("largo.mp4");
        externo::correr(
            "ffmpeg",
            &[
                "-v", "error", "-f", "lavfi", "-i", "testsrc=size=160x90:rate=10:duration=100",
                "-g", "50", "-pix_fmt", "yuv420p", "-y", &mp4.to_string_lossy(),
            ],
            Duration::from_secs(60),
        )
        .unwrap();
        let (r, t) = tira_video(dir.path(), "01TIRA", &mp4, 100.0).unwrap();
        assert_eq!((t.n, t.columnas, t.filas), (100, 10, 10));
        let (w, h) = image::image_dimensions(&r).unwrap();
        assert_eq!(w, TIRA_ANCHO * 10);
        assert!(h > 0);
    }

    #[test]
    fn un_audio_de_verdad_da_su_onda() {
        if !externo::hay("ffmpeg") {
            eprintln!("sin ffmpeg: prueba saltada");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("la.wav");
        externo::correr(
            "ffmpeg",
            &[
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=2",
                "-y",
                &wav.to_string_lossy(),
            ],
            Duration::from_secs(30),
        )
        .unwrap();
        let r = onda(dir.path(), "01ONDA", &wav).unwrap();
        let a = senal::Analisis::leer(&mut std::fs::File::open(r).unwrap()).unwrap();
        assert_eq!(a.tasa, TASA_ANALISIS);
        assert!((a.muestras as i64 - 88_200).abs() < 2_000, "{}", a.muestras);
    }

    #[test]
    fn un_video_en_prores_sale_en_h264() {
        if !externo::hay("ffmpeg") || !externo::hay("ffprobe") {
            eprintln!("sin ffmpeg: prueba saltada");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let mov = dir.path().join("x.mov");
        let hecho = externo::correr(
            "ffmpeg",
            &[
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "testsrc=duration=1:size=320x240:rate=10",
                "-c:v",
                "prores_ks",
                "-y",
                &mov.to_string_lossy(),
            ],
            Duration::from_secs(60),
        );
        if hecho.is_err() {
            eprintln!("este ffmpeg no escribe ProRes: prueba saltada");
            return;
        }
        let r = proxy_video(dir.path(), "01PRORES", &mov).unwrap();
        let dijo = externo::correr_leyendo(
            "ffprobe",
            &[
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=codec_name,pix_fmt",
                "-of",
                "default=nw=1:nk=1",
                &r.to_string_lossy(),
            ],
            Duration::from_secs(10),
        )
        .unwrap();
        assert!(dijo.contains("h264") && dijo.contains("yuv420p"), "{dijo}");
        let (t, tira) = tira_video(dir.path(), "01PRORES", &mov, 1.0).unwrap();
        assert!(t.exists());
        assert_eq!(tira.n, 1);
    }

    #[test]
    fn sin_herramienta_se_dice_que_falta() {
        let e = fallo("pdftoppm", Fallo::NoEsta).to_string();
        assert!(e.contains("pdftoppm") && e.contains("no está"));
    }
}
