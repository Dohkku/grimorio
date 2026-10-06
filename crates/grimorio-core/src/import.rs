//! Importación: de archivos sueltos a elementos de la biblioteca.
//!
//! Forma del trabajo: la parte cara (leer, hashear, decodificar, miniaturizar)
//! va en paralelo sobre todos los núcleos; la parte que toca el índice va en una
//! sola transacción al final. Un archivo corrupto se anota en el informe y no
//! detiene la importación.

use crate::error::{Error, Result};
use crate::image_ops;
use crate::item::{Item, Origin, OriginMode};
use crate::library::Library;
use crate::medio;
use crate::{THUMB_GRID, THUMB_PREVIEW};
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct ImportOptions {
    pub mode: OriginMode,
    pub recursive: bool,
    /// Salta archivos cuyo contenido ya está en la biblioteca (por hash).
    pub skip_duplicates: bool,
    pub tags: Vec<String>,
    pub folder: Option<String>,
    /// Genera también la previsualización de 1024 px. Apagarlo acelera pruebas.
    pub previews: bool,
    /// De dónde viene, si se sabe: la página o la imagen de la web de la que
    /// se pegó o se descargó. Se busca con el texto, como el nombre.
    pub source: Option<String>,
}

impl Default for ImportOptions {
    fn default() -> Self {
        ImportOptions {
            mode: OriginMode::Copy,
            recursive: true,
            skip_duplicates: true,
            tags: Vec::new(),
            folder: None,
            previews: true,
            source: None,
        }
    }
}

#[derive(Debug, Default)]
pub struct ImportReport {
    pub imported: usize,
    pub duplicates: usize,
    pub unsupported: usize,
    /// Los que entraron pero todavía no sabemos dibujar: el vídeo, el PDF, la
    /// tipografía. Están en la biblioteca con sus datos; les falta la cara.
    pub sin_miniatura: usize,
    /// Por qué se quedaron sin cara, sin repetir. Es donde acaba «hace falta
    /// ffmpeg para ver los vídeos»: dicho una vez, no doscientas.
    pub avisos: Vec<String>,
    pub failed: Vec<(PathBuf, String)>,
    /// Los identificadores de lo que entró, en el orden en que entró.
    ///
    /// Los devuelve para que importar se pueda deshacer: sin la lista, el
    /// único deshacer posible sería «no sé qué acabo de meter».
    pub ids: Vec<String>,
    pub bytes: u64,
    pub elapsed_ms: u128,
}

impl ImportReport {
    pub fn per_second(&self) -> f64 {
        if self.elapsed_ms == 0 {
            0.0
        } else {
            self.imported as f64 / (self.elapsed_ms as f64 / 1000.0)
        }
    }
}

struct Prepared {
    item: Item,
    /// De dónde sale. En modo mover, el original **sigue ahí** al acabar la
    /// fase paralela: se mueve después, en serie, cuando ya se sabe que el
    /// pack está libre. Ver `import`.
    origen: PathBuf,
    /// `None` si todavía no sabemos dibujar este formato. El elemento entra
    /// igual y la malla lo enseña con su color de marcador.
    thumb: Option<Vec<u8>>,
    /// Por qué no hay miniatura, si no la hay.
    motivo: Option<String>,
}

/// Rutas de la biblioteca, sin la conexión SQLite dentro.
/// `Library` no es `Sync` (la conexión no lo es), así que la fase paralela
/// trabaja solo con esto.
#[derive(Clone)]
struct Rutas {
    items: PathBuf,
    preview: PathBuf,
}

impl Rutas {
    fn de(lib: &Library) -> Self {
        Rutas {
            items: lib.items_dir(),
            preview: lib.root().join("thumbs").join("preview"),
        }
    }
    fn item_dir(&self, id: &str) -> PathBuf {
        crate::item::shard_dir(&self.items, id)
    }
    fn item_json(&self, id: &str) -> PathBuf {
        self.item_dir(id).join("item.json")
    }
    fn original(&self, id: &str, ext: &str) -> PathBuf {
        self.item_dir(id).join(format!("original.{ext}"))
    }
    /// Quita lo que una importación a medias dejó de un elemento: su carpeta
    /// y su previsualización. Sin quejarse: es limpieza de un fallo que ya se
    /// está contando.
    fn quitar(&self, id: &str) {
        let _ = std::fs::remove_dir_all(self.item_dir(id));
        let _ = std::fs::remove_file(self.preview(id));
    }
    fn preview(&self, id: &str) -> PathBuf {
        self.preview
            .join(id.get(0..2).unwrap_or("__"))
            .join(format!("{id}.jpg"))
    }
}

/// Recorre rutas (archivos o directorios) y devuelve los archivos candidatos.
pub fn collect_files(sources: &[PathBuf], recursive: bool) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for src in sources {
        if src.is_file() {
            out.push(src.clone());
        } else if src.is_dir() {
            walk(src, recursive, &mut out)?;
        }
    }
    out.sort();
    Ok(out)
}

