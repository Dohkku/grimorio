//! Herramientas de fuera, en procesos aislados.
//!
//! Hay formatos que no se decodifican en Rust puro sin arrastrar media
//! biblioteca de C dentro del proceso: vídeo (ffmpeg) y PDF (poppler). El plan
//! dice **procesos aislados**, y es por una razón concreta: esos
//! decodificadores se cuelgan o se caen con archivos rotos, y un archivo roto
//! no puede tirar la aplicación ni dejar la importación colgada para siempre.
//!
//! Aquí no se enlaza nada: se llama al programa de la línea de órdenes, con
//! tope de tiempo, y si no está instalado se dice una vez y se sigue. Un vídeo
//! entra en la biblioteca igual; lo único que le falta es la cara.
//!
//! La salida siempre va a un archivo y nunca a una tubería. Leer de una tubería
//! mientras se espera al proceso obliga a hilos o a `select`, y si no se lee, un
//! programa que escriba más de lo que cabe en el búfer se queda bloqueado para
//! siempre esperando a que alguien vacíe. Un archivo temporal no tiene ese
//! problema.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Lo que puede salir mal al llamar a una herramienta de fuera.
#[derive(Debug)]
pub enum Fallo {
    /// No está instalada. Es el caso normal, no un error: se dice una vez.
    NoEsta,
    /// Tardó más de la cuenta y se le mató.
    Tardo,
    /// Terminó mal. Dentro va lo que dijo por la salida de error, recortado.
    Salio(String),
}

impl std::fmt::Display for Fallo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Fallo::NoEsta => write!(f, "no está instalada"),
            Fallo::Tardo => write!(f, "tardó demasiado y se le paró"),
            Fallo::Salio(e) => write!(f, "terminó con error: {e}"),
        }
    }
}

/// Un `Command` para una herramienta de fuera, ya preparado para cada sistema.
///
/// En Windows, un programa de consola lanzado desde una aplicación de ventana
/// abre su propia consola negra: importar cien vídeos serían cien parpadeos de
/// `ffmpeg`. `CREATE_NO_WINDOW` se lo quita. Todo lo que se lance desde el
/// núcleo, la línea de órdenes o el puente pasa por aquí para no olvidarlo.
pub fn orden(programa: impl AsRef<OsStr>) -> Command {
    #[allow(unused_mut)]
    let mut c = Command::new(programa);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}

/// Abre un archivo con el programa que el sistema tenga para él.
///
/// En Linux es `xdg-open` y en macOS `open`; se espera al proceso en un hilo
/// aparte, porque sin el `wait` cada archivo abierto dejaba un zombi hasta que
/// se cerraba la aplicación. En Windows no se lanza nada: `ShellExecuteW` es la
/// llamada que usa el Explorador al hacer doble clic. Pasar por
/// `cmd /C start` sería lo corto, pero `cmd` interpreta `&`, `^` y compañía, y
/// un archivo llamado `a&b.jpg` acabaría ejecutando `b.jpg`.
pub fn abrir_fuera(ruta: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        abrir_fuera_windows(ruta)
    }
    #[cfg(not(windows))]
    {
        let programa = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        let mut hijo = orden(programa)
            .arg(ruta)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        std::thread::spawn(move || {
            let _ = hijo.wait();
        });
        Ok(())
    }
}

#[cfg(windows)]
fn abrir_fuera_windows(ruta: &Path) -> std::io::Result<()> {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "shell32")]
    extern "system" {
        fn ShellExecuteW(
            ventana: *mut c_void,
            verbo: *const u16,
            archivo: *const u16,
            parametros: *const u16,
            carpeta: *const u16,
            mostrar: i32,
        ) -> *mut c_void;
    }
    const SW_SHOWNORMAL: i32 = 1;
    let ancho = |s: &OsStr| s.encode_wide().chain(Some(0)).collect::<Vec<u16>>();
    let verbo = ancho(OsStr::new("open"));
    let archivo = ancho(ruta.as_os_str());
    // SAFETY: las dos cadenas acaban en cero y viven hasta que vuelve la
    // llamada; los punteros nulos son «sin ventana, sin parámetros, aquí».
    let r = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verbo.as_ptr(),
            archivo.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    // Por compatibilidad con Windows de 16 bits, devuelve un número y no un
    // booleano: por encima de 32 es que fue bien.
    if r as usize > 32 {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "Windows no supo abrirlo (código {})",
            r as usize
        )))
    }
}

