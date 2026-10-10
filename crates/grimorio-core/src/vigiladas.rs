//! Carpetas del disco que se importan solas.
//!
//! Lo que aparece en una carpeta vigilada —una descarga, una exportación de
//! otro programa— entra en la biblioteca sin pasar por «importar». Se guarda
//! en `vigiladas.json`, en la biblioteca, con el momento de la última pasada:
//! al volver a abrir solo entra lo nuevo desde entonces, y lo que ya estaba la
//! primera vez entra entero.
//!
//! Lo nuevo se reconoce por la fecha más reciente entre la de modificación y
//! la de cambio de estado (ctime): mover un archivo dentro de la carpeta no
//! toca su fecha de modificación —sigue siendo la del día que se hizo— pero sí
//! su ctime.
//!
//! Aquí solo está qué se vigila y qué hay de nuevo. Quién mira y cuándo está
//! en el puente (un hilo que pasa cada pocos segundos).

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const ARCHIVO: &str = "vigiladas.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Vigilada {
    pub id: String,
    /// La carpeta del disco.
    pub ruta: String,
    /// La carpeta de la biblioteca a la que entra lo nuevo, si alguna.
    #[serde(default)]
    pub carpeta: Option<String>,
    /// Lo que cambió hasta este momento (ms Unix) ya se miró.
    #[serde(default)]
    pub desde_ms: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Archivo {
    schema: u32,
    vigiladas: Vec<Vigilada>,
}

