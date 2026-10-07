//! `grim` — el núcleo de Grimorio entero, sin abrir una ventana.
//!
//! Existe por tres razones: probar el núcleo de verdad, medir rendimiento con
//! números reproducibles, y ser el primer cliente del mismo bus de comandos que
//! usarán la interfaz y los plugins.

mod bench;
mod fmt;
mod synth;

use clap::{Parser, Subcommand};
use grimorio_core::{
    filtro,
    import::{import, ImportOptions},
    item::OriginMode,
    query::{Orientation, SortBy},
    Library, Parche, Query,
};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "grim",
    version,
    about = "Grimorio — biblioteca visual para Linux",
    long_about = "Grimorio — biblioteca visual para Linux.\n\nTus datos son una carpeta con un JSON por elemento; el índice es derivado y \
                  se puede reconstruir en cualquier momento con `grim reindex`."
)]
struct Cli {
    /// Ruta de la biblioteca. Por defecto: $GRIMORIO_LIB, o el directorio actual.
    #[arg(long, short = 'L', global = true, value_name = "RUTA")]
    lib: Option<PathBuf>,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Crea una biblioteca vacía
    Init {
        #[arg(value_name = "RUTA")]
        path: PathBuf,
        /// Nombre visible. Por defecto, el del directorio.
        #[arg(long)]
        name: Option<String>,
    },
    /// Importa archivos o carpetas
    Import {
        #[arg(value_name = "RUTA", required = true)]
        paths: Vec<PathBuf>,
        /// Qué hacer con los originales
        #[arg(long, default_value = "copy", value_parser = ["copy", "move", "ref"])]
        mode: String,
        /// Etiqueta que se aplica a todo lo importado (repetible)
        #[arg(long = "tag", value_name = "ETIQUETA")]
        tags: Vec<String>,
        /// No generar previsualizaciones de 1024 px
        #[arg(long)]
        no_previews: bool,
        /// Importar aunque el contenido ya esté en la biblioteca
        #[arg(long)]
        allow_duplicates: bool,
        /// De dónde viene (una página o imagen de la web); se guarda en cada
        /// elemento importado y se busca con el texto
        #[arg(long, value_name = "URL")]
        origen: Option<String>,
    },
    /// Lista lo último importado
    Ls {
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long, default_value = "recientes")]
        sort: String,
    },
    /// Busca por texto y filtros
    Search {
        /// Palabras a buscar (prefijo automático: "gat" encuentra "gato")
        #[arg(value_name = "TEXTO")]
        terms: Vec<String>,
        #[arg(long = "ext", value_name = "EXT")]
        exts: Vec<String>,
        #[arg(long = "tag", value_name = "ETIQUETA")]
        tags: Vec<String>,
        #[arg(long, value_name = "N")]
        stars: Option<u8>,
        /// apaisada | vertical | cuadrada
        #[arg(long)]
        orientation: Option<String>,
        /// Color objetivo, por ejemplo "#1f6b72"
        #[arg(long)]
        color: Option<String>,
        #[arg(long, default_value = "recientes")]
        sort: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        /// Salida JSON, para scripts
        #[arg(long)]
        json: bool,
    },
    /// Muestra la ficha completa de un elemento
    Show {
        #[arg(value_name = "ID")]
        id: String,
    },
    /// Añade o quita etiquetas
    Tag {
        #[arg(value_name = "ID", required = true)]
        ids: Vec<String>,
        #[arg(long = "add", value_name = "ETIQUETA")]
        add: Vec<String>,
        #[arg(long = "rm", value_name = "ETIQUETA")]
        remove: Vec<String>,
    },
    /// Pone estrellas (0-5)
    Star {
        #[arg(value_name = "ID")]
        id: String,
        #[arg(value_name = "0-5")]
        n: u8,
    },
    /// Marca como contenido adulto (o lo desmarca con --quitar)
    Adult {
        #[arg(value_name = "ID", required = true)]
        ids: Vec<String>,
        /// Quitar la marca en vez de ponerla
        #[arg(long)]
        quitar: bool,
    },
    /// Manda a la papelera (o saca de ella con --sacar)
    Trash {
        #[arg(value_name = "ID", required = true)]
        ids: Vec<String>,
        /// Sacar de la papelera en vez de meter
        #[arg(long)]
        sacar: bool,
    },
    /// Borra de verdad todo lo que hay en la papelera. No se deshace
    Empty {
        /// Sin esto solo dice cuántos borraría
        #[arg(long)]
        de_verdad: bool,
    },
    /// Abre el archivo original con el visor del sistema
    Open {
        #[arg(value_name = "ID")]
        id: String,
    },
    /// Busca repetidos y casi iguales
    Dupes {
        /// Cuántos bits pueden diferir en la huella perceptual (0 a 3).
        /// Con 0 solo se buscan los idénticos byte a byte.
        #[arg(long, default_value_t = 3)]
        distancia: u32,
        /// Cuántos grupos enseñar
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Abre un archivo como lo abriría la importación, sin biblioteca, y dice
    /// qué se le saca: familia, medidas, triángulos, y la miniatura si se pide
    Mirar {
        archivo: PathBuf,
        /// Guarda aquí la imagen que saldría de miniatura
        #[arg(long)]
        png: Option<PathBuf>,
    },
    /// Rehace las miniaturas de lo ya importado, con lo que el núcleo sabe
    /// dibujar ahora (la onda de los sonidos, los modelos 3D…). Con la
    /// aplicación cerrada
    Miniaturas {
        /// Solo una familia: audio, video, modelo, documento…
        #[arg(long)]
        tipo: Option<String>,
    },
    /// Resumen de la biblioteca
    Stats,
    /// Reconstruye el índice desde los item.json y el pack de miniaturas
    Reindex,
    /// Genera una biblioteca sintética para medir rendimiento
    Synth {
        #[arg(value_name = "RUTA")]
        out: PathBuf,
        /// Cuántos elementos
        #[arg(long, default_value_t = 100_000)]
        items: usize,
        /// Cuántas imágenes distintas se reparten entre ellos
        #[arg(long, default_value_t = 512)]
        unique: usize,
    },
    /// Mediciones con presupuesto: falla si algo se sale de objetivo
    Bench {
        #[command(subcommand)]
        what: BenchCmd,
    },
}