fn walk(dir: &Path, recursive: bool, out: &mut Vec<PathBuf>) -> Result<()> {
    let rd = std::fs::read_dir(dir).map_err(|e| Error::io(dir, e))?;
    for entry in rd {
        let entry = entry.map_err(|e| Error::io(dir, e))?;
        let path = entry.path();
        let ft = entry.file_type().map_err(|e| Error::io(&path, e))?;
        if ft.is_dir() {
            if recursive {
                walk(&path, recursive, out)?;
            }
        } else if ft.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

pub fn import(
    lib: &mut Library,
    sources: &[PathBuf],
    opts: &ImportOptions,
    progress: &(dyn Fn(usize, usize) + Sync),
) -> Result<ImportReport> {
    let started = std::time::Instant::now();
    let files = collect_files(sources, opts.recursive)?;

    let mut report = ImportReport::default();
    let candidates: Vec<PathBuf> = files
        .into_iter()
        .filter(|p| {
            // Se acepta todo lo que sepamos **guardar**, no solo lo que
            // sepamos dibujar. Un vídeo entra hoy sin miniatura y la tendrá el
            // día que la sepamos hacer; dejarlo fuera sería perderlo.
            let ok = p
                .extension()
                .and_then(|e| e.to_str())
                .map(medio::se_puede_guardar)
                .unwrap_or(false);
            if !ok {
                report.unsupported += 1;
            }
            ok
        })
        .collect();

    let known: HashSet<String> = if opts.skip_duplicates {
        lib.index().known_hashes()?
    } else {
        HashSet::new()
    };
    let seen: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
    let done = AtomicUsize::new(0);
    let total = candidates.len();
    let dup_count = AtomicUsize::new(0);
    let rutas = Rutas::de(lib);

    // El pack se abre **antes** de tocar nada. Es lo único que puede estar
    // ocupado por otro proceso, y descubrirlo después de la fase paralela
    // era descubrirlo con los originales ya movidos dentro de la biblioteca
    // y sin índice que los apuntara: archivos desaparecidos de su carpeta y
    // que la ventana no enseñaba.
    let mut pack = lib.pack_writer()?;

    let prepared: Vec<std::result::Result<Option<Prepared>, (PathBuf, String)>> = candidates
        .par_iter()
        .map(|path| {
            let r = prepare_one(&rutas, path, opts, &known, &seen, &dup_count);
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            if n % 32 == 0 || n == total {
                progress(n, total);
            }
            match r {
                Ok(p) => Ok(p),
                Err(e) => Err((path.clone(), e.to_string())),
            }
        })
        .collect();

    // Todo lo que la fase paralela dejó escrito en `items/`, para poder
    // quitarlo si lo de abajo falla: también lo que el bucle no llegue a ver.
    let escritos: Vec<String> = prepared
        .iter()
        .filter_map(|r| r.as_ref().ok()?.as_ref().map(|p| p.item.id.clone()))
        .collect();

    // Serie: el pack es append-only y el índice, una sola transacción.
    let mut rows = Vec::with_capacity(prepared.len());
    let mut movidos: Vec<Movido> = Vec::new();
    let serie = (|| -> Result<()> {
        for r in prepared {
            match r {
                Ok(Some(p)) => {
                    if opts.mode == OriginMode::Move {
                        let dest = rutas.original(&p.item.id, &p.item.ext);
                        match mover(&p.item.id, &p.origen, &dest) {
                            Ok(m) => movidos.push(m),
                            Err(e) => {
                                // Este no entra, pero su carpeta ya estaba
                                // hecha: se quita para no dejar un
                                // `item.json` huérfano que un reindex
                                // resucitaría sin original.
                                rutas.quitar(&p.item.id);
                                report.failed.push((p.origen, e.to_string()));
                                continue;
                            }
                        }
                    }
                    // Sin miniatura no se toca el pack: es un archivo que solo
                    // crece, y meterle ceros bytes gastaría un hueco para siempre.
                    let tref = match &p.thumb {
                        Some(bytes) => Some(pack.append(&p.item.id, bytes)?),
                        None => None,
                    };
                    if let Some(m) = &p.motivo {
                        report.sin_miniatura += 1;
                        if !report.avisos.iter().any(|a| a == m) {
                            report.avisos.push(m.clone());
                        }
                    }
                    report.bytes += p.item.size;
                    report.imported += 1;
                    report.ids.push(p.item.id.clone());
                    rows.push((p.item, tref));
                }
                Ok(None) => {}
                Err(e) => report.failed.push(e),
            }
        }
        pack.flush()?;
        lib.index_mut()
            .upsert_many(rows.iter().map(|(i, t)| (i, *t)))
    })();
    drop(pack);

    if let Err(e) = serie {
        // Nada de esta importación llegó al índice. Los originales vuelven a
        // donde estaban y las carpetas que se crearon se quitan: que fallar
        // deje las cosas como antes de empezar, no a medias.
        //
        // Lo que no se pudo devolver se queda donde está, con su carpeta: es
        // el único sitio donde existe ahora ese archivo, y quitar la carpeta
        // sería borrarlo.
        let mut atrapados: HashSet<&str> = HashSet::new();
        for m in movidos.iter().rev() {
            if m.deshacer().is_err() {
                atrapados.insert(&m.id);
            }
        }
        for id in escritos.iter().filter(|id| !atrapados.contains(id.as_str())) {
            rutas.quitar(id);
        }
        let sin_volver = atrapados.len();
        if sin_volver > 0 {
            return Err(Error::Invalid(format!(
                "{e}\n  y {sin_volver} archivo{} no se pudo devolver a su sitio; \
                 está dentro de la biblioteca, en items/",
                if sin_volver == 1 { "" } else { "s" }
            )));
        }
        return Err(e);
    }

    // Ya está todo en el índice. Solo ahora se borra el original de lo que se
    // movió copiando entre discos: hasta aquí era la copia de seguridad.
    for m in &movidos {
        if let Err(e) = m.terminar() {
            let aviso = format!("no pude borrar el original de {}: {e}", m.origen.display());
            report.avisos.push(aviso);
        }
    }

    report.duplicates = dup_count.load(Ordering::Relaxed);
    report.elapsed_ms = started.elapsed().as_millis();
    Ok(report)
}

/// Un original que se movió dentro de la biblioteca, con lo necesario para
/// devolverlo si la importación falla después.
struct Movido {
    id: String,
    origen: PathBuf,
    destino: PathBuf,
    /// `true` si no se pudo renombrar —otro disco— y se copió. El origen
    /// sigue entonces en su sitio hasta [`Movido::terminar`].
    copiado: bool,
}

impl Movido {
    /// Lo devuelve a como estaba antes de importar.
    fn deshacer(&self) -> std::io::Result<()> {
        if self.copiado {
            std::fs::remove_file(&self.destino)
        } else {
            std::fs::rename(&self.destino, &self.origen)
        }
    }

    /// Cierra el movimiento cuando ya no hay vuelta atrás.
    fn terminar(&self) -> std::io::Result<()> {
        if self.copiado {
            std::fs::remove_file(&self.origen)
        } else {
            Ok(())
        }
    }
}

/// Mueve un original dentro de la biblioteca. Renombrar es instantáneo y no
/// duplica nada; si el original está en otro disco no se puede, y entonces se
/// copia y el original se deja hasta que el índice lo tenga apuntado.
fn mover(id: &str, origen: &Path, destino: &Path) -> Result<Movido> {
    if std::fs::rename(origen, destino).is_ok() {
        return Ok(Movido {
            id: id.to_string(),
            origen: origen.to_path_buf(),
            destino: destino.to_path_buf(),
            copiado: false,
        });
    }
    std::fs::copy(origen, destino).map_err(|e| Error::io(destino, e))?;
    Ok(Movido {
        id: id.to_string(),
        origen: origen.to_path_buf(),
        destino: destino.to_path_buf(),
        copiado: true,
    })
}

/// Lee lo que hace falta de un archivo y saca su blake3.
///
/// El hash sale siempre del archivo entero, pero leyéndolo a trozos: un vídeo
/// de cuatro gigas se hashea con unos pocos kilobytes de memoria. Solo se
/// carga en memoria lo que [`medio::que_leer`] dice que se va a decodificar;
/// si es el archivo entero, el hash sale de esos mismos bytes y el archivo se
/// lee una sola vez.
fn leer_y_hashear(path: &Path, ext: &str) -> Result<(String, Vec<u8>, u64)> {
    use std::io::Read;
    if medio::que_leer(ext) == medio::Lectura::Entero {
        let bytes = std::fs::read(path).map_err(|e| Error::io(path, e))?;
        let hash = blake3::hash(&bytes).to_hex().to_string();
        let size = bytes.len() as u64;
        return Ok((hash, bytes, size));
    }
    let f = std::fs::File::open(path).map_err(|e| Error::io(path, e))?;
    let mut h = blake3::Hasher::new();
    h.update_reader(&f).map_err(|e| Error::io(path, e))?;
    let hash = h.finalize().to_hex().to_string();
    let size = f.metadata().map_err(|e| Error::io(path, e))?.len();
    let bytes = match medio::que_leer(ext) {
        medio::Lectura::Principio(n) => {
            let mut b = Vec::new();
            std::fs::File::open(path)
                .and_then(|f| f.take(n as u64).read_to_end(&mut b))
                .map_err(|e| Error::io(path, e))?;
            b
        }
        _ => Vec::new(),
    };
    Ok((hash, bytes, size))
}

fn prepare_one(
    rutas: &Rutas,
    path: &Path,
    opts: &ImportOptions,
    known: &HashSet<String>,
    seen: &Mutex<HashSet<String>>,
    dup_count: &AtomicUsize,
) -> Result<Option<Prepared>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin")
        .to_ascii_lowercase();
    let (hash, bytes, size) = leer_y_hashear(path, &ext)?;

    if opts.skip_duplicates {
        if known.contains(&hash) {
            dup_count.fetch_add(1, Ordering::Relaxed);
            return Ok(None);
        }
        let mut s = seen.lock().unwrap();
        if !s.insert(hash.clone()) {
            dup_count.fetch_add(1, Ordering::Relaxed);
            return Ok(None);
        }
    }

    let lienzo = medio::abrir(path, &ext, &bytes)?;
    // Lo leído ya no hace falta: con un modelo de cientos de megas, soltarlo
    // aquí y no al final de la función es memoria que otro hilo puede usar
    // mientras este codifica miniaturas.
    drop(bytes);
    let (w, h) = (lienzo.ancho, lienzo.alto);
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("sin nombre")
        .to_string();

    let id = crate::id::new_id();
    let mut item = Item::new(id.clone(), name, ext.clone(), size, opts.mode);
    item.hash = hash;
    item.width = w;
    item.height = h;
    item.duration_ms = lienzo.duracion_ms;
    item.pages = lienzo.paginas;
    item.triangles = lienzo.triangulos;
    item.size_mm = lienzo.medidas_mm;
    if let Some(img) = &lienzo.imagen {
        // La huella perceptual y la paleta salen de lo que se ve. Sin imagen no
        // hay ninguna de las dos, y es correcto: un vídeo sin fotograma no se
        // parece a nada todavía.
        item.phash = format!("{:016x}", image_ops::dhash(img));
        item.palette = image_ops::palette(img, 5);
    }
    item.tags = opts.tags.clone();
    item.source = opts.source.clone();
    if let Some(f) = &opts.folder {
        item.folders = vec![f.clone()];
    }
    item.origin = Origin {
        mode: opts.mode,
        path: Some(path.to_string_lossy().to_string()),
        inode: inode_of(path),
    };

    // Miniaturas antes de tocar disco: si la imagen es basura, no dejamos rastro.
    let grid = match &lienzo.imagen {
        Some(img) => Some(image_ops::encode_jpeg(
            &image_ops::make_thumb(img, THUMB_GRID),
            82,
        )?),
        None => None,
    };

    // Lo que se escribe en la biblioteca va junto: si cualquier paso falla,
    // la carpeta del elemento y su previsualización se quitan y el archivo
    // sale en el informe sin dejar restos que un reindex resucitaría.
    let escribir = || -> Result<()> {
        let dir = rutas.item_dir(&id);
        std::fs::create_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
        match opts.mode {
            OriginMode::Copy => {
                // `fs::copy` y no escribir los bytes: el sistema copia sin
                // pasar el archivo por la memoria del proceso.
                let dest = rutas.original(&id, &ext);
                std::fs::copy(path, &dest).map_err(|e| Error::io(&dest, e))?;
            }
            // Mover se deja para la fase en serie. Aquí el original no se
            // toca: si algo falla antes de llegar al índice, sigue donde estaba.
            OriginMode::Move | OriginMode::Ref => {}
        }
        if opts.previews {
            if let Some(img) = &lienzo.imagen {
                let preview =
                    image_ops::encode_jpeg(&image_ops::make_thumb(img, THUMB_PREVIEW), 86)?;
                let p = rutas.preview(&id);
                let padre = p.parent().unwrap();
                std::fs::create_dir_all(padre).map_err(|e| Error::io(padre, e))?;
                std::fs::write(&p, &preview).map_err(|e| Error::io(&p, e))?;
            }
        }
        item.write(&rutas.item_json(&id))
    };
    if let Err(e) = escribir() {
        rutas.quitar(&id);
        return Err(e);
    }
    Ok(Some(Prepared {
        item,
        origen: path.to_path_buf(),
        thumb: grid,
        motivo: lienzo.sin_imagen,
    }))
}

/// Vuelve a sacar la cara de lo ya importado: miniatura, previsualización,
/// paleta, huella y los datos que salen de abrir el archivo (medidas,
/// duración, páginas, triángulos).
///
/// Sirve para cuando el núcleo aprende a dibujar algo mejor —la onda de los
/// sonidos, los modelos 3D— y lo que ya estaba en la biblioteca se quedaría
/// con la cara vieja para siempre. No toca nada de lo que pone una persona:
/// etiquetas, estrellas, notas, carpetas y nombre se quedan como estaban.
///
/// Las miniaturas viejas se quedan en el pack sin que nadie las apunte; el
/// pack solo crece. Es el precio de no reescribirlo entero, y se recupera
/// con `reindex` sobre una copia limpia si algún día importa.
pub fn rehacer_miniaturas(
    lib: &mut Library,
    familia: Option<medio::Familia>,
    progreso: &(dyn Fn(usize, usize) + Sync),
) -> Result<(usize, Vec<(String, String)>)> {
    let mut q = crate::query::Query {
        limit: usize::MAX / 2,
        ..Default::default()
    };
    if let Some(f) = familia {
        q.familias = vec![f];
    }
    let mut ids: Vec<String> = lib.index().search(&q)?.into_iter().map(|h| h.id).collect();
    // También lo que está en la papelera: si se recupera, que vuelva con cara.
    q.papelera = true;
    ids.extend(lib.index().search(&q)?.into_iter().map(|h| h.id));

    let total = ids.len();
    let mut pack = lib.pack_writer()?;
    let mut hechos = 0;
    let mut fallos = Vec::new();
    for (k, id) in ids.iter().enumerate() {
        progreso(k, total);
        let mut item = lib.load_item(id)?;
        let ruta = lib.original_path(&item);
        let bytes = match std::fs::read(&ruta) {
            Ok(b) => b,
            Err(e) => {
                fallos.push((item.name.clone(), e.to_string()));
                continue;
            }
        };
        let lienzo = match medio::abrir(&ruta, &item.ext, &bytes) {
            Ok(l) => l,
            Err(e) => {
                fallos.push((item.name.clone(), e.to_string()));
                continue;
            }
        };
        let Some(img) = &lienzo.imagen else {
            fallos.push((
                item.name.clone(),
                lienzo.sin_imagen.unwrap_or_else(|| "sin imagen".into()),
            ));
            continue;
        };
        item.width = lienzo.ancho;
        item.height = lienzo.alto;
        item.duration_ms = lienzo.duracion_ms.or(item.duration_ms);
        item.pages = lienzo.paginas.or(item.pages);
        item.triangles = lienzo.triangulos.or(item.triangles);
        item.size_mm = lienzo.medidas_mm.or(item.size_mm);
        item.phash = format!("{:016x}", image_ops::dhash(img));
        item.palette = image_ops::palette(img, 5);

        let grid = image_ops::encode_jpeg(&image_ops::make_thumb(img, THUMB_GRID), 82)?;
        let referencia = pack.append(&item.id, &grid)?;
        let preview = image_ops::encode_jpeg(&image_ops::make_thumb(img, THUMB_PREVIEW), 86)?;
        let p = lib.preview_path(&item.id);
        std::fs::create_dir_all(p.parent().unwrap())
            .map_err(|e| Error::io(p.parent().unwrap(), e))?;
        std::fs::write(&p, &preview).map_err(|e| Error::io(&p, e))?;

        item.write(&lib.item_json(&item.id))?;
        pack.flush()?;
        lib.index_mut().upsert(&item, Some(referencia))?;
        hechos += 1;
    }
    progreso(total, total);
    Ok((hechos, fallos))
}

fn inode_of(path: &Path) -> Option<u64> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        std::fs::metadata(path).ok().map(|m| m.ino())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    fn imagen_de_prueba(path: &Path, w: u32, h: u32, tinte: u8) {
        let mut im = RgbImage::new(w, h);
        for (x, y, p) in im.enumerate_pixels_mut() {
            *p = Rgb([(x % 256) as u8, (y % 256) as u8, tinte]);
        }
        im.save(path).unwrap();
    }

    #[test]
    fn importa_copiando_y_deja_todo_en_su_sitio() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("origen");
        std::fs::create_dir_all(&src).unwrap();
        for i in 0..5 {
            imagen_de_prueba(&src.join(format!("f{i}.png")), 200, 100, i as u8 * 10);
        }

        let mut lib = Library::create(&dir.path().join("x.grimorio"), "x").unwrap();
        let rep = import(
            &mut lib,
            &[src.clone()],
            &ImportOptions::default(),
            &|_, _| {},
        )
        .unwrap();

        assert_eq!(rep.imported, 5);
        assert!(rep.failed.is_empty());
        assert_eq!(lib.index().count().unwrap(), 5);

        // Los originales siguen donde estaban y hay copia dentro.
        assert!(src.join("f0.png").exists());
        let hits = lib.index().search(&crate::Query::default()).unwrap();
        let item = lib.load_item(&hits[0].id).unwrap();
        assert!(lib.original_path(&item).exists());
        assert!(lib.preview_path(&item.id).exists());
        assert_eq!(item.width, 200);
        assert_eq!(item.height, 100);
        assert!(!item.hash.is_empty());
        assert!(!item.palette.is_empty());
    }

    #[test]
    fn los_duplicados_por_contenido_se_saltan() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("origen");
        std::fs::create_dir_all(&src).unwrap();
        imagen_de_prueba(&src.join("a.png"), 64, 64, 3);
        std::fs::copy(src.join("a.png"), src.join("copia.png")).unwrap();

        let mut lib = Library::create(&dir.path().join("x.grimorio"), "x").unwrap();
        let rep = import(
            &mut lib,
            &[src.clone()],
            &ImportOptions::default(),
            &|_, _| {},
        )
        .unwrap();
        assert_eq!(rep.imported, 1);
        assert_eq!(rep.duplicates, 1);

        // Reimportar la misma carpeta no duplica nada.
        let rep2 = import(&mut lib, &[src], &ImportOptions::default(), &|_, _| {}).unwrap();
        assert_eq!(rep2.imported, 0);
        assert_eq!(lib.index().count().unwrap(), 1);
    }

    #[test]
    fn un_archivo_corrupto_no_tumba_la_importacion() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("origen");
        std::fs::create_dir_all(&src).unwrap();
        imagen_de_prueba(&src.join("buena.png"), 32, 32, 1);
        std::fs::write(src.join("mala.png"), b"esto no es un png").unwrap();

        let mut lib = Library::create(&dir.path().join("x.grimorio"), "x").unwrap();
        let rep = import(&mut lib, &[src], &ImportOptions::default(), &|_, _| {}).unwrap();
        assert_eq!(rep.imported, 1);
        assert_eq!(rep.failed.len(), 1);
        assert!(rep.failed[0].0.ends_with("mala.png"));
    }

    #[test]
    fn el_indice_se_reconstruye_desde_disco() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("origen");
        std::fs::create_dir_all(&src).unwrap();
        for i in 0..7 {
            imagen_de_prueba(&src.join(format!("f{i}.png")), 80, 60, i as u8 * 7);
        }
        let root = dir.path().join("x.grimorio");
        let mut lib = Library::create(&root, "x").unwrap();
        import(&mut lib, &[src], &ImportOptions::default(), &|_, _| {}).unwrap();
        let antes = lib.index().count().unwrap();
        let hits_antes = lib.index().search(&crate::Query::default()).unwrap();
        drop(lib);

        // Borramos el índice entero, como si se hubiera corrompido.
        std::fs::remove_file(root.join("index.sqlite")).unwrap();
        let mut lib = Library::open(&root).unwrap();
        let vistos = lib.reindex().unwrap().vistos;

        assert_eq!(vistos, antes);
        let hits = lib.index().search(&crate::Query::default()).unwrap();
        assert_eq!(hits.len(), hits_antes.len());
        // Y las miniaturas se recuperan del pack, no hay que regenerarlas.
        assert!(hits.iter().all(|h| h.thumb_off.is_some()));
    }

    #[test]
    fn modo_referencia_no_copia_el_original() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("origen");
        std::fs::create_dir_all(&src).unwrap();
        imagen_de_prueba(&src.join("a.png"), 40, 40, 9);

        let mut lib = Library::create(&dir.path().join("x.grimorio"), "x").unwrap();
        let opts = ImportOptions {
            mode: OriginMode::Ref,
            ..Default::default()
        };
        import(&mut lib, &[src.clone()], &opts, &|_, _| {}).unwrap();

        let hits = lib.index().search(&crate::Query::default()).unwrap();
        let item = lib.load_item(&hits[0].id).unwrap();
        assert_eq!(item.origin.mode, OriginMode::Ref);
        assert_eq!(lib.original_path(&item), src.join("a.png"));
        assert!(!lib.item_dir(&item.id).join("original.png").exists());
    }
}

