//! El `item.json`: la verdad sobre un elemento, legible sin la aplicación.

use crate::error::{Error, Result};
use crate::{time, SCHEMA_VERSION};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OriginMode {
    /// El original vive dentro de la biblioteca (por defecto, como Eagle).
    Copy,
    /// El original se movió dentro de la biblioteca.
    Move,
    /// El original se queda donde estaba; guardamos rastro para reencontrarlo.
    Ref,
}

impl OriginMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            OriginMode::Copy => "copy",
            OriginMode::Move => "move",
            OriginMode::Ref => "ref",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "copy" => Some(OriginMode::Copy),
            "move" => Some(OriginMode::Move),
            "ref" => Some(OriginMode::Ref),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Origin {
    pub mode: OriginMode,
    /// Ruta original. Obligatoria en modo `ref`, informativa en los demás.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Inodo en el momento de importar: permite reencontrar un `ref` movido.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inode: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PaletteEntry {
    pub rgb: [u8; 3],
    /// Peso: fracción de píxeles que caen en este grupo.
    pub w: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub schema: u32,
    pub name: String,
    pub ext: String,
    /// De qué familia es. Se guarda en el `item.json` y no se deduce de la
    /// extensión cada vez: el día que una extensión cambie de familia, lo ya
    /// importado tiene que seguir diciendo lo que dijo.
    #[serde(default)]
    pub kind: crate::medio::Familia,
    pub size: u64,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    /// Duración en milisegundos, si el archivo dura: vídeo y audio.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Páginas, si el archivo las tiene.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages: Option<u32>,
    /// Triángulos, si es un modelo 3D.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triangles: Option<u64>,
    /// Ancho, fondo y alto en milímetros, si el modelo dice sus unidades.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_mm: Option<[f32; 3]>,
    /// blake3 del contenido, en hex. Identidad real del archivo.
    pub hash: String,
    /// dHash perceptual de 64 bits, en hex. Vacío si no es imagen.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub phash: String,
    pub imported_at: String,
    pub modified_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub folders: Vec<String>,
    /// El orden a mano, por sitio: la clave es la carpeta ("" es «Todo») y el
    /// valor, una posición. Lo que no tiene posición en un sitio va donde le
    /// toca por fecha de importación (ver `Index::search` con orden manual).
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub orden: std::collections::BTreeMap<String, f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default)]
    pub stars: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub palette: Vec<PaletteEntry>,
    pub origin: Origin,
    #[serde(default)]
    pub trashed: bool,
    /// Marcado como contenido adulto.
    ///
    /// Es una marca, no un permiso: no esconde nada por sí sola. Lo que la mira
    /// es el modo seguro, que se enciende y se apaga desde la ventana, y que no
    /// vive aquí porque es de quien está delante y no del elemento.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub adult: bool,
}

impl Item {
    pub fn new(id: String, name: String, ext: String, size: u64, mode: OriginMode) -> Self {
        let now = time::to_rfc3339(time::now_ms());
        Item {
            id,
            schema: SCHEMA_VERSION,
            name,
            kind: crate::medio::familia_de(&ext).unwrap_or_default(),
            ext,
            size,
            width: 0,
            height: 0,
            duration_ms: None,
            pages: None,
            triangles: None,
            size_mm: None,
            hash: String::new(),
            phash: String::new(),
            imported_at: now.clone(),
            modified_at: now,
            source: None,
            folders: Vec::new(),
            orden: std::collections::BTreeMap::new(),
            tags: Vec::new(),
            stars: 0,
            note: None,
            palette: Vec::new(),
            origin: Origin {
                mode,
                path: None,
                inode: None,
            },
            trashed: false,
            adult: false,
        }
    }

    /// Texto que alimenta el índice de búsqueda.
    pub fn search_text(&self) -> String {
        let mut s = String::with_capacity(self.name.len() + 32);
        s.push_str(&self.name);
        for t in &self.tags {
            s.push(' ');
            s.push_str(t);
        }
        if let Some(n) = &self.note {
            s.push(' ');
            s.push_str(n);
        }
        if let Some(u) = &self.source {
            s.push(' ');
            s.push_str(u);
        }
        s.push(' ');
        s.push_str(&self.ext);
        s
    }

    pub fn imported_at_ms(&self) -> u64 {
        time::from_rfc3339(&self.imported_at).unwrap_or(0)
    }

    pub fn modified_at_ms(&self) -> u64 {
        time::from_rfc3339(&self.modified_at).unwrap_or(0)
    }

    pub fn touch(&mut self) {
        self.modified_at = time::to_rfc3339(time::now_ms());
    }

