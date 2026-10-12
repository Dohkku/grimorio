//! `grim` como programa nativo del navegador (native messaging).
//!
//! La extensión de Grimorio no puede abrir archivos ni hablar con otros
//! programas: el navegador no la deja. Lo que sí puede es hablar con un
//! programa registrado para ella, que el navegador lanza y con el que se
//! entiende por la entrada y la salida estándar. Ese programa es `grim`.
//!
//! El navegador lo arranca sin subórdenes: Chrome y sus derivados le pasan el
//! origen de la extensión (`chrome-extension://…/`) y Firefox, la ruta del
//! manifiesto y el id de la extensión. Por eso se reconoce mirando eso, antes
//! de leer la línea de órdenes de siempre.
//!
//! El protocolo del navegador: cada mensaje es su longitud (u32, en el orden
//! de bytes de la máquina) seguida de un JSON. `grim` reenvía cada mensaje,
//! tal cual, a la puerta de la aplicación abierta (ver app/src/puerta.h) y
//! devuelve su respuesta. Si la aplicación no está abierta, la abre y espera a
//! que la puerta conteste.

use serde_json::{json, Value};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// El nombre con el que se registra en los navegadores.
pub const NOMBRE: &str = "com.grimorio.puerta";
/// El id fijo de la extensión de Chrome y sus derivados. Sale de la clave
/// pública que lleva su manifiesto (`navegador/manifest.json`).
pub const ID_CHROME: &str = "nhmdhkmjpnpclpiopkajinahdcmacfko";
/// El id de la extensión en Firefox (`browser_specific_settings`).
pub const ID_FIREFOX: &str = "puerta@grimorio";

/// Lo más que se lee de un mensaje del navegador. Chrome no manda más de 64 MB.
const TOPE_MENSAJE: usize = 64 * 1024 * 1024;
/// Cuánto se espera a que la aplicación recién lanzada abra la puerta.
const ESPERA_ARRANQUE: Duration = Duration::from_secs(25);

/// Si a `grim` lo ha lanzado un navegador, y no una persona.
pub fn es_del_navegador(args: &[String]) -> bool {
    match args.get(1) {
        Some(a) if a.starts_with("chrome-extension://") => true,
        // Firefox: la ruta de su manifiesto y el id de la extensión.
        Some(a) if a.ends_with(".json") => args.get(2).is_some_and(|id| id == ID_FIREFOX),
        _ => false,
    }
}

/// Lee un mensaje del navegador. `None` es que el navegador cerró.
pub fn leer(entrada: &mut impl Read) -> io::Result<Option<Vec<u8>>> {
    let mut largo = [0u8; 4];
    match entrada.read_exact(&mut largo) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let n = u32::from_ne_bytes(largo) as usize;
    if n > TOPE_MENSAJE {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "mensaje demasiado grande"));
    }
    let mut datos = vec![0u8; n];
    entrada.read_exact(&mut datos)?;
    Ok(Some(datos))
}

pub fn escribir(salida: &mut impl Write, datos: &[u8]) -> io::Result<()> {
    salida.write_all(&(datos.len() as u32).to_ne_bytes())?;
    salida.write_all(datos)?;
    salida.flush()
}

/// El bucle entero: mientras el navegador mande, se reenvía y se contesta.
pub fn correr() -> io::Result<()> {
    let mut entrada = io::stdin().lock();
    let mut salida = io::stdout().lock();
    while let Some(mensaje) = leer(&mut entrada)? {
        let respuesta = match serde_json::from_slice::<Value>(&mensaje) {
            Ok(v) => atender(&v),
            Err(e) => json!({ "ok": false, "error": format!("no es JSON: {e}") }),
        };
        escribir(&mut salida, respuesta.to_string().as_bytes())?;
    }
    Ok(())
}