#[cfg(test)]
mod tests_mezcla {
    use super::*;
    use crate::medio::Familia;
    use crate::query::Query;
    use image::{Rgb, RgbImage};

    /// Rehacer la cara no puede tocar lo que puso una persona, y solo rehace
    /// la familia que se pide.
    #[test]
    fn rehacer_miniaturas_conserva_lo_que_puso_una_persona() {
        let dir = tempfile::tempdir().unwrap();
        let stl = dir.path().join("cubo.stl");
        std::fs::write(&stl, crate::modelo3d::tests::cubo_stl()).unwrap();
        let foto = dir.path().join("foto.png");
        RgbImage::from_pixel(30, 20, Rgb([200, 40, 40]))
            .save(&foto)
            .unwrap();

        let raiz = dir.path().join("r.grimorio");
        let mut lib = Library::create(&raiz, "r").unwrap();
        let informe = import(
            &mut lib,
            &[stl, foto],
            &ImportOptions::default(),
            &|_, _| {},
        )
        .unwrap();
        assert_eq!(informe.ids.len(), 2);

        let q = Query {
            familias: vec![Familia::Modelo],
            ..Default::default()
        };
        let id = lib.index().search(&q).unwrap()[0].id.clone();
        let mut it = lib.load_item(&id).unwrap();
        it.tags = vec!["pieza".into()];
        it.stars = 4;
        lib.save_item(&it).unwrap();
        let pack_antes = std::fs::metadata(lib.pack_path()).unwrap().len();

        let (hechos, fallos) =
            rehacer_miniaturas(&mut lib, Some(Familia::Modelo), &|_, _| {}).unwrap();
        assert_eq!((hechos, fallos.len()), (1, 0), "solo el modelo, no la foto");

        let it = lib.load_item(&id).unwrap();
        assert_eq!(it.tags, vec!["pieza".to_string()]);
        assert_eq!(it.stars, 4);
        assert_eq!(it.triangles, Some(12));
        assert!(std::fs::metadata(lib.pack_path()).unwrap().len() > pack_antes);
        assert!(lib.preview_path(&id).exists());
    }