#[derive(Subcommand)]
enum BenchCmd {
    /// Consultas contra la biblioteca indicada
    Query {
        #[arg(long, default_value_t = 200)]
        runs: usize,
    },
    /// Importación de imágenes generadas al vuelo
    Import {
        #[arg(long, default_value_t = 1000)]
        n: usize,
        /// Lado de las imágenes de prueba
        #[arg(long, default_value_t = 3000)]
        size: u32,
    },
    /// Reconstrucción del índice desde disco
    Reindex,
    /// Edición en lote: la puerta de M2
    Batch {
        /// A cuántos elementos se les pone la etiqueta
        #[arg(long, default_value_t = 10_000)]
        n: usize,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{} {e}", fmt::warn("error:"));
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    match &cli.cmd {
        Cmd::Init { path, name } => {
            let name = name.clone().unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Biblioteca")
                    .to_string()
            });
            std::fs::create_dir_all(path)?;
            let lib = Library::create(path, &name)?;
            println!(
                "{} {} en {}",
                fmt::accent("biblioteca creada:"),
                fmt::bold(&lib.meta().name),
                path.display()
            );
            println!(
                "{}",
                fmt::dim("modo de importación: copiar dentro de la biblioteca")
            );
            Ok(())
        }

        Cmd::Import {
            paths,
            mode,
            tags,
            no_previews,
            allow_duplicates,
            origen,
        } => {
            let mut lib = open_lib_escritura(cli)?;
            let opts = ImportOptions {
                mode: OriginMode::parse(mode).unwrap_or(OriginMode::Copy),
                recursive: true,
                skip_duplicates: !allow_duplicates,
                tags: tags.clone(),
                folder: None,
                previews: !no_previews,
                source: origen.clone(),
            };
            let report = import(&mut lib, paths, &opts, &|done, total| {
                let mut out = std::io::stdout().lock();
                let _ = write!(out, "\r  {}", fmt::progress_bar(done, total, 28));
                let _ = out.flush();
            })?;
            println!("\r{}", " ".repeat(60));
            println!(
                "{} {} elementos · {} · {} · {:.0}/s",
                fmt::accent("importado:"),
                fmt::bold(&report.imported.to_string()),
                fmt::bytes(report.bytes),
                fmt::millis(report.elapsed_ms),
                report.per_second()
            );
            if report.duplicates > 0 {
                println!(
                    "{}",
                    fmt::dim(&format!("  {} duplicados omitidos", report.duplicates))
                );
            }
            if report.unsupported > 0 {
                println!(
                    "{}",
                    fmt::dim(&format!(
                        "  {} archivos de formato no soportado",
                        report.unsupported
                    ))
                );
            }
            for (path, err) in report.failed.iter().take(10) {
                println!("  {} {}: {}", fmt::warn("falló"), path.display(), err);
            }
            if report.failed.len() > 10 {
                println!(
                    "  {}",
                    fmt::dim(&format!("… y {} más", report.failed.len() - 10))
                );
            }
            Ok(())
        }