fn atender(peticion: &Value) -> Value {
    let linea = peticion.to_string();
    match preguntar(&linea) {
        Ok(r) => r,
        Err(_) => {
            // Cerrada: se abre y se espera a que conteste, pero solo para
            // guardar algo. Preguntar si está, o por sus carpetas al abrir la
            // ventanita de la extensión, no puede ir abriendo la aplicación.
            if peticion["accion"] != "traer" {
                return json!({ "ok": false, "cerrada": true, "error": "Grimorio no está abierto" });
            }
            if let Err(e) = abrir_aplicacion() {
                return json!({ "ok": false, "error": format!("Grimorio no está abierto y no pude abrirlo: {e}") });
            }
            let hasta = Instant::now() + ESPERA_ARRANQUE;
            loop {
                std::thread::sleep(Duration::from_millis(300));
                match preguntar(&linea) {
                    Ok(r) => return r,
                    Err(_) if Instant::now() < hasta => continue,
                    Err(_) => {
                        return json!({ "ok": false, "error": "Grimorio tardó demasiado en abrir" })
                    }
                }
            }
        }
    }
}

/// Una petición a la puerta y su respuesta.
fn preguntar(linea: &str) -> io::Result<Value> {
    let mut canal = conectar()?;
    canal.write_all(linea.as_bytes())?;
    canal.write_all(b"\n")?;
    canal.flush()?;
    let mut respuesta = String::new();
    BufReader::new(canal).read_line(&mut respuesta)?;
    serde_json::from_str(&respuesta).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Dónde escucha la puerta. Lo mismo que `Puerta::nombre` en la aplicación.
pub fn ruta_puerta() -> PathBuf {
    #[cfg(windows)]
    {
        let usuario = std::env::var("USERNAME").unwrap_or_default();
        PathBuf::from(format!(r"\\.\pipe\grimorio-puerta-{usuario}"))
    }
    #[cfg(not(windows))]
    {
        match std::env::var_os("XDG_RUNTIME_DIR") {
            Some(d) if !d.is_empty() => Path::new(&d).join("grimorio.sock"),
            _ => std::env::temp_dir().join(format!("grimorio-{}.sock", uid_de_casa())),
        }
    }
}

#[cfg(not(windows))]
fn uid_de_casa() -> u32 {
    // Sin `libc` en la CLI: el dueño del directorio personal es el usuario.
    use std::os::unix::fs::MetadataExt;
    std::env::var_os("HOME")
        .and_then(|h| std::fs::metadata(h).ok())
        .map(|m| m.uid())
        .unwrap_or(0)
}

#[cfg(not(windows))]
fn conectar() -> io::Result<std::os::unix::net::UnixStream> {
    let s = std::os::unix::net::UnixStream::connect(ruta_puerta())?;
    s.set_read_timeout(Some(Duration::from_secs(60)))?;
    Ok(s)
}

#[cfg(windows)]
fn conectar() -> io::Result<std::fs::File> {
    std::fs::OpenOptions::new().read(true).write(true).open(ruta_puerta())
}

/// Lanza la aplicación, que está junto a `grim`, y la deja ir: en su propio
/// grupo de procesos, para que cuando el navegador cierre a `grim` no se la
/// lleve por delante.
fn abrir_aplicacion() -> io::Result<()> {
    let nombre = if cfg!(windows) { "grimorio.exe" } else { "grimorio" };
    let junto = std::env::current_exe()?.with_file_name(nombre);
    let programa = if junto.exists() { junto } else { PathBuf::from(nombre) };
    let mut orden = grimorio_core::externo::orden(&programa);
    orden
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        orden.process_group(0);
    }
    orden.spawn().map(|_| ())
}

// ------------------------------------------------------------- instalación

