//! La documentación no se queda atrás.
//!
//! `docs/FORMATO.md` promete que con él basta para recuperar una biblioteca
//! sin Grimorio, y `NOVEDADES.md` es lo que se enseña al actualizar y el texto
//! de cada release. Las dos se olvidan con facilidad al añadir algo: estas
//! pruebas fallan en cuanto el código escribe algo que la documentación no
//! cuenta, o se sube la versión sin decir qué trae. Ver `docs/PUBLICAR.md`.

use grimorio_core::{Item, OriginMode, PaletteEntry, Receta, Tramo};
use serde_json::Value;

const FORMATO: &str = include_str!("../../../docs/FORMATO.md");
const NOVEDADES: &str = include_str!("../../../NOVEDADES.md");

/// Un elemento con todos los campos puestos, para que ninguno se omita al
/// escribirlo.
fn completo() -> Item {
    let mut it = Item::new("01X".into(), "n".into(), "mp4".into(), 1, OriginMode::Copy);
    it.width = 1;
    it.height = 1;
    it.duration_ms = Some(1);
    it.pages = Some(1);
    it.triangles = Some(1);
    it.size_mm = Some([1.0; 3]);
    it.hash = "h".into();
    it.phash = "p".into();
    it.source = Some("s".into());
    it.folders = vec!["f".into()];
    it.orden.insert("f".into(), 1.0);
    it.tags = vec!["t".into()];
    it.stars = 1;
    it.note = Some("n".into());
    it.palette = vec![PaletteEntry { rgb: [1, 2, 3], w: 1.0 }];
    it.origin.path = Some("p".into());
    it.origin.inode = Some(1);
    it.trashed = true;
    it.adult = true;
    it.rotation = 90;
    it.pila = Some("01Y".into());
    it.receta = Some(Receta {
        formato: "mp4".into(),
        calidad: 1,
        lado: 1,
        recorte: Some("m".into()),
        tramo: Some(Tramo {
            desde_ms: 1,
            hasta_ms: 2,
            exacto: true,
        }),
    });
    it
}

/// Todas las claves de un JSON, también las de dentro.
fn claves(v: &Value, ruta: &str, out: &mut Vec<String>) {
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                let r = if ruta.is_empty() { k.clone() } else { format!("{ruta}.{k}") };
                out.push(r.clone());
                claves(x, &r, out);
            }
        }
        Value::Array(a) => {
            for x in a {
                claves(x, ruta, out);
            }
        }
        _ => {}
    }
}

#[test]
fn cada_campo_de_item_json_sale_en_formato_md() {
    let v = serde_json::to_value(completo()).unwrap();
    let mut todas = Vec::new();
    claves(&v, "", &mut todas);
    // Las claves de `orden` son ids de carpeta, datos y no campos.
    todas.retain(|r| !r.starts_with("orden."));
    // Se busca la última parte (`desde_ms`, no `receta.tramo.desde_ms`), como
    // se escribe en el texto: entre comillas en un ejemplo o entre acentos.
    let faltan: Vec<&String> = todas
        .iter()
        .filter(|r| {
            let k = r.rsplit('.').next().unwrap();
            !FORMATO.contains(&format!("\"{k}\"")) && !FORMATO.contains(&format!("`{k}`"))
                && !FORMATO.contains(&format!("`{r}`"))
        })
        .collect();
    assert!(
        faltan.is_empty(),
        "docs/FORMATO.md no cuenta estos campos de item.json: {faltan:?}"
    );
}

#[test]
fn cada_archivo_de_la_biblioteca_sale_en_formato_md() {
    let archivos = [
        "library.json",
        "index.sqlite",
        grimorio_core::folder::FOLDERS_FILE,
        grimorio_core::busquedas::ARCHIVO,
        grimorio_core::vigiladas::ARCHIVO,
        grimorio_core::grupos::ARCHIVO,
        grimorio_core::library::CERROJO_APP,
    ];
    let faltan: Vec<&&str> = archivos.iter().filter(|a| !FORMATO.contains(**a)).collect();
    assert!(faltan.is_empty(), "docs/FORMATO.md no cuenta estos archivos: {faltan:?}");
}

#[test]
fn la_version_tiene_su_apartado_en_novedades() {
    let v = env!("CARGO_PKG_VERSION");
    assert!(
        NOVEDADES.lines().any(|l| l.trim() == format!("## {v}")),
        "NOVEDADES.md no tiene el apartado «## {v}»: escribe qué trae esta versión"
    );
}
