//! Generador de bibliotecas sintéticas.
//!
//! Para medir la malla y el índice necesitamos 100.000 elementos hoy, no cuando
//! alguien haya importado 100.000 fotos. Se generan `unique` imágenes distintas
//! y se reparten entre `items` elementos, cada uno con su `item.json` real.
//!
//! Honestidad del banco de pruebas: **no se escriben los originales**, solo las
//! miniaturas y los metadatos. Mide lo que la interfaz toca (índice, pack,
//! JSON), no la importación. Para eso está `grim bench import`.

use grimorio_core::{
    id, image_ops,
    item::{Item, OriginMode},
    Library,
};
use image::{Rgb, RgbImage};
use rayon::prelude::*;
use std::path::Path;
use std::time::Instant;

use crate::fmt;

const ADJETIVOS: &[&str] = &[
    "nocturno",
    "sereno",
    "áspero",
    "luminoso",
    "denso",
    "quebrado",
    "húmedo",
    "quieto",
    "eléctrico",
    "tenue",
    "salvaje",
    "mínimo",
    "barroco",
    "frío",
    "cálido",
    "difuso",
];
const SUSTANTIVOS: &[&str] = &[
    "estudio",
    "retrato",
    "textura",
    "cartel",
    "boceto",
    "paisaje",
    "interfaz",
    "render",
    "fachada",
    "reflejo",
    "tipografía",
    "collage",
    "mapa",
    "patrón",
    "retícula",
    "diagrama",
];
const ETIQUETAS: &[&str] = &[
    "referencia",
    "moodboard",
    "inspiración",
    "azul",
    "rojo",
    "monocromo",
    "editorial",
    "producto",
    "arquitectura",
    "naturaleza",
    "tipografía",
    "ilustración",
    "3d",
    "foto",
    "vector",
    "oscuro",
    "claro",
    "cliente",
    "descartado",
    "favorito",
];

struct Variante {
    thumb: Vec<u8>,
    /// La previsualización de 1024 px que usa el visor. Sin ella la biblioteca
    /// sintética no se parece a una de verdad y el banco del visor no mide
    /// nada: el visor se quedaría enseñando la miniatura para siempre.
    previa: Vec<u8>,
    width: u32,
    height: u32,
    palette: Vec<grimorio_core::PaletteEntry>,
    phash: String,
    size: u64,
}