        Cmd::Ls { limit, sort } => {
            let lib = open_lib(cli)?;
            let q = Query {
                sort: SortBy::parse(sort).unwrap_or(SortBy::ImportedDesc),
                limit: *limit,
                ..Default::default()
            };
            let hits = lib.index().search(&q)?;
            print_hits(&hits);
            Ok(())
        }

        Cmd::Search {
            terms,
            exts,
            tags,
            stars,
            orientation,
            color,
            sort,
            limit,
            json,
        } => {
            let lib = open_lib(cli)?;
            let base = Query {
                exts: exts.clone(),
                tags: tags.clone(),
                min_stars: *stars,
                orientation: orientation.as_deref().and_then(Orientation::parse),
                color: color.as_deref().and_then(parse_hex_color),
                sort: SortBy::parse(sort).unwrap_or(SortBy::ImportedDesc),
                limit: *limit,
                ..Default::default()
            };
            // Lo escrito pasa por el mismo analizador que la caja de búsqueda de
            // la ventana. Antes se metía tal cual en `text`, así que `tipo:video`
            // aquí buscaba la cadena «tipo:video» y en la ventana filtraba: dos
            // verdades sobre lo que entiende el mismo buscador.
            let (q, quejas) = filtro::parsear(&terms.join(" "), base);
            for aviso in &quejas {
                eprintln!("{} {aviso}", fmt::dim("aviso:"));
            }
            let started = std::time::Instant::now();
            let hits = lib.index().search(&q)?;
            let took = started.elapsed();
            if *json {
                println!("{}", serde_json::to_string_pretty(&hits)?);
            } else {
                print_hits(&hits);
                println!(
                    "{}",
                    fmt::dim(&format!(
                        "  {} resultado{} en {:.1} ms",
                        hits.len(),
                        if hits.len() == 1 { "" } else { "s" },
                        took.as_secs_f64() * 1000.0
                    ))
                );
            }
            Ok(())
        }

        Cmd::Show { id } => {
            let lib = open_lib(cli)?;
            let item = lib.load_item(&resolver_id(&lib, id)?)?;
            let w = 14;
            let row = |k: &str, v: String| println!("  {} {v}", fmt::dim(&fmt::ellipsis(k, w)));
            println!("{}", fmt::bold(&item.name));
            row(
                "id",
                format!("{}   {}", corto(&item.id), fmt::dim(&item.id)),
            );
            row("formato", item.ext.to_uppercase());
            row("dimensiones", format!("{} × {}", item.width, item.height));
            row("peso", fmt::bytes(item.size));
            row("estrellas", fmt::stars(item.stars));
            row(
                "etiquetas",
                if item.tags.is_empty() {
                    fmt::dim("(ninguna)").to_string()
                } else {
                    item.tags.join(", ")
                },
            );
            row("importado", item.imported_at.clone());
            row("origen", item.origin.mode.as_str().to_string());
            if let Some(p) = &item.origin.path {
                row("ruta", p.clone());
            }
            if let Some(n) = &item.note {
                row("nota", n.clone());
            }
            if !item.palette.is_empty() {
                // Pegados forman una barra de color; en texto plano necesitan
                // aire o se leen como un churro de hexadecimales.
                let sep = if fmt::color_on() { "" } else { " " };
                let sw: String = item
                    .palette
                    .iter()
                    .map(|p| fmt::swatch(p.rgb))
                    .collect::<Vec<_>>()
                    .join(sep);
                row("paleta", sw);
            }
            row("archivo", lib.original_path(&item).display().to_string());
            Ok(())
        }