    /// La puerta de M3 en pequeño: una carpeta con de todo dentro tiene que
    /// entrar entera, sin perder un elemento y sin colar la basura.
    #[test]
    fn una_carpeta_con_de_todo_entra_entera() {
        let dir = tempfile::tempdir().unwrap();
        let fuente = dir.path().join("mezcla");
        std::fs::create_dir_all(&fuente).unwrap();

        let mut im = RgbImage::new(40, 20);
        for (x, _, p) in im.enumerate_pixels_mut() {
            *p = Rgb([(x * 6) as u8, 30, 90]);
        }
        im.save(fuente.join("foto.png")).unwrap();
        // Los demás no son archivos de verdad y da igual: lo que se comprueba
        // es que entran y no se pierden, no que sepamos dibujarlos.
        std::fs::write(fuente.join("clip.mp4"), b"no soy un mp4").unwrap();
        std::fs::write(fuente.join("manual.pdf"), b"%PDF-1.4 mentira").unwrap();
        std::fs::write(fuente.join("Rotulo.otf"), b"OTTO...").unwrap();
        std::fs::write(fuente.join("compilado.o"), b"\x7fELF").unwrap();
        std::fs::write(fuente.join("notas.log"), b"nada que ver").unwrap();

        let raiz = dir.path().join("p.grimorio");
        let mut lib = Library::create(&raiz, "p").unwrap();
        let informe = import(
            &mut lib,
            &[fuente.clone()],
            &ImportOptions::default(),
            &|_, _| {},
        )
        .unwrap();

        assert_eq!(
            informe.imported, 4,
            "la imagen, el vídeo, el PDF y la fuente"
        );
        assert_eq!(informe.sin_miniatura, 3, "todos menos la imagen");
        assert_eq!(informe.unsupported, 2, "el .o y el .log se quedan fuera");
        assert!(informe.failed.is_empty());

        let todos = Query {
            limit: 0,
            ..Default::default()
        };
        assert_eq!(lib.index().search(&todos).unwrap().len(), 4);

        // Y cada uno sabe lo que es, así que se puede filtrar por familia.
        let solo = |f: Familia| Query {
            familias: vec![f],
            limit: 0,
            ..Default::default()
        };
        assert_eq!(lib.index().search(&solo(Familia::Video)).unwrap().len(), 1);
        assert_eq!(
            lib.index().search(&solo(Familia::Documento)).unwrap().len(),
            1
        );
        assert_eq!(
            lib.index()
                .search(&solo(Familia::Tipografia))
                .unwrap()
                .len(),
            1
        );
        assert_eq!(lib.index().search(&solo(Familia::Imagen)).unwrap().len(), 1);

        // Solo la imagen tiene miniatura; los demás enseñan su color de
        // marcador hasta que sepamos dibujarlos.
        let con_thumb = lib
            .index()
            .search(&todos)
            .unwrap()
            .iter()
            .filter(|h| h.thumb_off.is_some())
            .count();
        assert_eq!(con_thumb, 1);
    }