pub fn run(out: &Path, items: usize, unique: usize) -> Result<(), Box<dyn std::error::Error>> {
    let unique = unique.max(1).min(items.max(1));
    println!(
        "{} {items} elementos, {unique} imágenes distintas → {}",
        fmt::accent("generando"),
        out.display()
    );

    std::fs::create_dir_all(out)?;
    let mut lib = Library::open_or_create(out, "Biblioteca sintética")?;

    // --- 1. imágenes distintas -------------------------------------------
    let t0 = Instant::now();
    let variantes: Vec<Variante> = (0..unique)
        .into_par_iter()
        .map(|i| {
            let img = generar_imagen(i);
            let dyn_img = image::DynamicImage::ImageRgb8(img);
            let thumb = image_ops::make_thumb(&dyn_img, grimorio_core::THUMB_GRID);
            let jpeg = image_ops::encode_jpeg(&thumb, 82).expect("codificar miniatura");
            let grande = image_ops::make_thumb(&dyn_img, grimorio_core::THUMB_PREVIEW);
            let previa = image_ops::encode_jpeg(&grande, 86).expect("codificar previsualización");
            Variante {
                width: dyn_img.width(),
                height: dyn_img.height(),
                palette: image_ops::palette(&dyn_img, 5),
                phash: format!("{:016x}", image_ops::dhash(&dyn_img)),
                size: 200_000 + (i as u64 % 97) * 40_000,
                thumb: jpeg,
                previa,
            }
        })
        .collect();
    let t_img = t0.elapsed();

    // --- 2. metadatos ------------------------------------------------------
    let t1 = Instant::now();
    let base_ms = grimorio_core::time::now_ms() - items as u64 * 1000;
    let elementos: Vec<Item> = (0..items)
        .into_par_iter()
        .map(|i| {
            let v = &variantes[i % unique];
            let id = id::id_at(base_ms + i as u64, i as u128);
            let nombre = format!(
                "{} {} {:04}",
                SUSTANTIVOS[i % SUSTANTIVOS.len()],
                ADJETIVOS[(i / 7) % ADJETIVOS.len()],
                i
            );
            let mut it = Item::new(id, nombre, "jpg".into(), v.size, OriginMode::Copy);
            it.width = v.width;
            it.height = v.height;
            it.hash = format!("{:064x}", i as u128);
            it.phash = v.phash.clone();
            it.palette = v.palette.clone();
            it.stars = ((i * 7) % 11).min(5) as u8;
            it.tags = vec![
                ETIQUETAS[i % ETIQUETAS.len()].to_string(),
                ETIQUETAS[(i / 13) % ETIQUETAS.len()].to_string(),
            ];
            it.tags.dedup();
            if i % 5 == 0 {
                it.note = Some(format!(
                    "nota de trabajo sobre {} número {i}",
                    SUSTANTIVOS[(i / 3) % SUSTANTIVOS.len()]
                ));
            }
            it.imported_at = grimorio_core::time::to_rfc3339(base_ms + i as u64 * 1000);
            it.modified_at = it.imported_at.clone();
            it
        })
        .collect();
    let t_meta = t1.elapsed();

    // --- 3. item.json en disco --------------------------------------------
    // `Library` no es Sync (lleva la conexión SQLite dentro), así que la fase
    // paralela solo ve la ruta de `items/`.
    let items_root = lib.items_dir();
    let t2 = Instant::now();
    let escrituras: Vec<Result<(), String>> = elementos
        .par_iter()
        .map(|it| {
            let destino = grimorio_core::item::shard_dir(&items_root, &it.id).join("item.json");
            it.write(&destino).map_err(|e| e.to_string())
        })
        .collect();
    for e in escrituras.iter().filter_map(|r| r.as_ref().err()).take(3) {
        eprintln!("{} {e}", fmt::warn("aviso:"));
    }
    let t_json = t2.elapsed();

    // --- 3.5 previsualizaciones -------------------------------------------
    let t_prev = Instant::now();
    {
        let raiz = lib.root().to_path_buf();
        let rutas: Vec<(std::path::PathBuf, usize)> = elementos
            .iter()
            .enumerate()
            .map(|(i, it)| (grimorio_core::preview_path(&raiz, &it.id), i % unique))
            .collect();
        rutas
            .par_iter()
            .try_for_each(|(ruta, v)| -> std::io::Result<()> {
                if let Some(dir) = ruta.parent() {
                    std::fs::create_dir_all(dir)?;
                }
                std::fs::write(ruta, &variantes[*v].previa)
            })?;
    }
    let t_previas = t_prev.elapsed();

    // --- 4. pack de miniaturas --------------------------------------------
    let t3 = Instant::now();
    let mut pack = lib.pack_writer()?;
    let mut refs = Vec::with_capacity(items);
    for (i, it) in elementos.iter().enumerate() {
        refs.push(pack.append(&it.id, &variantes[i % unique].thumb)?);
    }
    pack.flush()?;
    let pack_bytes = pack.len();
    drop(pack);
    let t_pack = t3.elapsed();

    // --- 5. índice ---------------------------------------------------------
    let t4 = Instant::now();
    let filas: Vec<(Item, Option<grimorio_core::thumbs::ThumbRef>)> = elementos
        .into_iter()
        .zip(refs.into_iter())
        .map(|(it, r)| (it, Some(r)))
        .collect();
    let total = filas.len();
    for (n, chunk) in filas.chunks(8192).enumerate() {
        lib.index_mut()
            .upsert_many(chunk.iter().map(|(i, t)| (i, *t)))?;
        let hechos = ((n + 1) * 8192).min(total);
        print!("\r  {}", fmt::progress_bar(hechos, total, 28));
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }
    lib.index().conn.execute_batch("ANALYZE")?;
    let t_index = t4.elapsed();
    println!("\r{}", " ".repeat(60));

    let fila = |k: &str, d: std::time::Duration| {
        println!(
            "  {} {:>9}  {}",
            fmt::dim(&fmt::ellipsis(k, 22)),
            fmt::millis(d.as_millis()),
            fmt::dim(&format!(
                "{:.0}/s",
                items as f64 / d.as_secs_f64().max(0.0001)
            ))
        )
    };
    println!("{}", fmt::bold("generación terminada"));
    fila("imágenes distintas", t_img);
    fila("metadatos en memoria", t_meta);
    fila("item.json a disco", t_json);
    fila("previsualizaciones", t_previas);
    fila("pack de miniaturas", t_pack);
    fila("índice SQLite", t_index);
    println!(
        "  {} {}",
        fmt::dim(&fmt::ellipsis("pack", 22)),
        fmt::bytes(pack_bytes)
    );
    println!(
        "\n  {} {}",
        fmt::dim("para usarla:"),
        fmt::bold(&format!("grim -L {} stats", out.display()))
    );
    Ok(())
}

