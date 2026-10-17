//! Etiquetas que probablemente le van a algo, sacadas de lo que ya hay.
//!
//! Nada de modelos ni de servicios: solo lo que la biblioteca ya sabe de sí
//! misma, que suele ser mucho. Cuatro pistas, cada una con su peso:
//!
//! - **el nombre**: `tacos_picantes.jpg` y ya existe `#picante`;
//! - **lo que se le parece**: las imágenes con la huella visual cercana
//!   comparten etiquetas más a menudo que no;
//! - **lo que va junto**: quien tiene `#ramen` suele tener `#comida`;
//! - **de dónde viene**: su carpeta y la web de la que se guardó.
//!
//! Y etiquetas nuevas, con más cuidado: palabras del nombre del archivo y la
//! carpeta de la que vino, sin códigos de cámara ni palabras vacías. Antes de
//! proponer una nueva se mira si ya hay una casi igual (sin tildes, en
//! plural): si la hay, se propone esa. Así no se llena la biblioteca de casi
//! repetidos (`#comida`, `#comidas`, `#Comida`).

use crate::error::Result;
use crate::library::Library;
use rusqlite::params;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

/// Por qué se propone, para decirlo al lado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Porque {
    Nombre,
    Parecidas,
    Juntas,
    Carpeta,
    Origen,
}

#[derive(Debug, Clone, Serialize)]
pub struct Sugerencia {
    pub etiqueta: String,
    pub puntos: f32,
    pub porque: Porque,
    /// Todavía no la tiene nadie: ponerla la crea.
    pub nueva: bool,
}

/// Palabras que salen en los nombres de archivo y no dicen nada de lo que
/// hay dentro: códigos de cámara y de programas, extensiones, relleno.
const VACIAS: &[&str] = &[
    "de", "del", "la", "las", "el", "los", "con", "sin", "para", "por", "una", "uno", "unos",
    "the", "and", "for", "with", "from", "this", "that", "copy", "copia", "final", "nuevo",
    "nueva", "new", "edit", "editado", "editada", "version", "img", "image", "imagen", "foto",
    "photo", "pic", "picture", "dsc", "dcim", "pxl", "screenshot", "screen", "shot", "captura",
    "pantalla", "whatsapp", "telegram", "signal", "video", "vid", "untitled", "titulo",
    "título", "file", "archivo", "download", "descarga", "scaled", "large", "small", "medium",
    "thumb", "thumbnail", "original", "export", "exportado", "render", "frame", "clip", "mov",
    "mp4", "jpg", "jpeg", "png", "webp", "gif", "heic", "raw", "tiff", "instagram", "pinterest",
    "tiktok", "twitter", "reddit", "facebook", "youtube", "lowres", "hires", "full", "size",
];

/// Carpetas que no dicen nada de lo que guardan.
const CARPETAS_VACIAS: &[&str] = &[
    "downloads", "descargas", "documents", "documentos", "desktop", "escritorio", "pictures",
    "imagenes", "imágenes", "images", "videos", "vídeos", "music", "musica", "música", "tmp",
    "temp", "nueva carpeta", "new folder", "misc", "varios", "otros", "otras", "fotos",
    "photos", "camera", "camara", "cámara", "dcim", "screenshots", "capturas",
    "capturas de pantalla", "whatsapp images", "home", "media", "mnt", "users", "usuarios",
    "export", "exports", "renders", "assets", "recursos", "files", "archivos", "content",
];

/// Si una palabra suelta del nombre vale como etiqueta nueva. Llega tal cual
/// estaba escrita: las mayúsculas mezcladas delatan un código.
fn vale_como_nueva(p: &str) -> bool {
    let mayusculas = p.chars().filter(|c| c.is_uppercase()).count();
    let minusculas = p.chars().filter(|c| c.is_lowercase()).count();
    p.chars().count() >= 4
        // `DDBOueegmw` (el código de una publicación) no; `Tacos` y `TACOS` sí.
        && !(mayusculas >= 2 && minusculas >= 1)
        && p.chars().all(|c| c.is_alphabetic())
        && !VACIAS.contains(&raiz(p).as_str())
        // Una ristra sin vocales es un código, no una palabra (`xkcdq`).
        && p.to_lowercase().chars().any(|c| "aeiouáéíóúü".contains(c))
}