    pub fn read(path: &Path) -> Result<Item> {
        let bytes = std::fs::read(path).map_err(|e| Error::io(path, e))?;
        let item: Item = serde_json::from_slice(&bytes).map_err(|e| Error::json(path, e))?;
        if item.schema > SCHEMA_VERSION {
            return Err(Error::SchemaTooNew {
                found: item.schema,
                supported: SCHEMA_VERSION,
            });
        }
        // El id es lo que decide en qué carpeta se escribe y qué se borra. Uno
        // con forma de ruta se rechaza aquí, en la puerta, y no en cada sitio
        // que lo use: ver `id::es_valido`.
        if !crate::id::es_valido(&item.id) {
            return Err(Error::Invalid(format!(
                "{}: el id «{}» no es un identificador de Grimorio",
                path.display(),
                item.id
            )));
        }
        Ok(item)
    }

    /// Escritura atómica: temporal en el mismo directorio + `rename`.
    /// Un corte de luz deja el archivo viejo intacto, nunca uno a medias.
    pub fn write(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_vec_pretty(self).map_err(|e| Error::json(path, e))?;
        write_atomic(path, &json)
    }
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| Error::Invalid(format!("{} no tiene directorio padre", path.display())))?;
    std::fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, bytes).map_err(|e| Error::io(&tmp, e))?;
    renombrar_con_paciencia(&tmp, path).map_err(|e| Error::io(path, e))?;
    Ok(())
}

/// `rename` encima de un archivo que existe. En Unix es atómico y no falla
/// aunque otro lo tenga abierto. En Windows sí falla («acceso denegado») si en
/// ese instante alguien lo está leyendo: el indexador de búsqueda, el
/// antivirus o la propia aplicación cargando la ficha. Esas lecturas duran
/// milisegundos, así que se reintenta unas pocas veces esperando cada vez un
/// poco más (en total, menos de un segundo) antes de rendirse.
fn renombrar_con_paciencia(de: &Path, a: &Path) -> std::io::Result<()> {
    let mut espera = std::time::Duration::from_millis(5);
    let mut intentos = 0;
    loop {
        match std::fs::rename(de, a) {
            Ok(()) => return Ok(()),
            // Acceso denegado (5) o archivo en uso por otro proceso (32).
            Err(e) if cfg!(windows) && intentos < 7 && matches!(e.raw_os_error(), Some(5 | 32)) => {
                intentos += 1;
                std::thread::sleep(espera);
                espera *= 2;
            }
            Err(e) => return Err(e),
        }
    }
}

/// `items/01/JK/01JKX7…/` — dos niveles de sharding para que ningún directorio
/// llegue a tener cien mil entradas.
pub fn shard_dir(items_root: &Path, id: &str) -> PathBuf {
    // `get` en vez de indexar: un id corto o con bytes raros (datos corruptos,
    // una prueba descuidada) no debe tumbar el proceso con un pánico.
    let a = id.get(0..2).unwrap_or("__");
    let b = id.get(2..4).unwrap_or("__");
    items_root.join(a).join(b).join(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ida_y_vuelta_json() {
        let mut it = Item::new(
            "01JKX7Q2M8V3N4P5R6S7T8XYZ0".into(),
            "prueba".into(),
            "jpg".into(),
            1234,
            OriginMode::Copy,
        );
        it.tags = vec!["nocturno".into(), "referencia".into()];
        it.stars = 4;
        let json = serde_json::to_string(&it).unwrap();
        let back: Item = serde_json::from_str(&json).unwrap();
        assert_eq!(back.tags, it.tags);
        assert_eq!(back.stars, 4);
        assert_eq!(back.origin.mode, OriginMode::Copy);
    }

    #[test]
    fn el_texto_de_busqueda_incluye_tags_y_nota() {
        let mut it = Item::new(
            "x".repeat(26),
            "gato azul".into(),
            "png".into(),
            1,
            OriginMode::Copy,
        );
        it.tags = vec!["felino".into()];
        it.note = Some("para el moodboard".into());
        let t = it.search_text();
        assert!(t.contains("gato azul"));
        assert!(t.contains("felino"));
        assert!(t.contains("moodboard"));
    }

    #[test]
    fn sharding_de_dos_niveles() {
        let p = shard_dir(Path::new("/lib/items"), "01JKX7Q2M8V3N4P5R6S7T8XYZ0");
        assert!(p.ends_with("01/JK/01JKX7Q2M8V3N4P5R6S7T8XYZ0"));
    }

    #[test]
    fn un_id_corto_no_provoca_panico() {
        let p = shard_dir(Path::new("/lib/items"), "ab");
        assert!(p.ends_with("ab/__/ab"));
        assert!(shard_dir(Path::new("/lib/items"), "").ends_with("__/__"));
    }
}
