//! Mediciones con presupuesto.
//!
//! Cada objetivo del plan tiene aquí un número que se puede volver a medir. Si
//! algo se sale del presupuesto, el comando termina con error: así una regresión
//! rompe la build en vez de pasar desapercibida.

use grimorio_core::{
    import::{import, ImportOptions},
    query::{Orientation, SortBy},
    Library, Query,
};
use image::{Rgb, RgbImage};
use rayon::prelude::*;
use std::time::Instant;

use crate::fmt;

/// Presupuestos del plan, en milisegundos.
const P95_BUSQUEDA_MS: f64 = 30.0;
const P95_PAGINA_MS: f64 = 16.0;

struct Muestras(Vec<f64>);

impl Muestras {
    fn pct(&mut self, p: f64) -> f64 {
        if self.0.is_empty() {
            return 0.0;
        }
        self.0
            .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let i = ((self.0.len() - 1) as f64 * p).round() as usize;
        self.0[i]
    }
}

fn medir(runs: usize, mut f: impl FnMut() -> usize) -> (Muestras, usize) {
    let mut m = Vec::with_capacity(runs);
    let mut ultimo = 0;
    for _ in 0..runs {
        let t = Instant::now();
        ultimo = f();
        m.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    (Muestras(m), ultimo)
}

pub fn queries(lib: &Library, runs: usize) -> Result<(), Box<dyn std::error::Error>> {
    let total = lib.index().count()?;
    println!(
        "{} {} elementos · {runs} repeticiones\n",
        fmt::bold("banco de consultas"),
        total
    );

    let casos: Vec<(&str, Query, f64)> = vec![
        ("primera página (200)", Query::default(), P95_PAGINA_MS),
        (
            "página 50 (offset 10k)",
            Query {
                offset: 10_000,
                ..Default::default()
            },
            P95_PAGINA_MS,
        ),
        (
            "texto: 'textura'",
            Query {
                text: Some("textura".into()),
                ..Default::default()
            },
            P95_BUSQUEDA_MS,
        ),
        (
            "texto con prefijo: 'noct'",
            Query {
                text: Some("noct".into()),
                ..Default::default()
            },
            P95_BUSQUEDA_MS,
        ),
        (
            "texto sin acentos: 'tipografia'",
            Query {
                text: Some("tipografia".into()),
                ..Default::default()
            },
            P95_BUSQUEDA_MS,
        ),
        (
            "etiqueta: 'moodboard'",
            Query {
                tags: vec!["moodboard".into()],
                ..Default::default()
            },
            P95_BUSQUEDA_MS,
        ),
        (
            "4+ estrellas y apaisadas",
            Query {
                min_stars: Some(4),
                orientation: Some(Orientation::Landscape),
                ..Default::default()
            },
            P95_BUSQUEDA_MS,
        ),
        (
            "por color #1f6b72",
            Query {
                color: Some([31, 107, 114]),
                ..Default::default()
            },
            P95_BUSQUEDA_MS,
        ),
        (
            "orden por nombre",
            Query {
                sort: SortBy::NameAsc,
                ..Default::default()
            },
            P95_BUSQUEDA_MS,
        ),
        (
            "al azar",
            Query {
                sort: SortBy::Random,
                ..Default::default()
            },
            P95_BUSQUEDA_MS,
        ),
    ];

    println!(
        "  {}  {:>8} {:>8} {:>8}  {:>7}",
        fmt::dim(&fmt::ellipsis("consulta", 30)),
        fmt::dim("p50"),
        fmt::dim("p95"),
        fmt::dim("máx"),
        fmt::dim("filas")
    );

    let mut fallos = Vec::new();
    for (nombre, q, presupuesto) in casos {
        let (mut m, filas) = medir(runs, || {
            lib.index().search(&q).map(|r| r.len()).unwrap_or(0)
        });
        let p50 = m.pct(0.50);
        let p95 = m.pct(0.95);
        let max = m.pct(1.0);
        let ok = p95 <= presupuesto;
        if !ok {
            fallos.push(format!("{nombre}: p95 {p95:.1} ms > {presupuesto:.0} ms"));
        }
        println!(
            "  {}  {:>8} {:>8} {:>8}  {:>7}  {}",
            fmt::ellipsis(nombre, 30),
            format!("{p50:.2}"),
            format!("{p95:.2}"),
            format!("{max:.2}"),
            filas,
            if ok {
                fmt::accent("ok")
            } else {
                fmt::warn("FUERA")
            }
        );
    }

    println!();
    if fallos.is_empty() {
        println!(
            "{}",
            fmt::accent("todas las consultas dentro de presupuesto")
        );
        Ok(())
    } else {
        for f in &fallos {
            println!("  {} {f}", fmt::warn("fuera de presupuesto:"));
        }
        Err(format!("{} consultas fuera de presupuesto", fallos.len()).into())
    }
}

pub fn import_bench(n: usize, size: u32) -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!("grimorio-bench-{}", std::process::id()));
    let src = dir.join("origen");
    std::fs::create_dir_all(&src)?;
    println!(
        "{} {n} imágenes de {size}×{} en {}",
        fmt::accent("preparando"),
        size * 3 / 4,
        src.display()
    );

    let t0 = Instant::now();
    (0..n)
        .into_par_iter()
        .try_for_each(|i| -> std::io::Result<()> {
            let mut im = RgbImage::new(size, size * 3 / 4);
            // Cada imagen tiene que ser distinta de verdad: si se repiten, el
            // deduplicador por contenido hace bien su trabajo y el banco acaba
            // midiendo 256 importaciones en vez de las 1000 que pediste.
            let base = ((i * 37) % 256) as u8;
            let sesgo = (i % 9973) as u32;
            for (x, y, p) in im.enumerate_pixels_mut() {
                *p = Rgb([
                    base.wrapping_add((x / 8) as u8),
                    ((y / 6 + sesgo) % 256) as u8,
                    (((x.wrapping_mul(y).wrapping_add(sesgo.wrapping_mul(7919))) % 251) as u8)
                        .wrapping_add(base),
                ]);
            }
            im.save(src.join(format!("img{i:06}.jpg")))
                .map_err(|e| std::io::Error::other(e.to_string()))
        })?;
    let preparar = t0.elapsed();
    let bytes_origen: u64 = std::fs::read_dir(&src)?
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum();
    println!(
        "  {} en {} ({})",
        fmt::dim("generadas"),
        fmt::millis(preparar.as_millis()),
        fmt::bytes(bytes_origen)
    );

    let libdir = dir.join("bench.grimorio");
    let _ = std::fs::remove_dir_all(&libdir);
    let mut lib = Library::open_or_create(&libdir, "bench")?;

    let rep = import(
        &mut lib,
        &[src.clone()],
        &ImportOptions::default(),
        &|_, _| {},
    )?;

    println!("\n{}", fmt::bold("importación"));
    println!(
        "  {} {} elementos en {} · {}",
        fmt::dim("total        "),
        rep.imported,
        fmt::millis(rep.elapsed_ms),
        fmt::accent(&format!("{:.1} img/s", rep.per_second()))
    );
    println!("  {} {}", fmt::dim("datos leídos "), fmt::bytes(rep.bytes));
    println!(
        "  {} {}",
        fmt::dim("por imagen   "),
        fmt::millis((rep.elapsed_ms as f64 / rep.imported.max(1) as f64) as u128)
    );
    if rep.duplicates > 0 {
        // Si esto no es cero, el número de arriba no mide lo que crees.
        println!(
            "  {} {} (no se midieron: contenido repetido)",
            fmt::warn("duplicados   "),
            rep.duplicates
        );
    }
    if !rep.failed.is_empty() {
        println!("  {} {}", fmt::warn("fallos       "), rep.failed.len());
    }

    let objetivo = 20.0;
    let ok = rep.per_second() >= objetivo;
    println!(
        "\n  presupuesto: > {objetivo:.0} img/s → {}",
        if ok {
            fmt::accent("ok")
        } else {
            fmt::warn("FUERA")
        }
    );
    println!(
        "{}",
        fmt::dim(&format!("  (limpia con: rm -rf {})", dir.display()))
    );
    if ok {
        Ok(())
    } else {
        Err("importación por debajo del presupuesto".into())
    }
}