    #[test]
    fn una_imagen_rota_sale_en_el_informe_y_no_para_a_las_demas() {
        let dir = tempfile::tempdir().unwrap();
        let fuente = dir.path().join("rotas");
        std::fs::create_dir_all(&fuente).unwrap();
        let mut im = RgbImage::new(10, 10);
        im.put_pixel(0, 0, Rgb([1, 2, 3]));
        im.save(fuente.join("buena.png")).unwrap();
        std::fs::write(fuente.join("rota.png"), b"esto no es un png").unwrap();

        let raiz = dir.path().join("p.grimorio");
        let mut lib = Library::create(&raiz, "p").unwrap();
        let informe = import(&mut lib, &[fuente], &ImportOptions::default(), &|_, _| {}).unwrap();
        assert_eq!(informe.imported, 1);
        assert_eq!(informe.failed.len(), 1);
        assert!(informe.failed[0].0.ends_with("rota.png"));
    }
}

#[cfg(test)]
mod tests_mover {
    use super::*;
    use crate::query::Query;
    use crate::registro::{Operacion, Parche};
    use image::{Rgb, RgbImage};

    fn fotos(dir: &Path, n: usize) -> Vec<PathBuf> {
        std::fs::create_dir_all(dir).unwrap();
        (0..n)
            .map(|i| {
                let p = dir.join(format!("f{i}.png"));
                RgbImage::from_pixel(16, 12, Rgb([i as u8 * 40, 10, 200]))
                    .save(&p)
                    .unwrap();
                p
            })
            .collect()
    }