/// Las carpetas de las que vino, las dos más cercanas, que dicen algo.
fn carpetas_de_origen(origen: &str) -> Vec<String> {
    let casa = std::env::var("HOME").unwrap_or_default();
    let usuario = std::path::Path::new(&casa)
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let ruta = std::path::Path::new(origen);
    // Lo que se importó desde una caché o un temporal (la de otra biblioteca,
    // la de un navegador) vive en carpetas que no son de nadie.
    let temporal = std::env::temp_dir();
    if ruta.starts_with(&temporal)
        || ruta.components().any(|c| {
            let c = c.as_os_str().to_string_lossy().to_lowercase();
            matches!(c.as_str(), "cache" | ".cache" | "tmp" | "temp" | "caches")
                || c.ends_with(".grimorio")
        })
    {
        return Vec::new();
    }
    ruta.ancestors()
        .skip(1)
        .take(2)
        .filter_map(|d| d.file_name().map(|n| n.to_string_lossy().to_lowercase()))
        .map(|n| n.replace(['_', '-'], " ").trim().to_string())
        .filter(|n| {
            n.chars().count() >= 3
                && n.chars().filter(|c| c.is_alphabetic()).count() >= 3
                && !n.chars().any(|c| c.is_ascii_digit())
                && !n.contains("grimorio")
                && !n.starts_with('.')
                && *n != usuario
                && !CARPETAS_VACIAS.contains(&n.as_str())
                && !CARPETAS_VACIAS.contains(&raiz(n).as_str())
        })
        .collect()
}

/// Cuántas imágenes parecidas se miran, y hasta qué distancia de huella (de
/// 64 bits) se consideran parecidas.
const VECINAS: usize = 12;
const DISTANCIA: u32 = 14;
/// Por debajo de esto no se propone: mejor ninguna que una al azar.
const MINIMO: f32 = 0.3;

/// Minúsculas y sin tildes: `Café` y `cafe` son la misma palabra para esto.
pub fn raiz(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            c => c,
        })
        .collect()
}

/// Las formas con las que una palabra puede casar con otra: tal cual, sin
/// la «s» y sin el «es» del plural. Adivinar cuál es el singular en español
/// falla (`picantes` → `picante`, `flores` → `flor`); probar las dos no.
fn formas(r: &str) -> Vec<String> {
    let mut v = vec![r.to_string()];
    if r.chars().count() > 3 && r.ends_with('s') {
        v.push(r[..r.len() - 1].to_string());
    }
    if r.chars().count() > 4 && r.ends_with("es") {
        v.push(r[..r.len() - 2].to_string());
    }
    v
}

/// Las palabras de un nombre de archivo: corta por todo lo que no es letra o
/// número, y por los cambios de minúscula a mayúscula (`tacoPicante`).
fn palabras(nombre: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut actual = String::new();
    let mut antes_minuscula = false;
    // Un trozo con letras y números pegados (`DSC04512`, `Dcam-v8A`,
    // `Dd4B0dTp3wG`) es un código entero: ni sus pedazos valen.
    let codigo = |trozo: &str| {
        trozo.chars().zip(trozo.chars().skip(1)).any(|(a, b)| {
            (a.is_alphabetic() && b.is_ascii_digit()) || (a.is_ascii_digit() && b.is_alphabetic())
        })
    };
    let limpio: String = nombre
        .split_whitespace()
        .filter(|t| !codigo(t))
        .collect::<Vec<_>>()
        .join(" ");
    for c in limpio.chars() {
        if !c.is_alphanumeric() {
            if !actual.is_empty() {
                out.push(std::mem::take(&mut actual));
            }
            antes_minuscula = false;
            continue;
        }
        if c.is_uppercase() && antes_minuscula && !actual.is_empty() {
            out.push(std::mem::take(&mut actual));
        }
        antes_minuscula = c.is_lowercase();
        actual.push(c);
    }
    if !actual.is_empty() {
        out.push(actual);
    }
    out.into_iter()
        .filter(|p| p.chars().count() >= 3 && !p.chars().all(|c| c.is_ascii_digit()))
        .collect()
}

/// El dominio de una URL de origen: `https://www.pinterest.com/pin/1` →
/// `pinterest.com`.
fn dominio(url: &str) -> Option<String> {
    let resto = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    let host = resto.split(['/', '?', '#']).next()?.trim_start_matches("www.");
    (!host.is_empty() && host.contains('.')).then(|| host.to_lowercase())
}