pub fn reindex(lib: &mut Library) -> Result<(), Box<dyn std::error::Error>> {
    let antes = lib.index().count()?;
    let t = Instant::now();
    let informe = lib.reindex()?;
    let d = t.elapsed();
    let n = informe.vistos;
    println!(
        "{} {n} elementos en {} · {}",
        fmt::bold("reconstrucción del índice"),
        fmt::millis(d.as_millis()),
        fmt::accent(&format!(
            "{:.0} elem/s",
            n as f64 / d.as_secs_f64().max(0.0001)
        ))
    );
    if antes != n {
        println!(
            "  {} el índice tenía {antes} y en disco hay {n}",
            fmt::warn("aviso:")
        );
    }
    Ok(())
}

/// La puerta de M2: etiquetar diez mil elementos sin congelar la interfaz.
///
/// Aquí no hay interfaz, así que lo que se mide es lo único que puede
/// congelarla: cuánto trabajo hace el hilo del núcleo y —lo importante— cada
/// cuánto suelta un aviso de progreso. Un lote que tarda cuatro segundos pero
/// avisa veinte veces se ve como una barra que avanza; uno que tarda dos y no
/// avisa se ve como un programa colgado.
pub fn lote(lib: &mut Library, n: usize) -> Result<(), Box<dyn std::error::Error>> {
    let hits = lib.index().search(&Query {
        limit: n,
        ..Default::default()
    })?;
    let ids: Vec<String> = hits.iter().map(|h| h.id.clone()).collect();
    if ids.is_empty() {
        return Err("la biblioteca está vacía".into());
    }
    println!(
        "{} {} elementos\n",
        fmt::bold("banco de edición en lote"),
        ids.len()
    );

    let etiqueta = format!("banco-{}", ids.len());
    let huecos = std::sync::Mutex::new(Vec::<f64>::new());
    let ultimo = std::sync::Mutex::new(Instant::now());

    let t = Instant::now();
    let (tocados, _) = lib.editar(
        &ids,
        "etiquetar",
        |item| {
            let mut v = item.tags.clone();
            v.push(etiqueta.clone());
            grimorio_core::Parche::etiquetas(v)
        },
        &|_, _| {
            let mut u = ultimo.lock().unwrap();
            huecos
                .lock()
                .unwrap()
                .push(u.elapsed().as_secs_f64() * 1000.0);
            *u = Instant::now();
        },
    )?;
    let ida = t.elapsed().as_secs_f64() * 1000.0;

    let t = Instant::now();
    lib.deshacer(&|_, _| {})?;
    let vuelta = t.elapsed().as_secs_f64() * 1000.0;

    let mut h = Muestras(huecos.into_inner().unwrap());
    let peor = h.pct(1.0);
    let mediana = h.pct(0.5);

    println!(
        "  etiquetar           {} · {}",
        fmt::millis(ida as u128),
        fmt::accent(&format!("{:.0} elem/s", tocados as f64 / (ida / 1000.0)))
    );
    println!("  deshacerlo          {}", fmt::millis(vuelta as u128));
    println!("  avisos de progreso  {}", h.0.len());
    println!(
        "  hueco entre avisos  mediana {:.0} ms · peor {:.0} ms",
        mediana, peor
    );

    // El presupuesto de verdad: entre dos avisos no puede pasar tanto tiempo
    // como para que la barra parezca parada. Un cuarto de segundo es el umbral
    // conocido a partir del cual una interfaz «se ha quedado colgada».
    const HUECO_MAX_MS: f64 = 250.0;
    println!(
        "  presupuesto         {HUECO_MAX_MS:.0} ms entre avisos  →  {}",
        if peor <= HUECO_MAX_MS {
            fmt::accent("dentro")
        } else {
            fmt::warn("FUERA")
        }
    );
    if peor > HUECO_MAX_MS {
        return Err(format!("el peor hueco fue de {peor:.0} ms").into());
    }
    Ok(())
}