/// ¿Está instalada? Se pregunta una vez por programa y se recuerda.
///
/// Sin la caché, importar diez mil vídeos lanzaría diez mil procesos solo para
/// averiguar diez mil veces lo mismo.
pub fn hay(programa: &str) -> bool {
    static CACHE: OnceLock<Mutex<HashMap<String, bool>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(c) = cache.lock() {
        if let Some(v) = c.get(programa) {
            return *v;
        }
    }
    let esta = orden(programa)
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        // Algunas herramientas no entienden `-version` pero existen: si el
        // fallo es «no encontrado», es que no está; cualquier otro, sí está.
        || orden(programa)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
            .map(|mut h| {
                let _ = h.kill();
                let _ = h.wait();
                true
            })
            .unwrap_or(false);
    if let Ok(mut c) = cache.lock() {
        c.insert(programa.to_string(), esta);
    }
    esta
}

/// Cada cuánto se mira si el proceso ya terminó.
///
/// Diez milisegundos: lo bastante corto para que un `pdftoppm` de treinta
/// milisegundos no pague una espera perceptible, y lo bastante largo para que
/// esperar treinta segundos a un vídeo no sean tres mil comprobaciones.
const LATIDO: Duration = Duration::from_millis(10);

/// Lanza el programa y espera, con tope. Devuelve lo que dijo por la salida de
/// error si terminó mal.
pub fn correr(programa: &str, args: &[&str], tope: Duration) -> Result<(), Fallo> {
    correr_con_salida(programa, args, tope, None)
}

/// Igual, pero guardando lo que el programa escriba por su salida normal.
///
/// Lo usan las herramientas que **informan** en vez de producir un archivo
/// (`ffprobe`, `pdfinfo`). Sigue siendo un archivo y no una tubería, por la
/// misma razón de siempre.
pub fn correr_leyendo(programa: &str, args: &[&str], tope: Duration) -> Result<String, Fallo> {
    let salida = Temporal::nuevo("dijo").map_err(|e| Fallo::Salio(e.to_string()))?;
    correr_con_salida(programa, args, tope, Some(&salida.ruta))?;
    Ok(std::fs::read_to_string(&salida.ruta).unwrap_or_default())
}

fn correr_con_salida(
    programa: &str,
    args: &[&str],
    tope: Duration,
    stdout: Option<&Path>,
) -> Result<(), Fallo> {
    // Con ruta absoluta basta con mirar que el archivo existe. Preguntar
    // lanzándolo con `-version` no vale para todos: Blender no entiende esa
    // opción y abre su ventana entera, una por cada pregunta.
    let por_ruta = Path::new(programa).is_absolute();
    if (por_ruta && !Path::new(programa).is_file()) || (!por_ruta && !hay(programa)) {
        return Err(Fallo::NoEsta);
    }
    let salida = Temporal::nuevo("queja").map_err(|e| Fallo::Salio(e.to_string()))?;
    let destino = match stdout {
        Some(p) => std::fs::File::create(p)
            .map(Stdio::from)
            .unwrap_or_else(|_| Stdio::null()),
        None => Stdio::null(),
    };
    let mut hijo = orden(programa)
        .args(args)
        .stdin(Stdio::null())
        .stdout(destino)
        .stderr(
            std::fs::File::create(&salida.ruta)
                .map(Stdio::from)
                .unwrap_or_else(|_| Stdio::null()),
        )
        .spawn()
        .map_err(|_| Fallo::NoEsta)?;

    let hasta = Instant::now() + tope;
    loop {
        match hijo.try_wait() {
            Ok(Some(estado)) => {
                if estado.success() {
                    return Ok(());
                }
                let queja = std::fs::read_to_string(&salida.ruta).unwrap_or_default();
                return Err(Fallo::Salio(recortar(&queja)));
            }
            Ok(None) => {
                if Instant::now() > hasta {
                    // Matarlo y esperarlo: sin el `wait` queda un zombi por cada
                    // archivo que se pase de tiempo.
                    let _ = hijo.kill();
                    let _ = hijo.wait();
                    return Err(Fallo::Tardo);
                }
                std::thread::sleep(LATIDO);
            }
            Err(e) => return Err(Fallo::Salio(e.to_string())),
        }
    }
}