    fn mover() -> ImportOptions {
        ImportOptions {
            mode: OriginMode::Move,
            previews: false,
            ..Default::default()
        }
    }

    fn nada(_: usize, _: usize) {}

    /// Mover: el original desaparece de su carpeta y queda dentro. Deshacer
    /// la importación —como la deshace la ventana— lo manda a la papelera sin
    /// perderlo, y rehacer lo devuelve.
    #[test]
    fn mover_se_lleva_el_original_y_deshacer_no_lo_pierde() {
        let dir = tempfile::tempdir().unwrap();
        let fuera = fotos(&dir.path().join("origen"), 3);
        let mut lib = Library::create(&dir.path().join("x.grimorio"), "x").unwrap();
        let rep = import(&mut lib, &[dir.path().join("origen")], &mover(), &nada).unwrap();
        assert_eq!(rep.imported, 3);
        assert!(rep.failed.is_empty());
        assert!(fuera.iter().all(|p| !p.exists()), "los originales se fueron");
        for id in &rep.ids {
            let item = lib.load_item(id).unwrap();
            assert_eq!(item.origin.mode, OriginMode::Move);
            assert!(lib.original_path(&item).exists(), "y están dentro");
        }

        // Lo que apunta la ventana al importar (app/puente, `importar`).
        let mut op = Operacion::nueva("importar 3 elementos");
        for id in &rep.ids {
            op.antes.push((id.clone(), Parche::papelera(true)));
            op.despues.push((id.clone(), Parche::papelera(false)));
        }
        lib.anotar(op);

        lib.deshacer(&nada).unwrap();
        assert_eq!(lib.index().count().unwrap(), 0);
        let papelera = Query {
            papelera: true,
            limit: 0,
            ..Default::default()
        };
        assert_eq!(lib.index().search(&papelera).unwrap().len(), 3);
        for id in &rep.ids {
            let item = lib.load_item(id).unwrap();
            assert!(lib.original_path(&item).exists(), "deshacer no borra nada");
        }

        lib.rehacer(&nada).unwrap();
        assert_eq!(lib.index().count().unwrap(), 3);
    }