/// Sugerencias para uno o varios elementos, las mejores primero. Con varios,
/// valen las que encajan con el conjunto, y no se propone lo que ya tienen
/// todos.
pub fn sugerir(lib: &Library, ids: &[String], tope: usize) -> Result<Vec<Sugerencia>> {
    let conn = &lib.index().conn;
    // Todas las etiquetas, con cuántos elementos tiene cada una.
    let mut todas: HashMap<String, u64> = HashMap::new();
    {
        let mut st = conn.prepare(
            "SELECT t.name, COUNT(*) FROM item_tags it JOIN tags t ON t.id = it.tag_id
              JOIN items i ON i.id = it.item_id AND i.trashed = 0 GROUP BY t.id",
        )?;
        for r in st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64)))? {
            let (n, c) = r?;
            todas.insert(n, c);
        }
    }
    if todas.is_empty() || ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut por_forma: HashMap<String, String> = HashMap::new();
    for t in todas.keys() {
        for f in formas(&raiz(t)) {
            por_forma.entry(f).or_insert_with(|| t.clone());
        }
    }

    // Las huellas de todo, una vez para todos los elementos pedidos.
    let mut huellas: Vec<(String, u64)> = Vec::new();
    {
        let mut st =
            conn.prepare("SELECT id, phash FROM items WHERE trashed = 0 AND phash IS NOT NULL")?;
        for r in st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64)))? {
            huellas.push(r?);
        }
    }
    let etiquetas_de = |id: &str| -> Result<Vec<String>> {
        let mut st = conn.prepare_cached(
            "SELECT t.name FROM item_tags it JOIN tags t ON t.id = it.tag_id WHERE it.item_id = ?1",
        )?;
        let v = st
            .query_map(params![id], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(v)
    };

    let mut puntos: HashMap<String, (f32, Porque, f32)> = HashMap::new();
    // Las nuevas, por su forma sin tildes: la misma palabra en dos elementos
    // suma, aunque en uno lleve tilde y en otro no. Se enseña la primera.
    let mut nuevas: HashMap<String, (String, f32, Porque)> = HashMap::new();
    let casa_con_existente = |p: &str| formas(&raiz(p)).iter().find_map(|f| por_forma.get(f)).cloned();
    let mut tienen_todos: Option<HashSet<String>> = None;
    let sumar = |t: &str, p: f32, porque: Porque, puntos: &mut HashMap<String, (f32, Porque, f32)>| {
        let e = puntos.entry(t.to_string()).or_insert((0.0, porque, 0.0));
        e.0 += p;
        // El porqué que se dice es el de la pista que más aportó.
        if p > e.2 {
            e.1 = porque;
            e.2 = p;
        }
    };

    for id in ids {
        let Ok(it) = lib.load_item(id) else { continue };
        let propias: HashSet<String> = it.tags.iter().cloned().collect();
        tienen_todos = Some(match tienen_todos {
            None => propias.clone(),
            Some(t) => t.intersection(&propias).cloned().collect(),
        });

        // 1. El nombre.
        let mut vistas = HashSet::new();
        let mut nuevas_aqui: HashMap<String, (String, f32, Porque)> = HashMap::new();
        for p in palabras(&it.name) {
            if let Some(t) = casa_con_existente(&p) {
                if vistas.insert(t.clone()) {
                    sumar(&t, 0.9, Porque::Nombre, &mut puntos);
                }
            } else if vale_como_nueva(&p) {
                nuevas_aqui.entry(raiz(&p)).or_insert((p.to_lowercase(), 0.45, Porque::Nombre));
            }
        }
        // 5. La carpeta de la que vino, si todavía está en su ruta.
        if let Some(origen) = it.origin.path.as_deref() {
            for c in carpetas_de_origen(origen) {
                if let Some(t) = casa_con_existente(&c) {
                    if vistas.insert(t.clone()) {
                        sumar(&t, 0.6, Porque::Carpeta, &mut puntos);
                    }
                } else {
                    nuevas_aqui.entry(raiz(&c)).or_insert((c.clone(), 0.5, Porque::Carpeta));
                }
            }
        }
        for (clave, (texto, p, porque)) in nuevas_aqui {
            let e = nuevas.entry(clave).or_insert((texto, 0.0, porque));
            e.1 += p;
        }

        // 2. Lo que se le parece.
        if let Ok(h) = u64::from_str_radix(&it.phash, 16) {
            let mut cerca: Vec<(u32, &str)> = huellas
                .iter()
                .filter(|(otro, _)| otro != id)
                .map(|(otro, ph)| ((ph ^ h).count_ones(), otro.as_str()))
                .filter(|(d, _)| *d <= DISTANCIA)
                .collect();
            cerca.sort_unstable_by_key(|(d, _)| *d);
            cerca.truncate(VECINAS);
            let mut suma: HashMap<String, f32> = HashMap::new();
            for (d, otro) in &cerca {
                let peso = 1.0 - *d as f32 / (DISTANCIA + 2) as f32;
                for t in etiquetas_de(otro)? {
                    *suma.entry(t).or_default() += peso;
                }
            }
            for (t, s) in suma {
                // Dos o tres vecinas de cerca con la misma etiqueta ya pesan.
                sumar(&t, (s / 2.5).min(1.0), Porque::Parecidas, &mut puntos);
            }
        }

        // 3. Lo que va junto con lo que ya tiene.
        for t in &propias {
            let Some(&n) = todas.get(t) else { continue };
            if n < 3 {
                continue;
            }
            let mut st = conn.prepare_cached(
                "SELECT t2.name, COUNT(*) FROM item_tags a
                   JOIN tags t1 ON t1.id = a.tag_id AND t1.name = ?1
                   JOIN item_tags b ON b.item_id = a.item_id AND b.tag_id != a.tag_id
                   JOIN tags t2 ON t2.id = b.tag_id
                  GROUP BY t2.id",
            )?;
            for r in st.query_map(params![t], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as f32)))? {
                let (otra, c) = r?;
                let p = c / n as f32;
                if p >= 0.4 {
                    sumar(&otra, p * 0.7, Porque::Juntas, &mut puntos);
                }
            }
        }

        // 4. Su carpeta y su web.
        for f in &it.folders {
            let mut st = conn.prepare_cached(
                "SELECT t.name, COUNT(*), (SELECT COUNT(*) FROM item_folders WHERE folder_id = ?1)
                   FROM item_folders f JOIN item_tags it ON it.item_id = f.item_id
                   JOIN tags t ON t.id = it.tag_id
                  WHERE f.folder_id = ?1 GROUP BY t.id",
            )?;
            for r in st.query_map(params![f], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as f32, r.get::<_, i64>(2)? as f32))
            })? {
                let (t, c, total) = r?;
                if total >= 3.0 && c / total >= 0.5 {
                    sumar(&t, c / total * 0.6, Porque::Carpeta, &mut puntos);
                }
            }
        }
        if let Some(d) = it.source.as_deref().and_then(dominio) {
            let mut st = conn.prepare_cached(
                "SELECT t.name, COUNT(*), (SELECT COUNT(*) FROM items WHERE source LIKE ?1)
                   FROM items i JOIN item_tags it ON it.item_id = i.id
                   JOIN tags t ON t.id = it.tag_id
                  WHERE i.source LIKE ?1 GROUP BY t.id",
            )?;
            let patron = format!("%{d}%");
            for r in st.query_map(params![patron], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as f32, r.get::<_, i64>(2)? as f32))
            })? {
                let (t, c, total) = r?;
                if total >= 5.0 && c / total >= 0.5 {
                    sumar(&t, c / total * 0.5, Porque::Origen, &mut puntos);
                }
            }
        }
    }

    let ya = tienen_todos.unwrap_or_default();
    let n = ids.len() as f32;
    let mut out: Vec<Sugerencia> = puntos
        .into_iter()
        .filter(|(t, _)| !ya.contains(t))
        .map(|(etiqueta, (p, porque, _))| Sugerencia {
            etiqueta,
            puntos: (p / n).min(1.0),
            porque,
            nueva: false,
        })
        .chain(nuevas.into_values().map(|(etiqueta, p, porque)| Sugerencia {
            etiqueta,
            puntos: (p / n).min(1.0),
            porque,
            nueva: true,
        }))
        .filter(|s| s.puntos >= MINIMO)
        .collect();
    out.sort_by(|a, b| b.puntos.total_cmp(&a.puntos).then_with(|| a.etiqueta.cmp(&b.etiqueta)));
    out.truncate(tope);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn las_palabras_del_nombre_se_comparan_sin_tildes_ni_plural() {
        assert_eq!(palabras("Tacos_PICANTES-01"), vec!["Tacos", "PICANTES"]);
        assert!(formas("picantes").contains(&"picante".to_string()));
        assert!(formas("flores").contains(&"flor".to_string()));
        assert_eq!(palabras("fotoMontañaNevada"), vec!["foto", "Montaña", "Nevada"]);
        assert_eq!(raiz("Café"), "cafe");
        assert!(vale_como_nueva("tacos"));
        assert!(!vale_como_nueva("IMG"));
        assert!(!vale_como_nueva("dsc04512"));
        assert!(!vale_como_nueva("screenshot"));
        assert!(!vale_como_nueva("xkcd"));
        assert!(!vale_como_nueva("DDBOueegmw"));
        assert!(vale_como_nueva("TACOS"));
        assert_eq!(palabras("@nwinvisuals · instagram Dcam-v8A tacos"), vec!["nwinvisuals", "instagram", "tacos"]);
        assert!(carpetas_de_origen("/home/yo/otra.grimorio/cache/salida/01ABC/x.mp4").is_empty());
        assert_eq!(
            carpetas_de_origen("/home/yo/Referencias/Comida_japonesa/ramen.jpg"),
            vec!["comida japonesa", "referencias"]
        );
        assert!(carpetas_de_origen("/home/yo/Descargas/x.jpg").is_empty());
    }

    #[test]
    fn el_dominio_de_la_fuente() {
        assert_eq!(dominio("https://www.pinterest.com/pin/1").as_deref(), Some("pinterest.com"));
        assert_eq!(dominio("nada"), None);
    }

    #[test]
    fn propone_por_nombre_por_parecido_y_por_lo_que_va_junto() {
        use crate::library::Library;
        let dir = tempfile::tempdir().unwrap();
        let mut lib = Library::create(&dir.path().join("x.grimorio"), "x").unwrap();
        let mut ids = Vec::new();
        // Tres de ramen con #comida y #japon, parecidas entre sí.
        for (k, ph) in [0x00ff_00ff_00ff_00ffu64, 0x00ff_00ff_00ff_00fe, 0x00ff_00ff_00ff_00fc]
            .into_iter()
            .enumerate()
        {
            let mut it = crate::Item::new(
                crate::id::new_id(),
                format!("ramen {k}"),
                "jpg".into(),
                10,
                crate::OriginMode::Copy,
            );
            it.tags = vec!["comida".into(), "japon".into()];
            it.phash = format!("{ph:016x}");
            lib.save_item(&it).unwrap();
            ids.push(it.id);
        }
        // Una con #picante para que la etiqueta exista.
        let mut p = crate::Item::new(crate::id::new_id(), "chile".into(), "jpg".into(), 10, crate::OriginMode::Copy);
        p.tags = vec!["picante".into()];
        p.phash = format!("{:016x}", 0xffff_0000_ffff_0000u64);
        lib.save_item(&p).unwrap();
        // La nueva: se llama «tacos picantes» y se parece a los ramen.
        let mut n = crate::Item::new(crate::id::new_id(), "tacos picantes".into(), "jpg".into(), 10, crate::OriginMode::Copy);
        n.phash = format!("{:016x}", 0x00ff_00ff_00ff_00f8u64);
        lib.save_item(&n).unwrap();

        let s = sugerir(&lib, &[n.id.clone()], 5).unwrap();
        let por: HashMap<&str, Porque> = s.iter().map(|x| (x.etiqueta.as_str(), x.porque)).collect();
        assert_eq!(por.get("picante"), Some(&Porque::Nombre), "{s:?}");
        assert_eq!(por.get("comida"), Some(&Porque::Parecidas), "{s:?}");
        assert!(por.contains_key("japon"));

        // De su nombre sale también una nueva: «tacos» no la tiene nadie.
        let tacos = s.iter().find(|x| x.etiqueta == "tacos");
        assert!(tacos.is_some_and(|x| x.nueva), "{s:?}");
        assert!(!s.iter().find(|x| x.etiqueta == "picante").unwrap().nueva);

        // Con #comida puesta, ya no se propone, y #japon llega también por ir junta.
        let mut n2 = lib.load_item(&n.id).unwrap();
        n2.tags = vec!["comida".into()];
        lib.save_item(&n2).unwrap();
        let s = sugerir(&lib, &[n.id.clone()], 5).unwrap();
        assert!(!s.iter().any(|x| x.etiqueta == "comida"));
        assert!(s.iter().any(|x| x.etiqueta == "japon"));
    }
}