        Cmd::Tag { ids, add, remove } => {
            let mut lib = open_lib_escritura(cli)?;
            let resueltos: Vec<String> = ids
                .iter()
                .map(|i| resolver_id(&lib, i))
                .collect::<Result<_, _>>()?;
            let (tocados, _) = lib.editar(
                &resueltos,
                "etiquetar",
                |item| {
                    let mut v = item.tags.clone();
                    for t in add {
                        if !v.iter().any(|x| x == t) {
                            v.push(t.clone());
                        }
                    }
                    v.retain(|t| !remove.contains(t));
                    Parche::etiquetas(v)
                },
                &|_, _| {},
            )?;
            println!(
                "{} {} elemento{}",
                fmt::accent("etiquetas actualizadas en"),
                tocados,
                if tocados == 1 { "" } else { "s" }
            );
            Ok(())
        }

        Cmd::Star { id, n } => {
            let mut lib = open_lib_escritura(cli)?;
            let id = resolver_id(&lib, id)?;
            lib.editar(
                std::slice::from_ref(&id),
                "estrellas",
                |_| Parche::estrellas(*n),
                &|_, _| {},
            )?;
            let item = lib.load_item(&id)?;
            println!("{} {}", fmt::stars(item.stars), item.name);
            Ok(())
        }

        Cmd::Adult { ids, quitar } => {
            let mut lib = open_lib_escritura(cli)?;
            let resueltos: Vec<String> = ids
                .iter()
                .map(|i| resolver_id(&lib, i))
                .collect::<Result<_, _>>()?;
            let si = !*quitar;
            let (tocados, _) = lib.editar(
                &resueltos,
                if si {
                    "marcar como +18"
                } else {
                    "quitar la marca de +18"
                },
                |_| Parche::adulto(si),
                &|_, _| {},
            )?;
            println!(
                "{} {} elemento{}",
                fmt::accent(if si { "marcado" } else { "sin marcar" }),
                tocados,
                if tocados == 1 { "" } else { "s" }
            );
            Ok(())
        }

        Cmd::Trash { ids, sacar } => {
            let mut lib = open_lib_escritura(cli)?;
            let resueltos: Vec<String> = ids
                .iter()
                .map(|i| resolver_id(&lib, i))
                .collect::<Result<_, _>>()?;
            let (tocados, _) = lib.tirar(&resueltos, !sacar, &|_, _| {})?;
            println!(
                "{} {} elemento{}",
                fmt::accent(if *sacar {
                    "recuperados"
                } else {
                    "a la papelera"
                }),
                tocados,
                if tocados == 1 { "" } else { "s" }
            );
            Ok(())
        }

        Cmd::Empty { de_verdad } => {
            let mut lib = if *de_verdad {
                open_lib_escritura(cli)?
            } else {
                open_lib(cli)?
            };
            let cuantos = lib.index().ids_en_papelera()?.len();
            if !de_verdad {
                println!(
                    "{} {} elemento{} en la papelera. Repite con --de-verdad para borrarlos.",
                    fmt::accent("hay"),
                    cuantos,
                    if cuantos == 1 { "" } else { "s" }
                );
                return Ok(());
            }
            let n = lib.vaciar_papelera(&|_, _| {})?;
            println!("{} {} elementos", fmt::accent("borrados de verdad"), n);
            Ok(())
        }