/// Imágenes de mentira que se parecen a fotos de verdad: paletas apagadas,
/// degradados suaves, viñeteado y un poco de grano. No es capricho estético —
/// con tableros de ajedrez fluorescentes no se puede juzgar ni la disposición de
/// la malla ni el peso real de un JPEG de miniatura.
fn generar_imagen(i: usize) -> RgbImage {
    let formas = [
        (1600u32, 1000u32),
        (1000, 1600),
        (1200, 1200),
        (1920, 1080),
        (900, 1350),
        (2000, 800),
    ];
    let (w, h) = formas[i % formas.len()];
    let mut im = RgbImage::new(w, h);

    // Dos colores de la misma familia + un acento, en tonos desaturados.
    let tono = (i as f32 * 0.191) % 1.0;
    let sombra = hsl(tono, 0.30, 0.16);
    let luz = hsl((tono + 0.06) % 1.0, 0.38, 0.62);
    let acento = hsl((tono + 0.45) % 1.0, 0.45, 0.55);

    let modo = i % 5;
    let cx = 0.5 + ((i % 7) as f32 - 3.0) * 0.08;
    let cy = 0.4 + ((i % 5) as f32 - 2.0) * 0.09;
    let horizonte = 0.45 + ((i % 11) as f32 - 5.0) * 0.03;

    for (x, y, p) in im.enumerate_pixels_mut() {
        let fx = x as f32 / w as f32;
        let fy = y as f32 / h as f32;

        let mut c = match modo {
            // Degradado suave en diagonal.
            0 => mezcla(sombra, luz, (fx * 0.6 + fy * 0.4).clamp(0.0, 1.0)),
            // Foco de luz difuso sobre fondo oscuro.
            1 => {
                let r = ((fx - cx).powi(2) + (fy - cy).powi(2) * 1.4).sqrt();
                mezcla(luz, sombra, (r * 1.7).clamp(0.0, 1.0))
            }
            // "Paisaje": cielo arriba, masa oscura abajo.
            2 => {
                let t = ((fy - horizonte) * 6.0).clamp(-1.0, 1.0) * 0.5 + 0.5;
                mezcla(luz, sombra, t)
            }
            // Bloque de color con una banda de acento.
            3 => {
                let banda = ((fy - horizonte).abs() * 9.0).clamp(0.0, 1.0);
                mezcla(acento, mezcla(sombra, luz, fy), banda)
            }
            // Degradado vertical con acento en una esquina.
            _ => {
                let base = mezcla(sombra, luz, fy * 0.8 + 0.1);
                let esquina = ((1.0 - fx).powi(2) + fy.powi(2)).sqrt();
                mezcla(acento, base, (esquina * 1.3).clamp(0.0, 1.0))
            }
        };

        // Viñeteado: oscurece los bordes como cualquier objetivo real.
        let vin = 1.0 - 0.35 * (((fx - 0.5).powi(2) + (fy - 0.5).powi(2)) * 2.0).min(1.0);
        // Grano determinista, para que el JPEG pese lo que pesaría de verdad.
        let grano =
            ((x.wrapping_mul(1103515245) ^ y.wrapping_mul(12345)) % 17) as f32 / 17.0 * 0.06 - 0.03;
        for k in 0..3 {
            c[k] = ((c[k] * vin + grano).clamp(0.0, 1.0) * 255.0) as u8 as f32;
        }
        *p = Rgb([c[0] as u8, c[1] as u8, c[2] as u8]);
    }
    im
}

fn mezcla(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// HSL a RGB en 0..1. Suficiente para inventar paletas coherentes.
fn hsl(hh: f32, s: f32, l: f32) -> [f32; 3] {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = hh * 6.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    [r + m, g + m, b + m]
}