/// Un archivo temporal que se borra solo.
///
/// Son quince líneas y evitan meter una dependencia más en el binario que se
/// distribuye. El nombre lleva el pid y un contador, y se crea con
/// `create_new`, que falla si ya existe: dos importaciones a la vez no pueden
/// pisarse el archivo.
pub struct Temporal {
    pub ruta: PathBuf,
}

impl Temporal {
    pub fn nuevo(para: &str) -> std::io::Result<Temporal> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static CUENTA: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir();
        for _ in 0..64 {
            let n = CUENTA.fetch_add(1, Ordering::Relaxed);
            let ruta = dir.join(format!(
                "grimorio-{}-{}-{}-{para}",
                std::process::id(),
                crate::time::now_ms(),
                n
            ));
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&ruta)
            {
                Ok(_) => return Ok(Temporal { ruta }),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }
        Err(std::io::Error::other("no pude crear un archivo temporal"))
    }

    /// La misma ruta pero con otra extensión, para las herramientas que la
    /// añaden ellas (`pdftoppm` escribe `salida.jpg` si le dices `salida`).
    pub fn con_extension(&self, ext: &str) -> PathBuf {
        let mut p = self.ruta.clone().into_os_string();
        p.push(".");
        p.push(ext);
        PathBuf::from(p)
    }
}

impl Drop for Temporal {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.ruta);
        // Las herramientas que añaden extensión dejan un archivo hermano.
        for ext in ["jpg", "png", "ppm"] {
            let _ = std::fs::remove_file(self.con_extension(ext));
        }
    }
}

/// La queja de una herramienta puede ser de miles de líneas. En el informe
/// cabe la última, que suele ser la que dice qué pasó.
fn recortar(s: &str) -> String {
    let ultima = s
        .lines()
        .filter(|l| !l.trim().is_empty())
        .next_back()
        .unwrap_or("");
    let ultima = ultima.trim();
    if ultima.chars().count() > 200 {
        ultima.chars().take(200).collect::<String>() + "…"
    } else {
        ultima.to_string()
    }
}