        Cmd::Open { id } => {
            let lib = open_lib(cli)?;
            let item = lib.load_item(&resolver_id(&lib, id)?)?;
            let ruta = lib.original_path(&item);
            if !ruta.exists() {
                return Err(format!(
                    "el archivo original no está en {}\n  (en modo referencia puede haberse movido o borrado)",
                    ruta.display()
                )
                .into());
            }
            grimorio_core::externo::abrir_fuera(&ruta)
                .map_err(|e| format!("no pude abrirlo con el programa del sistema: {e}"))?;
            println!("{} {}", fmt::accent("abriendo"), ruta.display());
            Ok(())
        }

        Cmd::Dupes { distancia, limit } => {
            let lib = open_lib(cli)?;
            let t = std::time::Instant::now();
            let encontrado = lib.index().duplicados(*distancia)?;
            let mut grupos = encontrado.grupos;
            let d = t.elapsed();
            // Los exactos primero y los grupos grandes antes: es el orden en
            // que uno quiere revisarlos.
            grupos.sort_by(|a, b| b.exacto.cmp(&a.exacto).then(b.ids.len().cmp(&a.ids.len())));
            let sobran: usize = grupos.iter().map(|g| g.ids.len() - 1).sum();
            println!(
                "{} {} grupos · {} elementos de más · {}",
                fmt::bold("duplicados"),
                grupos.len(),
                sobran,
                fmt::millis(d.as_millis())
            );
            if encontrado.saltados > 0 {
                println!(
                    "  {} {} montones eran demasiado grandes para compararlos enteros; la respuesta está incompleta",
                    fmt::warn("aviso:"),
                    encontrado.saltados
                );
            }
            for g in grupos.iter().take(*limit) {
                println!(
                    "\n  {} ({} elementos)",
                    if g.exacto {
                        fmt::accent("idénticos")
                    } else {
                        fmt::warn("parecidos")
                    },
                    g.ids.len()
                );
                for id in &g.ids {
                    let nombre = lib
                        .index()
                        .get(id)?
                        .map(|h| h.name)
                        .unwrap_or_else(|| String::from("?"));
                    println!("    {}  {}", &id[..8.min(id.len())], nombre);
                }
            }
            if grupos.len() > *limit {
                println!("\n  … y {} grupos más", grupos.len() - *limit);
            }
            Ok(())
        }

        Cmd::Stats => {
            let lib = open_lib(cli)?;
            let idx = lib.index();
            let count = idx.count()?;
            println!("{}", fmt::bold(&lib.meta().name));
            println!("  {} {}", fmt::dim("ruta          "), lib.root().display());
            println!("  {} {count}", fmt::dim("elementos     "));
            println!(
                "  {} {}",
                fmt::dim("peso original "),
                fmt::bytes(idx.total_size()?)
            );
            if let Ok(pack) = lib.pack_reader() {
                println!(
                    "  {} {}",
                    fmt::dim("miniaturas    "),
                    fmt::bytes(pack.size() as u64)
                );
            }
            let exts = idx.ext_histogram()?;
            if !exts.is_empty() {
                let linea: Vec<String> = exts
                    .iter()
                    .take(8)
                    .map(|(e, n)| format!("{e} {}", fmt::dim(&n.to_string())))
                    .collect();
                println!("  {} {}", fmt::dim("formatos      "), linea.join("  "));
            }
            let tags = idx.tag_cloud(10)?;
            if !tags.is_empty() {
                let linea: Vec<String> = tags
                    .iter()
                    .map(|(t, n)| format!("{t} {}", fmt::dim(&n.to_string())))
                    .collect();
                println!("  {} {}", fmt::dim("etiquetas     "), linea.join("  "));
            }
            Ok(())
        }