    /// Con el pack ocupado por otro escritor la importación no puede
    /// terminar, y entonces no puede haber movido nada.
    #[test]
    fn con_el_pack_ocupado_no_se_mueve_nada() {
        let dir = tempfile::tempdir().unwrap();
        let fuera = fotos(&dir.path().join("origen"), 3);
        let mut lib = Library::create(&dir.path().join("x.grimorio"), "x").unwrap();
        let otro = lib.pack_writer().unwrap();

        let r = import(&mut lib, &[dir.path().join("origen")], &mover(), &nada);
        assert!(r.is_err(), "el pack estaba ocupado");
        drop(otro);

        assert!(fuera.iter().all(|p| p.exists()), "ningún original se movió");
        assert_eq!(lib.index().count().unwrap(), 0);
        // Y no quedaron carpetas a medias que un reindex resucitaría.
        assert_eq!(lib.reindex().unwrap().vistos, 0);
    }

    /// Si falla el índice después de haber movido, los originales vuelven a
    /// su sitio y no queda nada a medias dentro de la biblioteca.
    #[test]
    fn si_el_indice_falla_los_originales_vuelven() {
        let dir = tempfile::tempdir().unwrap();
        let fuera = fotos(&dir.path().join("origen"), 2);
        let mut lib = Library::create(&dir.path().join("x.grimorio"), "x").unwrap();
        lib.index()
            .conn
            .execute_batch(
                "CREATE TRIGGER romper BEFORE INSERT ON items
                 BEGIN SELECT RAISE(ABORT, 'roto a propósito'); END;",
            )
            .unwrap();
        let r = import(&mut lib, &[dir.path().join("origen")], &mover(), &nada);
        assert!(r.is_err());
        assert!(fuera.iter().all(|p| p.exists()), "volvieron a su carpeta");
        lib.index().conn.execute_batch("DROP TRIGGER romper").unwrap();
        assert_eq!(lib.reindex().unwrap().vistos, 0, "sin restos en items/");
    }