pub fn cargar(raiz: &Path) -> Result<Vec<Vigilada>> {
    let ruta = raiz.join(ARCHIVO);
    if !ruta.exists() {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(&ruta).map_err(|e| Error::io(&ruta, e))?;
    let a: Archivo = serde_json::from_slice(&bytes).map_err(|e| Error::json(&ruta, e))?;
    Ok(a.vigiladas)
}

fn guardar(raiz: &Path, v: &[Vigilada]) -> Result<()> {
    let ruta = raiz.join(ARCHIVO);
    let a = Archivo {
        schema: 1,
        vigiladas: v.to_vec(),
    };
    let json = serde_json::to_vec_pretty(&a).map_err(|e| Error::json(&ruta, e))?;
    crate::item::write_atomic(&ruta, &json)
}

/// Empieza a vigilar `ruta`. Lo que ya hay dentro entra en la primera pasada.
pub fn anadir(raiz: &Path, ruta: &str, carpeta: Option<&str>) -> Result<String> {
    let p = Path::new(ruta);
    if !p.is_dir() {
        return Err(Error::Invalid(format!("«{ruta}» no es una carpeta")));
    }
    // Vigilar la propia biblioteca metería cada importación de vuelta en sí
    // misma, para siempre.
    if let (Ok(a), Ok(b)) = (p.canonicalize(), raiz.canonicalize()) {
        if a.starts_with(&b) || b.starts_with(&a) {
            return Err(Error::Invalid(
                "no se puede vigilar la biblioteca ni una carpeta que la contenga".into(),
            ));
        }
    }
    let mut todas = cargar(raiz)?;
    if todas.iter().any(|v| v.ruta == ruta && v.carpeta.as_deref() == carpeta) {
        return Err(Error::Invalid(format!("«{ruta}» ya está vinculada")));
    }
    let id = crate::id::new_id();
    todas.push(Vigilada {
        id: id.clone(),
        ruta: ruta.to_string(),
        carpeta: carpeta.map(str::to_string),
        desde_ms: 0,
    });
    guardar(raiz, &todas)?;
    Ok(id)
}

pub fn quitar(raiz: &Path, id: &str) -> Result<()> {
    let mut todas = cargar(raiz)?;
    let antes = todas.len();
    todas.retain(|v| v.id != id);
    if todas.len() == antes {
        return Err(Error::Invalid(format!("no hay ninguna carpeta vinculada con id {id}")));
    }
    guardar(raiz, &todas)
}

/// Apunta que hasta `hasta_ms` ya está mirado.
pub fn marcar(raiz: &Path, id: &str, hasta_ms: u64) -> Result<()> {
    let mut todas = cargar(raiz)?;
    if let Some(v) = todas.iter_mut().find(|v| v.id == id) {
        v.desde_ms = v.desde_ms.max(hasta_ms);
        guardar(raiz, &todas)?;
    }
    Ok(())
}

/// Cuándo cambió por última vez, en ms: lo más reciente entre modificación y
/// cambio de estado.
#[cfg(unix)]
fn cuando(m: &std::fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    let mtime = (m.mtime() as u64) * 1000 + (m.mtime_nsec() as u64) / 1_000_000;
    let ctime = (m.ctime() as u64) * 1000 + (m.ctime_nsec() as u64) / 1_000_000;
    mtime.max(ctime)
}

/// Windows no tiene «cambio de estado». Lo que más se le parece es la fecha de
/// creación: un archivo copiado o movido a la carpeta conserva su fecha de
/// modificación antigua, pero estrena la de creación. Así que se toma la más
/// reciente de las dos, que es lo que hace el `ctime` en Linux en ese caso.
#[cfg(not(unix))]
fn cuando(m: &std::fs::Metadata) -> u64 {
    let ms = |t: std::io::Result<std::time::SystemTime>| {
        t.ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    };
    ms(m.modified()).max(ms(m.created()))
}

/// Oculto a la manera de cada sistema. En todos, el punto delante: `.part`,
/// `.DS_Store`, las carpetas de configuración. En Windows además el atributo
/// de oculto, que es como marca él `desktop.ini`, `Thumbs.db` y compañía.
fn es_oculto(nombre: &std::ffi::OsStr, _m: &std::fs::Metadata) -> bool {
    if nombre.to_string_lossy().starts_with('.') {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        if _m.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0 {
            return true;
        }
    }
    false
}

/// Lo que hay en la carpeta que se puede importar y cambió después de
/// `desde_ms`, con su tamaño (para saber si se sigue escribiendo).
pub fn nuevos(v: &Vigilada) -> Vec<(PathBuf, u64)> {
    let mut out = Vec::new();
    recorrer(Path::new(&v.ruta), v.desde_ms, &mut out, 0);
    out
}

fn recorrer(dir: &Path, desde: u64, out: &mut Vec<(PathBuf, u64)>, hondo: u32) {
    // Un tope de profundidad, por si alguien vigila algo con enlaces en bucle.
    if hondo > 12 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        let nombre = e.file_name();
        let Ok(m) = std::fs::metadata(&p) else {
            continue;
        };
        // Ocultos fuera: `.part`, `.DS_Store`, las carpetas de configuración.
        if es_oculto(&nombre, &m) {
            continue;
        }
        if m.is_dir() {
            recorrer(&p, desde, out, hondo + 1);
        } else if m.is_file() {
            let ext = p.extension().and_then(|x| x.to_str()).unwrap_or("");
            if crate::medio::se_puede_guardar(ext) && cuando(&m) > desde {
                out.push((p, m.len()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primero_entra_todo_y_luego_solo_lo_nuevo() {
        let lib = tempfile::tempdir().unwrap();
        let disco = tempfile::tempdir().unwrap();
        std::fs::write(disco.path().join("a.png"), b"x").unwrap();
        std::fs::write(disco.path().join("notas.txt"), b"x").unwrap();
        std::fs::write(disco.path().join(".oculto.png"), b"x").unwrap();
        std::fs::create_dir(disco.path().join("sub")).unwrap();
        std::fs::write(disco.path().join("sub/b.jpg"), b"x").unwrap();

        let ruta = disco.path().to_str().unwrap();
        let id = anadir(lib.path(), ruta, Some("carpeta1")).unwrap();
        let v = cargar(lib.path()).unwrap().remove(0);
        let mut n: Vec<String> = nuevos(&v)
            .into_iter()
            .map(|(p, _)| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        n.sort();
        assert_eq!(n, vec!["a.png", "b.jpg"], "ni el txt ni el oculto");

        marcar(lib.path(), &id, crate::time::now_ms() + 1000).unwrap();
        let v = cargar(lib.path()).unwrap().remove(0);
        assert!(nuevos(&v).is_empty(), "lo ya mirado no vuelve");

        assert!(anadir(lib.path(), ruta, Some("carpeta1")).is_err(), "dos veces no");
        quitar(lib.path(), &id).unwrap();
        assert!(cargar(lib.path()).unwrap().is_empty());
    }

    #[test]
    fn no_se_vigila_la_propia_biblioteca() {
        let lib = tempfile::tempdir().unwrap();
        let ruta = lib.path().to_str().unwrap();
        assert!(anadir(lib.path(), ruta, None).is_err());
        assert!(anadir(lib.path(), "/no/existe/esto", None).is_err());
    }
}