        Cmd::Mirar { archivo, png } => {
            let ext = archivo
                .extension()
                .map(|e| e.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            let bytes = std::fs::read(archivo)?;
            let t = std::time::Instant::now();
            let l = grimorio_core::medio::abrir(archivo, &ext, &bytes)?;
            let ms = t.elapsed().as_millis();
            let familia = grimorio_core::medio::familia_de(&ext)
                .map(|f| f.as_str())
                .unwrap_or("desconocida");
            println!(
                "{} {familia} (.{ext}) en {}",
                fmt::accent("familia:"),
                fmt::millis(ms)
            );
            println!("  {} {}x{}", fmt::dim("medidas      "), l.ancho, l.alto);
            if let Some(d) = l.duracion_ms {
                println!("  {} {:.1} s", fmt::dim("duración     "), d as f64 / 1000.0);
            }
            if let Some(p) = l.paginas {
                println!("  {} {p}", fmt::dim("páginas      "));
            }
            if let Some(t) = l.triangulos {
                println!("  {} {t}", fmt::dim("triángulos   "));
            }
            if let Some([a, f, h]) = l.medidas_mm {
                println!(
                    "  {} {a:.1} × {f:.1} × {h:.1} mm",
                    fmt::dim("tamaño       ")
                );
            }
            if let Some(m) = &l.sin_imagen {
                println!("  {} {m}", fmt::dim("sin imagen   "));
            }
            if let (Some(img), Some(destino)) = (&l.imagen, png) {
                img.save(destino)?;
                println!("  {} {}", fmt::dim("miniatura    "), destino.display());
            }
            Ok(())
        }

        Cmd::Miniaturas { tipo } => {
            let familia = match tipo {
                Some(t) => Some(
                    grimorio_core::medio::Familia::parse(t)
                        .ok_or_else(|| format!("no conozco el tipo «{t}»"))?,
                ),
                None => None,
            };
            let mut lib = open_lib_escritura(cli)?;
            let started = std::time::Instant::now();
            let (hechos, fallos) =
                grimorio_core::import::rehacer_miniaturas(&mut lib, familia, &|k, n| {
                    if n > 0 {
                        eprint!("\r  {k}/{n}   ");
                    }
                })?;
            eprintln!();
            println!(
                "{} {hechos} en {}",
                fmt::accent("miniaturas rehechas:"),
                fmt::millis(started.elapsed().as_millis())
            );
            for (nombre, motivo) in &fallos {
                println!("  {} {nombre}: {motivo}", fmt::dim("sin cambiar"));
            }
            Ok(())
        }

        Cmd::Reindex => {
            let mut lib = open_lib_escritura(cli)?;
            let started = std::time::Instant::now();
            let informe = lib.reindex()?;
            println!(
                "{} {} elementos en {}",
                fmt::accent("índice reconstruido:"),
                informe.vistos,
                fmt::millis(started.elapsed().as_millis())
            );
            // Lo roto se dice, no se calla: el índice ya no los enseña y quien
            // los busque tiene que saber dónde mirar.
            for (ruta, motivo) in informe.rotos.iter().take(10) {
                println!("  {} {}: {motivo}", fmt::warn("saltado"), ruta.display());
            }
            if informe.rotos.len() > 10 {
                println!(
                    "  {}",
                    fmt::dim(&format!("… y {} más", informe.rotos.len() - 10))
                );
            }
            Ok(())
        }

        Cmd::Synth { out, items, unique } => synth::run(out, *items, *unique),

        Cmd::Bench { what } => match what {
            BenchCmd::Query { runs } => bench::queries(&open_lib(cli)?, *runs),
            BenchCmd::Import { n, size } => bench::import_bench(*n, *size),
            BenchCmd::Reindex => bench::reindex(&mut open_lib_escritura(cli)?),
            BenchCmd::Batch { n } => bench::lote(&mut open_lib_escritura(cli)?, *n),
        },
    }
}

fn open_lib(cli: &Cli) -> Result<Library, Box<dyn std::error::Error>> {
    let path = resolve_lib(cli.lib.clone())?;
    Ok(Library::open(&path)?)
}

/// Abre la biblioteca para cambiarla, negándose si la aplicación la tiene
/// abierta.
///
/// La aplicación guarda los JSON de la biblioteca —el árbol de carpetas, las
/// búsquedas— enteros y sin preguntar, y con los dos escribiendo a la vez el
/// último pisaría al otro sin aviso. Leer sí se puede con la ventana abierta:
/// el índice va en WAL y está hecho para eso.
fn open_lib_escritura(cli: &Cli) -> Result<Library, Box<dyn std::error::Error>> {
    let path = resolve_lib(cli.lib.clone())?;
    if let Some(pid) = grimorio_core::library::abierta_por_la_app(&path) {
        return Err(format!(
            "la aplicación tiene abierta esta biblioteca (proceso {pid}).\n  \
             ciérrala para cambiarla desde aquí; si no está abierta, borra {}",
            path.join(grimorio_core::library::CERROJO_APP).display()
        )
        .into());
    }
    Ok(Library::open(&path)?)
}