    /// Lo que no se decodifica aquí no se carga en memoria, pero se hashea
    /// entero: dos vídeos que solo difieren al final no son el mismo.
    #[test]
    fn el_hash_cubre_el_archivo_entero_aunque_no_se_lea() {
        assert_eq!(medio::que_leer("mp4"), medio::Lectura::Nada);
        assert_eq!(medio::que_leer("PNG"), medio::Lectura::Entero);
        assert!(matches!(medio::que_leer("flac"), medio::Lectura::Principio(_)));

        let dir = tempfile::tempdir().unwrap();
        let mut a = vec![7u8; 300_000];
        let (ha, ba, ta) = {
            std::fs::write(dir.path().join("a.mp4"), &a).unwrap();
            leer_y_hashear(&dir.path().join("a.mp4"), "mp4").unwrap()
        };
        *a.last_mut().unwrap() = 8;
        std::fs::write(dir.path().join("b.mp4"), &a).unwrap();
        let (hb, _, _) = leer_y_hashear(&dir.path().join("b.mp4"), "mp4").unwrap();
        assert_ne!(ha, hb);
        assert!(ba.is_empty(), "el vídeo no se carga");
        assert_eq!(ta, 300_000);
        assert_eq!(ha, blake3::hash(&vec![7u8; 300_000]).to_hex().to_string());
    }
}