/// Igual que `correr`, pero además comprueba que el archivo que tenía que
/// aparecer está y no está vacío. Una herramienta que devuelve cero sin
/// escribir nada es un fallo aunque ella crea que no.
pub fn correr_y_leer(
    programa: &str,
    args: &[&str],
    tope: Duration,
    esperado: &Path,
) -> Result<Vec<u8>, Fallo> {
    correr(programa, args, tope)?;
    match std::fs::read(esperado) {
        Ok(b) if !b.is_empty() => Ok(b),
        Ok(_) => Err(Fallo::Salio("no escribió nada".into())),
        Err(e) => Err(Fallo::Salio(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Blender abría su ventana entera cada vez que se preguntaba si estaba,
    /// porque la pregunta era lanzarlo con `-version`. Con ruta absoluta no se
    /// pregunta: se lanza una vez, con lo que se pidió.
    #[test]
    #[cfg(unix)]
    fn con_ruta_absoluta_se_lanza_una_vez_y_sin_preguntar() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let apunte = dir.path().join("llamadas");
        let prog = dir.path().join("programa");
        std::fs::write(
            &prog,
            format!("#!/bin/sh\necho \"$@\" >> '{}'\n", apunte.display()),
        )
        .unwrap();
        std::fs::set_permissions(&prog, std::fs::Permissions::from_mode(0o755)).unwrap();
        // Recién escrito, lanzarlo puede dar «Text file busy»: otra prueba que
        // hace `fork` en paralelo hereda un instante el descriptor con el que
        // se escribió. Es del sistema, no de `correr`; se reintenta un poco.
        let mut r = Err(Fallo::NoEsta);
        for _ in 0..40 {
            r = correr(&prog.to_string_lossy(), &["-b", "hola"], Duration::from_secs(5));
            if r.is_ok() {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        r.unwrap();
        assert_eq!(std::fs::read_to_string(&apunte).unwrap(), "-b hola\n");
    }

    #[test]
    fn un_programa_que_no_existe_se_nota_y_no_cuelga() {
        let r = correr(
            "grimorio-programa-que-no-existe",
            &[],
            Duration::from_secs(1),
        );
        assert!(matches!(r, Err(Fallo::NoEsta)));
    }

    #[test]
    fn se_recuerda_si_esta_o_no() {
        // Dos veces seguidas para pasar por la caché; lo que importa es que la
        // respuesta no cambie sola.
        let a = hay("grimorio-programa-que-no-existe");
        let b = hay("grimorio-programa-que-no-existe");
        assert_eq!(a, b);
        assert!(!a);
    }

    #[test]
    fn el_que_tarda_demasiado_se_para() {
        if !hay("sleep") {
            return;
        }
        let t = Instant::now();
        let r = correr("sleep", &["30"], Duration::from_millis(120));
        assert!(matches!(r, Err(Fallo::Tardo)), "salió {r:?}");
        assert!(t.elapsed() < Duration::from_secs(3), "y se para pronto");
    }

    #[test]
    fn el_que_termina_mal_cuenta_por_que() {
        if !hay("sh") {
            return;
        }
        let r = correr(
            "sh",
            &["-c", "echo se rompió esto >&2; exit 3"],
            Duration::from_secs(5),
        );
        match r {
            Err(Fallo::Salio(q)) => assert!(q.contains("se rompió"), "salió «{q}»"),
            otro => panic!("salió {otro:?}"),
        }
    }

    #[test]
    fn si_no_escribe_el_archivo_es_un_fallo_aunque_diga_que_no() {
        if !hay("sh") {
            return;
        }
        let tmp = Temporal::nuevo("prueba").unwrap();
        let destino = tmp.con_extension("jpg");
        let r = correr_y_leer("sh", &["-c", "exit 0"], Duration::from_secs(5), &destino);
        assert!(matches!(r, Err(Fallo::Salio(_))));
    }

    #[test]
    fn se_puede_leer_lo_que_dice_un_programa() {
        if !hay("sh") {
            return;
        }
        let d = correr_leyendo(
            "sh",
            &["-c", "echo 1920; echo 1080"],
            Duration::from_secs(5),
        )
        .unwrap();
        assert_eq!(d.lines().collect::<Vec<_>>(), vec!["1920", "1080"]);
    }

    #[test]
    fn la_queja_se_recorta_a_lo_ultimo_que_dijo() {
        let largo = "aviso\n".repeat(500) + "esto es lo que pasó";
        assert_eq!(recortar(&largo), "esto es lo que pasó");
        assert!(recortar(&"x".repeat(500)).chars().count() <= 201);
    }
}

#[cfg(test)]
mod tests_temporal {
    use super::*;

    #[test]
    fn el_temporal_se_borra_solo() {
        let ruta = {
            let t = Temporal::nuevo("prueba").unwrap();
            assert!(t.ruta.exists());
            t.ruta.clone()
        };
        assert!(!ruta.exists(), "al soltarlo se va");
    }

    #[test]
    fn dos_temporales_seguidos_no_se_pisan() {
        let a = Temporal::nuevo("x").unwrap();
        let b = Temporal::nuevo("x").unwrap();
        assert_ne!(a.ruta, b.ruta);
    }

    #[test]
    fn tambien_se_lleva_el_hermano_con_extension() {
        let hermano = {
            let t = Temporal::nuevo("x").unwrap();
            let h = t.con_extension("jpg");
            std::fs::write(&h, b"algo").unwrap();
            h
        };
        assert!(!hermano.exists());
    }
}