fn resolve_lib(explicit: Option<PathBuf>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(p) = explicit {
        return Ok(p);
    }
    // Una variable definida pero vacía es lo mismo que no tenerla: si no,
    // el error acaba siendo "«» no es una biblioteca", que no ayuda a nadie.
    if let Some(p) = std::env::var_os("GRIMORIO_LIB").filter(|p| !p.is_empty()) {
        return Ok(PathBuf::from(p));
    }
    let cwd = std::env::current_dir()?;
    if cwd.join("library.json").exists() {
        return Ok(cwd);
    }
    Err(format!(
        "no sé con qué biblioteca trabajar.\n  usa {}, la variable {} o sitúate dentro de una biblioteca",
        fmt::bold("--lib <RUTA>"),
        fmt::bold("GRIMORIO_LIB")
    )
    .into())
}

/// Acepta el id completo, el identificador corto de los listados (los ocho
/// últimos caracteres) o un prefijo.
///
/// Los listados enseñan el final del id a propósito: el principio de un ULID es
/// el reloj, y todo lo importado en el mismo cuarto de segundo lo comparte.
fn resolver_id(lib: &Library, entrada: &str) -> Result<String, Box<dyn std::error::Error>> {
    let entrada = entrada.trim();
    if entrada.len() == 26 && lib.index().get(&entrada.to_ascii_uppercase())?.is_some() {
        return Ok(entrada.to_ascii_uppercase());
    }
    let mut candidatos = lib.index().ids_con_sufijo(entrada, 6)?;
    if candidatos.is_empty() {
        candidatos = lib.index().ids_con_prefijo(entrada, 6)?;
    }
    match candidatos.len() {
        0 => Err(format!("no hay ningún elemento cuyo id empiece por «{entrada}»").into()),
        1 => Ok(candidatos.into_iter().next().unwrap()),
        _ => {
            let mut msg = format!("«{entrada}» encaja con {} elementos:\n", candidatos.len());
            for id in candidatos.iter().take(5) {
                let nombre = lib.index().get(id)?.map(|h| h.name).unwrap_or_default();
                msg.push_str(&format!("  {}  {nombre}\n", corto(id)));
            }
            msg.push_str("  usa unos caracteres más");
            Err(msg.into())
        }
    }
}

/// Identificador corto para enseñar en listados: el final del ULID.
fn corto(id: &str) -> &str {
    if id.len() >= 26 {
        &id[18..]
    } else {
        id
    }
}

fn print_hits(hits: &[grimorio_core::QueryHit]) {
    if hits.is_empty() {
        println!("{}", fmt::dim("  (nada por aquí)"));
        return;
    }
    for h in hits {
        let dim_str = format!("{}×{}", h.width, h.height);
        println!(
            "  {} {} {}  {}  {}  {}",
            h.dominant.map(fmt::swatch).unwrap_or_else(|| "  ".into()),
            fmt::dim(corto(&h.id)),
            fmt::ellipsis(&h.name, 34),
            fmt::ellipsis(&dim_str, 11),
            fmt::ellipsis(&fmt::bytes(h.size), 8),
            fmt::stars(h.stars),
        );
    }
}

fn parse_hex_color(s: &str) -> Option<[u8; 3]> {
    let s = s.trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    Some([
        u8::from_str_radix(&s[0..2], 16).ok()?,
        u8::from_str_radix(&s[2..4], 16).ok()?,
        u8::from_str_radix(&s[4..6], 16).ok()?,
    ])
}

#[allow(dead_code)]
fn ensure_dir(p: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colores_hex() {
        assert_eq!(parse_hex_color("#1f6b72"), Some([31, 107, 114]));
        assert_eq!(parse_hex_color("1f6b72"), Some([31, 107, 114]));
        assert_eq!(parse_hex_color("nope"), None);
    }
}