/// Dónde guarda cada navegador los programas nativos, en este sistema. Solo
/// los que están instalados: crear la carpeta de un navegador que no está
/// sería ensuciar.
fn sitios() -> Vec<(&'static str, PathBuf, bool)> {
    let mut v = Vec::new();
    #[cfg(target_os = "linux")]
    {
        let Some(casa) = std::env::var_os("HOME").map(PathBuf::from) else {
            return v;
        };
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| casa.join(".config"));
        for (nombre, dir) in [
            ("Brave", "BraveSoftware/Brave-Browser"),
            ("Chrome", "google-chrome"),
            ("Chromium", "chromium"),
            ("Edge", "microsoft-edge"),
            ("Vivaldi", "vivaldi"),
        ] {
            let base = config.join(dir);
            if base.is_dir() {
                v.push((nombre, base.join("NativeMessagingHosts"), false));
            }
        }
        if casa.join(".mozilla").is_dir() {
            v.push(("Firefox", casa.join(".mozilla/native-messaging-hosts"), true));
        }
    }
    v
}

fn manifiesto(firefox: bool) -> io::Result<Value> {
    let ruta = std::env::current_exe()?;
    let mut m = json!({
        "name": NOMBRE,
        "description": "Grimorio: guardar en la biblioteca desde el navegador",
        "path": ruta.to_string_lossy(),
        "type": "stdio",
    });
    if firefox {
        m["allowed_extensions"] = json!([ID_FIREFOX]);
    } else {
        m["allowed_origins"] = json!([format!("chrome-extension://{ID_CHROME}/")]);
    }
    Ok(m)
}

/// Registra `grim` en los navegadores que haya. Devuelve en cuáles.
pub fn instalar() -> io::Result<Vec<String>> {
    let mut hechos = Vec::new();
    for (nombre, dir, firefox) in sitios() {
        std::fs::create_dir_all(&dir)?;
        let texto = serde_json::to_string_pretty(&manifiesto(firefox)?)?;
        std::fs::write(dir.join(format!("{NOMBRE}.json")), texto)?;
        hechos.push(nombre.to_string());
    }
    Ok(hechos)
}

pub fn desinstalar() -> io::Result<Vec<String>> {
    let mut hechos = Vec::new();
    for (nombre, dir, _) in sitios() {
        let f = dir.join(format!("{NOMBRE}.json"));
        if f.exists() {
            std::fs::remove_file(f)?;
            hechos.push(nombre.to_string());
        }
    }
    Ok(hechos)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn reconoce_al_navegador_y_no_a_una_persona() {
        assert!(es_del_navegador(&args(&["grim", "chrome-extension://abc/"])));
        assert!(es_del_navegador(&args(&[
            "grim",
            "/home/x/.mozilla/native-messaging-hosts/com.grimorio.puerta.json",
            ID_FIREFOX
        ])));
        assert!(!es_del_navegador(&args(&["grim", "search", "gato"])));
        assert!(!es_del_navegador(&args(&["grim", "import", "fotos.json"])));
        assert!(!es_del_navegador(&args(&["grim"])));
    }

    #[test]
    fn el_marco_del_navegador_va_y_vuelve() {
        let mut buf = Vec::new();
        escribir(&mut buf, br#"{"accion":"hola"}"#).unwrap();
        escribir(&mut buf, b"{}").unwrap();
        let mut c = std::io::Cursor::new(buf);
        assert_eq!(leer(&mut c).unwrap().unwrap(), br#"{"accion":"hola"}"#);
        assert_eq!(leer(&mut c).unwrap().unwrap(), b"{}");
        assert!(leer(&mut c).unwrap().is_none(), "al acabar, el navegador cerró");
    }

    #[test]
    fn un_largo_absurdo_no_reserva_memoria() {
        let mut c = std::io::Cursor::new(u32::MAX.to_ne_bytes().to_vec());
        assert!(leer(&mut c).is_err());
    }

    #[test]
    fn el_manifiesto_dice_quien_puede_hablar() {
        let m = manifiesto(false).unwrap();
        assert_eq!(m["allowed_origins"][0], format!("chrome-extension://{ID_CHROME}/"));
        let f = manifiesto(true).unwrap();
        assert_eq!(f["allowed_extensions"][0], ID_FIREFOX);
        assert_eq!(m["type"], "stdio");
    }
}
