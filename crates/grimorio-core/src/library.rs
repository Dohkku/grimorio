//! La biblioteca: una carpeta en disco que se entiende sin la aplicación.
//!
//! ```text
//! MiBiblioteca.grimorio/
//! ├── library.json      identidad y ajustes
//! ├── index.sqlite      ÍNDICE DERIVADO, borrable
//! ├── items/01/JK/<id>/ original + item.json
//! ├── thumbs/grid.pack  miniaturas de malla concatenadas
//! ├── thumbs/preview/   previsualizaciones 1024 px
//! ├── trash/            borrado reversible
//! ├── themes/  plugins/
//! ```

use crate::error::{Error, Result};
use crate::folder::Folders;
use crate::index::Index;
use crate::item::{shard_dir, write_atomic, Item, OriginMode};
use crate::registro::{Operacion, Parche, Registro};
use crate::thumbs::{PackReader, PackWriter};
use crate::{time, SCHEMA_VERSION};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const LIBRARY_FILE: &str = "library.json";
/// El cerrojo que pone la aplicación mientras tiene la biblioteca abierta
/// (`QLockFile` en `app/src/main.cpp`).
pub const CERROJO_APP: &str = "abierta.lock";

/// Si la aplicación tiene abierta la biblioteca de `root`, el pid que la tiene.
///
/// Los JSON de la biblioteca —`folders.json`, las búsquedas guardadas, los
/// grupos, las carpetas vigiladas— se escriben enteros cada vez, sin cerrojo:
/// si la ventana y `grim` cambian uno a la vez, el último en guardar pisa al
/// otro sin decir nada. La aplicación ya se protege de otra ventana con
/// `abierta.lock`; esto deja que la terminal mire el mismo cerrojo antes de
/// escribir, en vez de inventar un segundo cerrojo que la aplicación no vería.
///
/// El formato es el de `QLockFile`: el pid en la primera línea y el nombre de
/// la máquina en la tercera. Un cerrojo cuyo proceso ya no existe es de un
/// cierre a lo bruto y no cuenta, igual que no cuenta para la propia
/// aplicación. Uno de otra máquina —una biblioteca en un disco compartido— no
/// se puede comprobar y se da por bueno: mejor negarse de más que pisar.
pub fn abierta_por_la_app(root: &Path) -> Option<u32> {
    let texto = std::fs::read_to_string(root.join(CERROJO_APP)).ok()?;
    let mut lineas = texto.lines();
    let pid: u32 = lineas.next()?.trim().parse().ok()?;
    let maquina = lineas.nth(1).map(str::trim).unwrap_or("");
    if !maquina.is_empty() && maquina != nombre_de_maquina().as_deref().unwrap_or(maquina) {
        return Some(pid);
    }
    proceso_vivo(pid).then_some(pid)
}

#[cfg(unix)]
fn proceso_vivo(pid: u32) -> bool {
    if pid == 0 || pid > i32::MAX as u32 {
        return false;
    }
    // La señal 0 no se envía: solo pregunta si el proceso existe. EPERM quiere
    // decir que existe y es de otro usuario, que para esto es «vivo».
    let r = unsafe { libc::kill(pid as libc::pid_t, 0) };
    r == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// Sin una forma sencilla de preguntar por un pid, se da por vivo. Un
/// cerrojo viejo se quita borrando `abierta.lock` con la aplicación cerrada.
#[cfg(not(unix))]
fn proceso_vivo(pid: u32) -> bool {
    pid != 0
}

#[cfg(unix)]
fn nombre_de_maquina() -> Option<String> {
    let mut buf = [0u8; 256];
    let r = unsafe { libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()) };
    if r != 0 {
        return None;
    }
    let fin = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
    Some(String::from_utf8_lossy(&buf[..fin]).into_owned())
}

#[cfg(not(unix))]
fn nombre_de_maquina() -> Option<String> {
    std::env::var("COMPUTERNAME").ok()
}
pub const EXTENSION: &str = "grimorio";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryMeta {
    pub schema: u32,
    pub id: String,
    pub name: String,
    pub created_at: String,
    /// Qué hace la importación con tus archivos. Por defecto: copiar dentro.
    #[serde(default = "default_mode")]
    pub import_mode: OriginMode,
    #[serde(default)]
    pub app_version: String,
}

fn default_mode() -> OriginMode {
    OriginMode::Copy
}

/// Dónde vive la previsualización de 1024 px de un elemento.
///
/// Es una función libre y no solo un método porque el puente la necesita sin
/// tener una `Library` a mano, y tener el reparto de carpetas escrito en dos
/// sitios es la forma más tonta de que un día dejen de coincidir.
pub fn preview_path(root: &Path, id: &str) -> PathBuf {
    root.join("thumbs")
        .join("preview")
        .join(id.get(0..2).unwrap_or("__"))
        .join(format!("{id}.jpg"))
}

pub struct Library {
    root: PathBuf,
    meta: LibraryMeta,
    index: Index,
    folders: Folders,
    registro: Registro,
}

/// Lo que hizo [`Library::aplicar_lote`]: lo aplicado y, si lo hubo, el
/// error que lo paró.
struct Lote {
    op: Operacion,
    ilegibles: usize,
    error: Option<Error>,
}

/// Lo que dejó vaciar la papelera.
#[derive(Debug, Default)]
pub struct Vaciado {
    /// Cuántos se borraron de verdad, del disco y del índice.
    pub borrados: usize,
    /// Los que siguen en la papelera, con el motivo.
    pub fallos: Vec<(String, String)>,
}

/// Si `ruta` cuelga de `base` sin salirse por el camino.
///
/// `Path::starts_with` compara componentes, no rutas resueltas: para él
/// `items/../../x` empieza por `items`. Por eso además se exige que lo que
/// viene después sean nombres normales, sin `..` ni raíces.
fn dentro_de(ruta: &Path, base: &Path) -> bool {
    use std::path::Component;
    match ruta.strip_prefix(base) {
        Ok(resto) => {
            resto.components().next().is_some()
                && resto
                    .components()
                    .all(|c| matches!(c, Component::Normal(_)))
        }
        Err(_) => false,
    }
}

impl std::fmt::Debug for Library {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Library")
            .field("root", &self.root)
            .field("name", &self.meta.name)
            .finish_non_exhaustive()
    }
}

impl Library {
    pub fn create(root: &Path, name: &str) -> Result<Library> {
        if root.join(LIBRARY_FILE).exists() {
            return Err(Error::AlreadyExists(root.to_path_buf()));
        }
        for sub in ["items", "thumbs/preview", "trash", "themes", "plugins"] {
            let d = root.join(sub);
            std::fs::create_dir_all(&d).map_err(|e| Error::io(&d, e))?;
        }
        let meta = LibraryMeta {
            schema: SCHEMA_VERSION,
            id: crate::id::new_id(),
            name: name.to_string(),
            created_at: time::to_rfc3339(time::now_ms()),
            import_mode: OriginMode::Copy,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
        };
        let path = root.join(LIBRARY_FILE);
        let json = serde_json::to_vec_pretty(&meta).map_err(|e| Error::json(&path, e))?;
        write_atomic(&path, &json)?;

        let index = Index::open(&root.join("index.sqlite"))?;
        index.set_meta("library_id", &meta.id)?;
        let folders = Folders::default();
        folders.guardar(root)?;
        Ok(Library {
            root: root.to_path_buf(),
            meta,
            index,
            folders,
            registro: Registro::default(),
        })
    }

    pub fn open(root: &Path) -> Result<Library> {
        let path = root.join(LIBRARY_FILE);
        if !path.exists() {
            return Err(Error::NotALibrary(root.to_path_buf()));
        }
        let bytes = std::fs::read(&path).map_err(|e| Error::io(&path, e))?;
        let meta: LibraryMeta =
            serde_json::from_slice(&bytes).map_err(|e| Error::json(&path, e))?;
        if meta.schema > SCHEMA_VERSION {
            return Err(Error::SchemaTooNew {
                found: meta.schema,
                supported: SCHEMA_VERSION,
            });
        }
        let mut index = Index::open(&root.join("index.sqlite"))?;
        // `folders.json` es la verdad; la tabla del índice es una copia para
        // poder filtrar en SQL. Se rehace al abrir porque es barata —son
        // decenas de filas— y así una edición a mano del archivo se nota.
        let folders = Folders::cargar(root)?;
        index.replace_folders(&folders.folders)?;
        Ok(Library {
            root: root.to_path_buf(),
            meta,
            index,
            folders,
            registro: Registro::default(),
        })
    }

    /// Abre si existe, crea si no. Lo que hace `grim init` sin sorpresas.
    pub fn open_or_create(root: &Path, name: &str) -> Result<Library> {
        if root.join(LIBRARY_FILE).exists() {
            Library::open(root)
        } else {
            std::fs::create_dir_all(root).map_err(|e| Error::io(root, e))?;
            Library::create(root, name)
        }
    }

    /// Cambia el nombre que se enseña. Solo el nombre: la carpeta no se mueve,
    /// porque su ruta está en las recientes, en los accesos directos y en lo
    /// que otros programas tengan apuntado.
    ///
    /// En `library.json` se toca `name` y nada más, sin reescribirlo desde
    /// `LibraryMeta`: lo que escriba una versión más nueva y esta no conozca
    /// tiene que seguir ahí.
    pub fn rename(&mut self, name: &str) -> Result<()> {
        let nombre = name.trim();
        if nombre.is_empty() {
            return Err(Error::Invalid("el nombre de la biblioteca no puede quedar vacío".into()));
        }
        let path = self.root.join(LIBRARY_FILE);
        let bytes = std::fs::read(&path).map_err(|e| Error::io(&path, e))?;
        let mut valor: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|e| Error::json(&path, e))?;
        let Some(objeto) = valor.as_object_mut() else {
            return Err(Error::Invalid(format!("{} no es un objeto", path.display())));
        };
        objeto.insert("name".into(), serde_json::Value::String(nombre.to_string()));
        let json = serde_json::to_vec_pretty(&valor).map_err(|e| Error::json(&path, e))?;
        write_atomic(&path, &json)?;
        self.meta.name = nombre.to_string();
        Ok(())
    }

    // ------------------------------------------------------------------ rutas

    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn meta(&self) -> &LibraryMeta {
        &self.meta
    }
    pub fn index(&self) -> &Index {
        &self.index
    }
    pub fn index_mut(&mut self) -> &mut Index {
        &mut self.index
    }
    pub fn items_dir(&self) -> PathBuf {
        self.root.join("items")
    }
    pub fn item_dir(&self, id: &str) -> PathBuf {
        shard_dir(&self.items_dir(), id)
    }
    pub fn item_json(&self, id: &str) -> PathBuf {
        self.item_dir(id).join("item.json")
    }
    pub fn original_path(&self, item: &Item) -> PathBuf {
        match item.origin.mode {
            OriginMode::Ref => item
                .origin
                .path
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    self.item_dir(&item.id)
                        .join(format!("original.{}", item.ext))
                }),
            _ => self
                .item_dir(&item.id)
                .join(format!("original.{}", item.ext)),
        }
    }
    pub fn pack_path(&self) -> PathBuf {
        self.root.join("thumbs").join("grid.pack")
    }
    pub fn preview_path(&self, id: &str) -> PathBuf {
        preview_path(&self.root, id)
    }
    pub fn trash_dir(&self) -> PathBuf {
        self.root.join("trash")
    }

    pub fn pack_writer(&self) -> Result<PackWriter> {
        PackWriter::open(&self.pack_path())
    }
    pub fn pack_reader(&self) -> Result<PackReader> {
        PackReader::open(&self.pack_path())
    }

    // ----------------------------------------------------------------- items

    pub fn load_item(&self, id: &str) -> Result<Item> {
        Item::read(&self.item_json(id))
    }

    /// Escribe el JSON (verdad) y actualiza el índice (derivado), en ese orden.
    /// Si el proceso muere entre medias, `reindex` lo recupera.
    ///
    /// No hace falta buscarle la miniatura: pasar `None` la conserva, porque el
    /// `upsert` usa `COALESCE`. Preguntarla antes era un `SELECT` por elemento
    /// que no cambiaba nada.
    pub fn save_item(&mut self, item: &Item) -> Result<()> {
        item.write(&self.item_json(&item.id))?;
        self.index.upsert(item, None)
    }

    // --------------------------------------------------------------- edición

    /// Cuántos elementos se cargan, cambian y guardan por tanda.
    ///
    /// El número tiene consecuencias medibles: cada tanda es **una**
    /// transacción de SQLite. Guardar de uno en uno convertía «etiquetar diez
    /// mil» en diez mil transacciones, que es de donde salían los segundos.
    /// Quinientos doce es lo bastante grande para que la transacción compense y
    /// lo bastante pequeño para que el progreso se mueva.
    const LOTE: usize = 512;

    pub fn registro(&self) -> &Registro {
        &self.registro
    }

    /// El título de lo que se desharía, para poder ponerlo en el menú.
    pub fn que_se_deshace(&self) -> Option<&str> {
        self.registro.ultima().map(|o| o.titulo.as_str())
    }

    pub fn que_se_rehace(&self) -> Option<&str> {
        self.registro.ultima_deshecha().map(|o| o.titulo.as_str())
    }

    /// El motor de toda edición: aplica un parche por elemento, en tandas.
    ///
    /// Carga y escribe en paralelo —son muchos archivos pequeños, justo lo que
    /// un disco moderno hace mejor a la vez que en fila— y mete cada tanda en
    /// el índice de una sola vez.
    ///
    /// Un elemento cuyo `item.json` no se pueda leer **no aborta el lote**: se
    /// cuenta aparte y se sigue. Con diez mil elementos, dejar la mitad a
    /// medio hacer por un archivo roto es peor que terminar y avisar.
    ///
    /// Devuelve la operación —con su camino de vuelta ya calculado—, cuántos
    /// elementos no se pudieron leer y, si algo falló al escribir, el error.
    ///
    /// El error va **dentro** y no como `Err` por una razón: cuando falla la
    /// tercera tanda, las dos primeras ya están escritas en disco. Devolver
    /// solo el error tiraba la operación con ellas dentro, y lo ya cambiado se
    /// quedaba sin camino de vuelta. Así quien llama apunta lo que sí se hizo
    /// y después avisa del fallo.
    fn aplicar_lote(
        &mut self,
        ids: &[String],
        que: &(dyn Fn(&Item) -> Parche + Sync),
        progreso: &(dyn Fn(usize, usize) + Sync),
    ) -> Lote {
        let root = self.root.clone();
        let total = ids.len();
        let mut op = Operacion::nueva("");
        let mut ilegibles = 0usize;
        let mut hechos = 0usize;

        for trozo in ids.chunks(Self::LOTE) {
            let cargados: Vec<std::result::Result<Option<(Item, Parche, Parche)>, ()>> = trozo
                .par_iter()
                .map(|id| {
                    let Ok(mut item) = Item::read(&Index::item_json_path(&root, id)) else {
                        return Err(());
                    };
                    let atras = que(&item).aplicar(&mut item);
                    if atras.vacio() {
                        return Ok(None);
                    }
                    let ida = Parche::tomar(&item, &atras);
                    Ok(Some((item, atras, ida)))
                })
                .collect();

            let mut cambiados = Vec::with_capacity(trozo.len());
            for c in cargados {
                match c {
                    Err(()) => ilegibles += 1,
                    Ok(None) => {}
                    Ok(Some(t)) => cambiados.push(t),
                }
            }

            // El `item.json` primero y el índice después, como en `save_item`:
            // si el proceso muere en medio, la verdad está en disco y un
            // reindex la recupera. Al revés se perdería el cambio.
            //
            // Se escriben todos aunque alguno falle, y se sabe cuáles: los que
            // sí se escribieron ya han cambiado y tienen que poder deshacerse.
            let escritos: Vec<Result<()>> = cambiados
                .par_iter()
                .map(|(item, _, _)| item.write(&Index::item_json_path(&root, &item.id)))
                .collect();
            let mut error = None;
            let mut hechos_aqui = Vec::with_capacity(cambiados.len());
            for (t, r) in cambiados.into_iter().zip(escritos) {
                match r {
                    Ok(()) => hechos_aqui.push(t),
                    Err(e) => {
                        error.get_or_insert(e);
                    }
                }
            }
            let indexado = self
                .index
                .upsert_many(hechos_aqui.iter().map(|(i, _, _)| (i, None)));

            // Se apuntan aunque el índice haya fallado: el disco es la verdad,
            // y lo que está en disco cambiado tiene que poder volver.
            for (item, atras, ida) in hechos_aqui {
                op.antes.push((item.id.clone(), atras));
                op.despues.push((item.id, ida));
            }
            if let Err(e) = indexado {
                error.get_or_insert(e);
            }
            if error.is_some() {
                return Lote {
                    op,
                    ilegibles,
                    error,
                };
            }
            hechos += trozo.len();
            progreso(hechos, total);
        }
        Lote {
            op,
            ilegibles,
            error: None,
        }
    }

    /// Aplica el mismo cambio a muchos elementos y lo apunta como **una sola**
    /// cosa que se puede deshacer.
    ///
    /// El cierre recibe cada elemento y devuelve qué hacerle: así «añadir la
    /// etiqueta X» puede dar un parche distinto por elemento sin que quien
    /// llama tenga que cargarlos antes.
    ///
    /// Devuelve cuántos cambiaron de verdad y cuántos no se pudieron leer.
    pub fn editar(
        &mut self,
        ids: &[String],
        titulo: impl Into<String>,
        que: impl Fn(&Item) -> Parche + Sync,
        progreso: &(dyn Fn(usize, usize) + Sync),
    ) -> Result<(usize, usize)> {
        let Lote {
            mut op,
            ilegibles,
            error,
        } = self.aplicar_lote(ids, &que, progreso);
        op.titulo = titulo.into();
        let tocados = op.tocados();
        // Si el lote se paró a mitad, lo que sí cambió se apunta igual: así
        // deshacer devuelve a su sitio lo aplicado antes del fallo.
        if error.is_none() || tocados > 0 {
            self.registro.empujar(op);
        }
        match error {
            Some(e) => Err(e),
            None => Ok((tocados, ilegibles)),
        }
    }

    /// Apunta como deshacible algo que no pasó por [`Library::editar`].
    ///
    /// Lo usa la importación: lo contrario de meter cincuenta archivos es
    /// mandarlos a la papelera, no borrarlos, y eso se escribe como una
    /// operación normal en vez de como un caso aparte en el deshacer.
    pub fn anotar(&mut self, op: Operacion) {
        self.registro.empujar(op);
    }

    /// Deshace lo último. Devuelve la operación aplicada, o `None` si no había.
    ///
    /// Devuelve la operación entera y no solo su título porque quien llama
    /// necesita saber **qué** cambió: si fueron tres estrellas basta con
    /// repintar tres celdas, y si fue una carpeta hay que rehacer la vista.
    pub fn deshacer(
        &mut self,
        progreso: &(dyn Fn(usize, usize) + Sync),
    ) -> Result<Option<Operacion>> {
        let Some(op) = self.registro.ultima().cloned() else {
            return Ok(None);
        };
        self.revertir(&op, true, progreso)?;
        // Se marca **después** de aplicar: si aplicar falla, la pila queda como
        // estaba y el menú sigue ofreciendo lo mismo en vez de haber perdido un
        // paso sin haberlo dado.
        self.registro.marcar_deshecha();
        Ok(Some(op))
    }

    pub fn rehacer(
        &mut self,
        progreso: &(dyn Fn(usize, usize) + Sync),
    ) -> Result<Option<Operacion>> {
        let Some(op) = self.registro.ultima_deshecha().cloned() else {
            return Ok(None);
        };
        self.revertir(&op, false, progreso)?;
        self.registro.marcar_rehecha();
        Ok(Some(op))
    }

    fn revertir(
        &mut self,
        op: &Operacion,
        atras: bool,
        progreso: &(dyn Fn(usize, usize) + Sync),
    ) -> Result<()> {
        // El árbol primero. Si la operación borró una carpeta, hay que volver a
        // crearla antes de devolverle sus elementos: al revés, la pertenencia
        // apuntaría un rato a una carpeta que no existe.
        let arbol = if atras {
            &op.carpetas_antes
        } else {
            &op.carpetas_despues
        };
        if let Some(f) = arbol {
            self.folders = f.clone();
            self.commit_folders()?;
        }

        let parches = if atras { &op.antes } else { &op.despues };
        if parches.is_empty() {
            return Ok(());
        }
        let mapa: HashMap<&str, &Parche> = parches.iter().map(|(id, p)| (id.as_str(), p)).collect();
        let ids: Vec<String> = parches.iter().map(|(id, _)| id.clone()).collect();
        // Aquí no hay operación nueva que apuntar: los parches dicen valores,
        // no incrementos, así que si deshacer falla a mitad, repetirlo termina
        // el trabajo sin estropear lo que ya se hizo.
        let lote = self.aplicar_lote(
            &ids,
            &|item| {
                mapa.get(item.id.as_str())
                    .map(|p| (*p).clone())
                    .unwrap_or_default()
            },
            progreso,
        );
        match lote.error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    // -------------------------------------------------------------- papelera

    /// Manda a la papelera o saca de ella.
    ///
    /// Es un campo del `item.json`, no un movimiento de archivos: así
    /// deshacerlo es instantáneo y no depende de que el disco coopere, y un
    /// elemento en la papelera sigue teniendo sus etiquetas y sus carpetas por
    /// si vuelve.
    pub fn tirar(
        &mut self,
        ids: &[String],
        dentro: bool,
        progreso: &(dyn Fn(usize, usize) + Sync),
    ) -> Result<(usize, usize)> {
        let titulo = if dentro {
            "mandar a la papelera"
        } else {
            "sacar de la papelera"
        };
        self.editar(ids, titulo, |_| Parche::papelera(dentro), progreso)
    }

    /// Borra de verdad todo lo que hay en la papelera. Devuelve cuántos.
    ///
    /// **Esto no se deshace**, y por eso tira la historia entera: dejar el menú
    /// ofreciendo «deshacer» después de haber borrado archivos sería mentir.
    ///
    /// Lo que no recupera es el sitio de las miniaturas dentro del pack: el
    /// pack se escribe añadiendo al final y solo se encoge al compactarlo. Un
    /// hueco de 320 px cuesta unos pocos kilobytes y compactar cuesta reescribir
    /// el archivo entero, así que espera a que haya un motivo.
    ///
    /// Si alguno no se puede borrar, los demás se borran igual y el error
    /// dice cuáles quedaron: ver [`Library::vaciar_papelera_con_informe`].
    pub fn vaciar_papelera(&mut self, progreso: &(dyn Fn(usize, usize) + Sync)) -> Result<usize> {
        let v = self.vaciar_papelera_con_informe(progreso)?;
        if v.fallos.is_empty() {
            return Ok(v.borrados);
        }
        let mut msg = format!(
            "se borraron {} y {} no se pudieron borrar; siguen en la papelera:",
            v.borrados,
            v.fallos.len()
        );
        for (id, motivo) in v.fallos.iter().take(5) {
            msg.push_str(&format!("\n  {id}: {motivo}"));
        }
        if v.fallos.len() > 5 {
            msg.push_str(&format!("\n  … y {} más", v.fallos.len() - 5));
        }
        Err(Error::Invalid(msg))
    }

    /// Lo mismo que [`Library::vaciar_papelera`], pero en vez de un error
    /// devuelve qué se borró y qué no.
    ///
    /// Antes el primer elemento que no se dejaba borrar —un archivo abierto en
    /// otro programa, un permiso— cortaba todo con `?`. Los anteriores ya
    /// estaban borrados del disco pero seguían en el índice, apuntando a
    /// carpetas que no existían. Ahora cada uno va por su cuenta, y del índice
    /// sale exactamente lo que de verdad se borró.
    ///
    /// Antes de borrar se comprueba que la carpeta está **dentro** de `items/`.
    /// El id sale del índice, que sale de los `item.json`, y un `item.json`
    /// editado a mano con un id como `../../x` convertía `remove_dir_all` en
    /// borrar fuera de la biblioteca.
    pub fn vaciar_papelera_con_informe(
        &mut self,
        progreso: &(dyn Fn(usize, usize) + Sync),
    ) -> Result<Vaciado> {
        let ids = self.index.ids_en_papelera()?;
        let total = ids.len();
        let items = self.items_dir();
        let mut borrados = Vec::with_capacity(total);
        let mut fallos = Vec::new();
        for (n, id) in ids.iter().enumerate() {
            if n % 128 == 0 {
                progreso(n, total);
            }
            let dir = self.item_dir(id);
            if !crate::id::es_valido(id) || !dentro_de(&dir, &items) {
                fallos.push((
                    id.clone(),
                    "no es un identificador de Grimorio; no se borra nada".to_string(),
                ));
                continue;
            }
            if dir.exists() {
                if let Err(e) = std::fs::remove_dir_all(&dir) {
                    fallos.push((id.clone(), Error::io(&dir, e).to_string()));
                    continue;
                }
            }
            // La previsualización es un archivo aparte y derivado: si no está,
            // no pasa nada.
            let _ = std::fs::remove_file(preview_path(&self.root, id));
            borrados.push(id.clone());
        }
        self.index.borrar(&borrados)?;
        if !borrados.is_empty() {
            self.registro.limpiar();
        }
        progreso(total, total);
        Ok(Vaciado {
            borrados: borrados.len(),
            fallos,
        })
    }

    // -------------------------------------------------------------- carpetas

    pub fn folders(&self) -> &Folders {
        &self.folders
    }

    /// Guarda el árbol en disco y refresca la copia del índice. Todo cambio de
    /// carpetas pasa por aquí; nadie toca `folders.json` por su cuenta.
    fn commit_folders(&mut self) -> Result<()> {
        self.folders.guardar(&self.root)?;
        self.index.replace_folders(&self.folders.folders)
    }

    /// Apunta un cambio del árbol como operación deshacible.
    ///
    /// Guarda el árbol entero antes y después en vez de la diferencia. Son
    /// decenas de filas, y el inverso de «borrar una carpeta con tres hijas»
    /// escrito a mano es justo la clase de código que se equivoca callado.
    fn anotar_arbol(&mut self, titulo: impl Into<String>, antes: Folders) {
        let mut op = Operacion::nueva(titulo);
        op.carpetas_antes = Some(antes);
        op.carpetas_despues = Some(self.folders.clone());
        self.registro.empujar(op);
    }

    pub fn create_folder(&mut self, name: &str, parent: Option<&str>) -> Result<String> {
        let antes = self.folders.clone();
        let id = self.folders.crear(name, parent)?;
        self.commit_folders()?;
        self.anotar_arbol(format!("crear «{name}»"), antes);
        Ok(id)
    }

    pub fn rename_folder(&mut self, id: &str, name: &str) -> Result<()> {
        let antes = self.folders.clone();
        self.folders.renombrar(id, name)?;
        self.commit_folders()?;
        self.anotar_arbol(format!("renombrar a «{name}»"), antes);
        Ok(())
    }

    pub fn color_folder(&mut self, id: &str, color: Option<&str>) -> Result<()> {
        let antes = self.folders.clone();
        self.folders.colorear(id, color)?;
        self.commit_folders()?;
        let titulo = if color.is_some_and(|c| !c.trim().is_empty()) {
            "cambiar el color de una carpeta"
        } else {
            "quitar el color de una carpeta"
        };
        self.anotar_arbol(titulo, antes);
        Ok(())
    }

    /// Mueve la carpeta: de quién cuelga y, si se dice, delante de qué hermana
    /// queda. Colgar y colocar son el mismo gesto —arrastrar una fila— y por
    /// eso son la misma operación: dos dejarían dos entradas en el deshacer
    /// para lo que quien mira vio pasar una sola vez.
    pub fn move_folder(
        &mut self,
        id: &str,
        parent: Option<&str>,
        before: Option<&str>,
    ) -> Result<()> {
        let antes = self.folders.clone();
        self.folders.recolocar(id, parent, before)?;
        self.commit_folders()?;
        self.anotar_arbol("mover una carpeta", antes);
        Ok(())
    }

    /// Borra la carpeta y sus descendientes, y desvincula sus elementos.
    ///
    /// Los elementos **no se borran**: una carpeta es una etiqueta con
    /// jerarquía, y quitar la etiqueta no tira la foto. Devuelve cuántos
    /// elementos quedaron sueltos.
    pub fn delete_folder(&mut self, id: &str) -> Result<usize> {
        let nombre = self
            .folders
            .folders
            .iter()
            .find(|f| f.id == id)
            .map(|f| f.name.clone())
            .unwrap_or_else(|| id.to_string());
        let arbol_antes = self.folders.clone();
        let fuera = self.folders.borrar(id)?;
        let afectados = self.index.items_in_folders(&fuera)?;

        // El desenganche de los elementos y el borrado del árbol son **una
        // sola** operación: deshacer a medias —la carpeta vuelve pero vacía—
        // sería peor que no poder deshacer.
        //
        // Los elementos van **antes** que el árbol. Si el lote falla a mitad,
        // la carpeta sigue existiendo en disco y lo ya desenganchado se apunta
        // para poder volver a engancharlo. Al revés, como estaba, un fallo
        // dejaba el árbol guardado sin la carpeta y lo aplicado sin vuelta.
        let Lote { mut op, error, .. } = self.aplicar_lote(
            &afectados,
            &|item| {
                Parche::carpetas(
                    item.folders
                        .iter()
                        .filter(|f| !fuera.contains(f))
                        .cloned()
                        .collect(),
                )
            },
            &|_, _| {},
        );
        if let Some(e) = error {
            // El árbol en memoria vuelve a como estaba: en disco no se tocó.
            self.folders = arbol_antes;
            if op.tocados() > 0 {
                op.titulo = format!("borrar «{nombre}» (a medias)");
                self.registro.empujar(op);
            }
            return Err(e);
        }
        self.commit_folders()?;
        op.titulo = format!("borrar «{nombre}»");
        op.carpetas_antes = Some(arbol_antes);
        op.carpetas_despues = Some(self.folders.clone());
        self.registro.empujar(op);
        Ok(afectados.len())
    }

    /// Sustituye la pertenencia de un elemento. Las carpetas que no existan se
    /// descartan en silencio: un `item.json` copiado de otra biblioteca no debe
    /// impedir abrir la propia.
    pub fn set_item_folders(&mut self, item_id: &str, folders: &[String]) -> Result<()> {
        let nuevas: Vec<String> = folders
            .iter()
            .filter(|f| self.folders.existe(f))
            .cloned()
            .collect();
        self.editar(
            std::slice::from_ref(&item_id.to_string()),
            "cambiar de carpeta",
            |_| Parche::carpetas(nuevas.clone()),
            &|_, _| {},
        )?;
        Ok(())
    }

    /// Añade o quita carpetas de varios elementos a la vez, sin tocar el resto
    /// de su pertenencia. Es lo que hace arrastrar una selección a la barra
    /// lateral.
    pub fn update_item_folders(
        &mut self,
        item_ids: &[String],
        anadir: &[String],
        quitar: &[String],
        progreso: &(dyn Fn(usize, usize) + Sync),
    ) -> Result<usize> {
        // Las que no existen se descartan **antes** del lote: dentro del cierre
        // no se puede consultar la biblioteca, y además así se comprueba una
        // vez y no diez mil.
        let ponen: Vec<String> = anadir
            .iter()
            .filter(|f| self.folders.existe(f))
            .cloned()
            .collect();
        let titulo = if ponen.is_empty() {
            "sacar de la carpeta"
        } else {
            "meter en la carpeta"
        };
        let (tocados, _) = self.editar(
            item_ids,
            titulo,
            |item| {
                let mut v = item.folders.clone();
                for f in &ponen {
                    if !v.iter().any(|x| x == f) {
                        v.push(f.clone());
                    }
                }
                v.retain(|f| !quitar.iter().any(|x| x == f));
                Parche::carpetas(v)
            },
            progreso,
        )?;
        Ok(tocados)
    }

    pub fn set_import_mode(&mut self, mode: OriginMode) -> Result<()> {
        self.meta.import_mode = mode;
        let path = self.root.join(LIBRARY_FILE);
        let json = serde_json::to_vec_pretty(&self.meta).map_err(|e| Error::json(&path, e))?;
        write_atomic(&path, &json)
    }

    /// Reconstruye `index.sqlite` desde cero. Devuelve cuántos elementos
    /// entraron y qué `item.json` se saltaron por estar rotos.
    pub fn reindex(&mut self) -> Result<crate::index::Reindexado> {
        let root = self.root.clone();
        self.index.reconstruir(&root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crear_y_reabrir() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("Mi.grimorio");
        let lib = Library::create(&root, "Mi").unwrap();
        assert_eq!(lib.meta().name, "Mi");
        assert_eq!(lib.meta().import_mode, OriginMode::Copy);
        drop(lib);

        let lib2 = Library::open(&root).unwrap();
        assert_eq!(lib2.meta().name, "Mi");
        assert!(root.join("items").is_dir());
        assert!(root.join("thumbs/preview").is_dir());
    }

    #[test]
    fn renombrar_cambia_el_nombre_y_no_toca_lo_demas() {
        let dir = tempfile::tempdir().unwrap();
        let mut lib = Library::create(dir.path(), "Vieja").unwrap();
        // Lo que escribiría una versión más nueva: tiene que sobrevivir.
        let path = dir.path().join(LIBRARY_FILE);
        let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        v["futuro"] = serde_json::json!({ "algo": 1 });
        std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();

        lib.rename("  Referencias de luz  ").unwrap();
        assert_eq!(lib.meta().name, "Referencias de luz");
        let v: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(v["futuro"]["algo"], 1);
        assert_eq!(Library::open(dir.path()).unwrap().meta().name, "Referencias de luz");

        assert!(lib.rename("   ").is_err());
        assert_eq!(lib.meta().name, "Referencias de luz");
    }

    #[test]
    fn abrir_algo_que_no_es_biblioteca_falla_claro() {
        let dir = tempfile::tempdir().unwrap();
        let err = Library::open(dir.path()).unwrap_err();
        assert!(matches!(err, Error::NotALibrary(_)));
    }

    #[test]
    fn no_se_crea_dos_veces_encima() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("x.grimorio");
        Library::create(&root, "x").unwrap();
        assert!(matches!(
            Library::create(&root, "x").unwrap_err(),
            Error::AlreadyExists(_)
        ));
    }

    #[test]
    fn guardar_item_actualiza_json_e_indice() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("x.grimorio");
        let mut lib = Library::create(&root, "x").unwrap();
        let mut it = Item::new(
            crate::id::new_id(),
            "prueba".into(),
            "jpg".into(),
            10,
            OriginMode::Copy,
        );
        it.tags = vec!["a".into()];
        lib.save_item(&it).unwrap();

        assert_eq!(lib.index().count().unwrap(), 1);
        let back = lib.load_item(&it.id).unwrap();
        assert_eq!(back.tags, vec!["a".to_string()]);
    }
}

#[cfg(test)]
mod tests_carpetas {
    use super::*;
    use crate::item::Item;

    fn biblioteca() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let raiz = dir.path().join("prueba.grimorio");
        let lib = Library::create(&raiz, "prueba").unwrap();
        (dir, lib)
    }

    fn mete_item(lib: &mut Library, nombre: &str, carpetas: &[String]) -> String {
        let mut item = Item::new(
            crate::id::new_id(),
            nombre.to_string(),
            "jpg".into(),
            1024,
            OriginMode::Copy,
        );
        item.folders = carpetas.to_vec();
        let id = item.id.clone();
        std::fs::create_dir_all(lib.item_dir(&id)).unwrap();
        lib.save_item(&item).unwrap();
        id
    }

    #[test]
    fn una_carpeta_ensena_tambien_lo_de_sus_hijas() {
        let (_d, mut lib) = biblioteca();
        let padre = lib.create_folder("Referencias", None).unwrap();
        let hija = lib.create_folder("Tipografía", Some(&padre)).unwrap();
        mete_item(&mut lib, "en el padre", &[padre.clone()]);
        mete_item(&mut lib, "en la hija", &[hija.clone()]);
        mete_item(&mut lib, "suelto", &[]);

        let q = |folder: &str, rec: bool| crate::Query {
            folder: Some(folder.to_string()),
            folder_recursive: rec,
            ..Default::default()
        };
        assert_eq!(lib.index().search(&q(&padre, true)).unwrap().len(), 2);
        assert_eq!(lib.index().search(&q(&padre, false)).unwrap().len(), 1);
        assert_eq!(lib.index().search(&q(&hija, true)).unwrap().len(), 1);
    }

    #[test]
    fn borrar_una_carpeta_no_borra_sus_elementos() {
        let (_d, mut lib) = biblioteca();
        let padre = lib.create_folder("Referencias", None).unwrap();
        let hija = lib.create_folder("Tipografía", Some(&padre)).unwrap();
        let id = mete_item(&mut lib, "una foto", &[hija.clone()]);

        let sueltos = lib.delete_folder(&padre).unwrap();
        assert_eq!(sueltos, 1);
        assert!(lib.folders().folders.is_empty(), "se va también la hija");
        let item = lib.load_item(&id).unwrap();
        assert!(item.folders.is_empty(), "el elemento se queda sin carpeta");
        assert_eq!(
            lib.index().count().unwrap(),
            1,
            "pero sigue en la biblioteca"
        );
    }

    #[test]
    fn arrastrar_una_seleccion_no_pisa_el_resto_de_carpetas() {
        let (_d, mut lib) = biblioteca();
        let a = lib.create_folder("A", None).unwrap();
        let b = lib.create_folder("B", None).unwrap();
        let id = mete_item(&mut lib, "foto", &[a.clone()]);

        let tocados = lib
            .update_item_folders(&[id.clone()], &[b.clone()], &[], &|_, _| {})
            .unwrap();
        assert_eq!(tocados, 1);
        let item = lib.load_item(&id).unwrap();
        assert_eq!(item.folders.len(), 2, "sigue en A y ahora también en B");

        // Repetir la misma operación no debe volver a escribir el archivo.
        assert_eq!(
            lib.update_item_folders(&[id], &[b], &[], &|_, _| {})
                .unwrap(),
            0
        );
    }

    #[test]
    fn una_carpeta_que_no_existe_no_se_pega_al_elemento() {
        let (_d, mut lib) = biblioteca();
        let a = lib.create_folder("A", None).unwrap();
        let id = mete_item(&mut lib, "foto", &[]);
        lib.set_item_folders(&id, &[a.clone(), "inventada".into()])
            .unwrap();
        assert_eq!(lib.load_item(&id).unwrap().folders, vec![a]);
    }

    #[test]
    fn el_arbol_sobrevive_a_cerrar_y_abrir() {
        let (dir, mut lib) = biblioteca();
        let raiz = lib.root().to_path_buf();
        let padre = lib.create_folder("Referencias", None).unwrap();
        lib.create_folder("Tipografía", Some(&padre)).unwrap();
        drop(lib);

        let lib = Library::open(&raiz).unwrap();
        assert_eq!(lib.folders().folders.len(), 2);
        assert_eq!(lib.folders().hijas(Some(&padre)).len(), 1);
        drop(dir);
    }
}

#[cfg(test)]
mod tests_edicion {
    use super::*;
    use crate::item::Item;
    use crate::query::Query;

    fn biblioteca() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let raiz = dir.path().join("prueba.grimorio");
        let lib = Library::create(&raiz, "prueba").unwrap();
        (dir, lib)
    }

    fn mete(lib: &mut Library, cuantos: usize) -> Vec<String> {
        let mut ids = Vec::with_capacity(cuantos);
        for i in 0..cuantos {
            let item = Item::new(
                crate::id::new_id(),
                format!("foto {i}"),
                "jpg".into(),
                1024,
                OriginMode::Copy,
            );
            let id = item.id.clone();
            std::fs::create_dir_all(lib.item_dir(&id)).unwrap();
            lib.save_item(&item).unwrap();
            ids.push(id);
        }
        ids
    }

    fn nada(_: usize, _: usize) {}

    #[test]
    fn editar_en_lote_y_deshacerlo_deja_todo_como_estaba() {
        let (_d, mut lib) = biblioteca();
        let ids = mete(&mut lib, 20);
        let (tocados, _) = lib
            .editar(&ids, "poner 4 estrellas", |_| Parche::estrellas(4), &nada)
            .unwrap();
        assert_eq!(tocados, 20);
        assert_eq!(lib.load_item(&ids[7]).unwrap().stars, 4);

        let deshecha = lib.deshacer(&nada).unwrap().unwrap();
        assert_eq!(deshecha.titulo, "poner 4 estrellas");
        assert_eq!(lib.load_item(&ids[7]).unwrap().stars, 0);
        // Y el índice también, que es de donde bebe la malla.
        let q = Query {
            min_stars: Some(1),
            limit: 0,
            ..Default::default()
        };
        assert!(lib.index().search(&q).unwrap().is_empty());

        assert!(lib.rehacer(&nada).unwrap().is_some());
        assert_eq!(lib.load_item(&ids[7]).unwrap().stars, 4);
    }

    #[test]
    fn deshacer_solo_toca_los_elementos_que_cambiaron() {
        let (_d, mut lib) = biblioteca();
        let ids = mete(&mut lib, 3);
        // Uno ya tenía las cuatro estrellas de antes, puestas por separado.
        lib.editar(&ids[..1], "previo", |_| Parche::estrellas(4), &nada)
            .unwrap();
        lib.editar(&ids, "todos a 4", |_| Parche::estrellas(4), &nada)
            .unwrap();
        lib.deshacer(&nada).unwrap();
        assert_eq!(
            lib.load_item(&ids[0]).unwrap().stars,
            4,
            "el que ya estaba a 4 no entró en la operación y no vuelve a cero"
        );
        assert_eq!(lib.load_item(&ids[1]).unwrap().stars, 0);
    }

    #[test]
    fn deshacer_etiquetas_devuelve_las_de_antes_y_no_una_lista_vacia() {
        let (_d, mut lib) = biblioteca();
        let ids = mete(&mut lib, 4);
        lib.editar(
            &ids,
            "primera",
            |_| Parche::etiquetas(vec!["azul".into()]),
            &nada,
        )
        .unwrap();
        lib.editar(
            &ids,
            "añadir",
            |item| {
                let mut v = item.tags.clone();
                v.push("nocturno".into());
                Parche::etiquetas(v)
            },
            &nada,
        )
        .unwrap();
        assert_eq!(lib.load_item(&ids[0]).unwrap().tags.len(), 2);
        lib.deshacer(&nada).unwrap();
        assert_eq!(
            lib.load_item(&ids[0]).unwrap().tags,
            vec!["azul".to_string()]
        );
    }

    #[test]
    fn la_papelera_esconde_sin_borrar_y_se_puede_deshacer() {
        let (_d, mut lib) = biblioteca();
        let ids = mete(&mut lib, 5);
        lib.tirar(&ids[..2], true, &nada).unwrap();

        let normal = Query {
            limit: 0,
            ..Default::default()
        };
        let papelera = Query {
            limit: 0,
            papelera: true,
            ..Default::default()
        };
        assert_eq!(lib.index().search(&normal).unwrap().len(), 3);
        assert_eq!(lib.index().search(&papelera).unwrap().len(), 2);
        assert!(lib.item_json(&ids[0]).exists(), "el archivo sigue ahí");

        lib.deshacer(&nada).unwrap();
        assert_eq!(lib.index().search(&normal).unwrap().len(), 5);
        assert!(lib.index().search(&papelera).unwrap().is_empty());
    }

    #[test]
    fn vaciar_la_papelera_borra_de_verdad_y_no_deja_deshacer() {
        let (_d, mut lib) = biblioteca();
        let ids = mete(&mut lib, 4);
        lib.tirar(&ids[..1], true, &nada).unwrap();
        assert_eq!(lib.vaciar_papelera(&nada).unwrap(), 1);

        assert!(
            !lib.item_dir(&ids[0]).exists(),
            "el directorio se va entero"
        );
        assert_eq!(lib.index().count().unwrap(), 3);
        assert!(
            lib.deshacer(&nada).unwrap().is_none(),
            "no se ofrece deshacer lo que no se puede deshacer"
        );
    }

    #[test]
    fn borrar_una_carpeta_se_deshace_con_sus_elementos_dentro() {
        let (_d, mut lib) = biblioteca();
        let ids = mete(&mut lib, 3);
        let carpeta = lib.create_folder("Moodboard", None).unwrap();
        lib.update_item_folders(&ids, &[carpeta.clone()], &[], &nada)
            .unwrap();
        lib.delete_folder(&carpeta).unwrap();
        assert!(lib.folders().folders.is_empty());

        lib.deshacer(&nada).unwrap();
        assert_eq!(lib.folders().folders.len(), 1, "la carpeta vuelve");
        assert_eq!(
            lib.load_item(&ids[0]).unwrap().folders,
            vec![carpeta.clone()],
            "y vuelve con sus elementos dentro, no vacía"
        );
        let q = Query {
            folder: Some(carpeta),
            limit: 0,
            ..Default::default()
        };
        assert_eq!(
            lib.index().search(&q).unwrap().len(),
            3,
            "también en el índice"
        );
    }

    #[test]
    fn renombrar_una_carpeta_se_deshace() {
        let (_d, mut lib) = biblioteca();
        let id = lib.create_folder("Rótulos", None).unwrap();
        lib.rename_folder(&id, "Carteles").unwrap();
        lib.deshacer(&nada).unwrap();
        assert_eq!(lib.folders().folders[0].name, "Rótulos");
    }

    #[test]
    fn un_item_json_ilegible_no_tumba_el_lote_entero() {
        let (_d, mut lib) = biblioteca();
        let ids = mete(&mut lib, 6);
        std::fs::write(lib.item_json(&ids[2]), b"{ esto no es json").unwrap();

        let (tocados, ilegibles) = lib
            .editar(&ids, "estrellas", |_| Parche::estrellas(3), &nada)
            .unwrap();
        assert_eq!((tocados, ilegibles), (5, 1));
        assert_eq!(
            lib.load_item(&ids[5]).unwrap().stars,
            3,
            "los demás sí cambian"
        );
    }

    #[test]
    fn el_progreso_llega_hasta_el_final() {
        let (_d, mut lib) = biblioteca();
        let ids = mete(&mut lib, 1100);
        let visto = std::sync::Mutex::new(Vec::new());
        lib.editar(&ids, "estrellas", |_| Parche::estrellas(1), &|h, t| {
            visto.lock().unwrap().push((h, t));
        })
        .unwrap();
        let v = visto.lock().unwrap();
        assert!(
            v.len() >= 3,
            "avisa por tanda, no una sola vez: {}",
            v.len()
        );
        assert_eq!(*v.last().unwrap(), (1100, 1100), "y termina en el total");
    }
}

#[cfg(test)]
mod tests_fallos {
    use super::*;
    use crate::item::Item;
    use crate::query::Query;

    fn biblioteca() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let raiz = dir.path().join("prueba.grimorio");
        let lib = Library::create(&raiz, "prueba").unwrap();
        (dir, lib)
    }

    fn mete(lib: &mut Library, nombre: &str) -> String {
        let item = Item::new(
            crate::id::new_id(),
            nombre.into(),
            "jpg".into(),
            1024,
            OriginMode::Copy,
        );
        lib.save_item(&item).unwrap();
        item.id
    }

    fn nada(_: usize, _: usize) {}

    /// Un id con forma de ruta en la papelera no puede hacer que vaciarla
    /// borre fuera de `items/`. Se meten directamente en el índice, que es
    /// por donde llegarían desde un `item.json` editado a mano.
    #[test]
    fn vaciar_la_papelera_no_sale_de_items_aunque_el_id_lo_pida() {
        let (d, mut lib) = biblioteca();
        // `items/../..` es la carpeta que contiene la biblioteca: el id
        // «....victima» apuntaba justo a esta carpeta de al lado.
        let victima = d.path().join("....victima");
        std::fs::create_dir_all(&victima).unwrap();
        std::fs::write(victima.join("importante.txt"), b"no me borres").unwrap();
        assert_eq!(lib.item_dir("....victima"), lib.items_dir().join("..").join("..").join("....victima"));

        let bueno = mete(&mut lib, "de verdad");
        for id in ["../x", "....victima", &bueno] {
            let mut it = Item::new(id.into(), "x".into(), "jpg".into(), 1, OriginMode::Copy);
            it.trashed = true;
            lib.index_mut().upsert(&it, None).unwrap();
        }

        let v = lib.vaciar_papelera_con_informe(&nada).unwrap();
        assert_eq!(v.borrados, 1, "solo el de verdad");
        assert_eq!(v.fallos.len(), 2);
        assert!(victima.join("importante.txt").exists(), "borró fuera de la biblioteca");
        assert!(!lib.item_dir(&bueno).exists());
        // Los que no se borraron siguen en la papelera, a la vista.
        let papelera = Query {
            papelera: true,
            limit: 0,
            ..Default::default()
        };
        assert_eq!(lib.index().search(&papelera).unwrap().len(), 2);

        // Y la versión de siempre lo dice como error, sin callarlo.
        let e = lib.vaciar_papelera(&nada).unwrap_err().to_string();
        assert!(e.contains("no se pudieron borrar"), "{e}");
    }

    /// `Item::read` rechaza un id con forma de ruta, que es la puerta por la
    /// que entraría en el índice al reindexar.
    #[test]
    fn un_item_json_con_un_id_de_ruta_no_se_lee() {
        let (_d, mut lib) = biblioteca();
        let id = mete(&mut lib, "x");
        let ruta = lib.item_json(&id);
        let texto = std::fs::read_to_string(&ruta)
            .unwrap()
            .replace(&id, "../../../../x");
        std::fs::write(&ruta, texto).unwrap();
        assert!(Item::read(&ruta).is_err());
    }

    /// Un elemento que no se deja borrar —aquí, una carpeta sin permisos—
    /// no para a los demás, y del índice sale solo lo que se borró.
    #[cfg(unix)]
    #[test]
    fn vaciar_sigue_si_uno_no_se_deja_y_solo_quita_lo_borrado() {
        use std::os::unix::fs::PermissionsExt;
        let (_d, mut lib) = biblioteca();
        let a = mete(&mut lib, "a");
        let b = mete(&mut lib, "b");
        let c = mete(&mut lib, "c");
        lib.tirar(&[a.clone(), b.clone(), c.clone()], true, &nada).unwrap();
        // Sin permiso de escritura en la carpeta del elemento, no se puede
        // vaciar su contenido.
        let dir_b = lib.item_dir(&b);
        std::fs::set_permissions(&dir_b, std::fs::Permissions::from_mode(0o555)).unwrap();
        let v = lib.vaciar_papelera_con_informe(&nada);
        std::fs::set_permissions(&dir_b, std::fs::Permissions::from_mode(0o755)).unwrap();
        let v = v.unwrap();
        if v.fallos.is_empty() {
            // Como root los permisos no cuentan; no hay nada que comprobar.
            return;
        }
        assert_eq!(v.borrados, 2);
        assert_eq!(v.fallos.len(), 1);
        assert_eq!(v.fallos[0].0, b);
        assert_eq!(lib.index().ids_en_papelera().unwrap(), vec![b]);
    }

    /// Si una edición en lote falla a mitad, lo que sí se cambió se puede
    /// deshacer. Antes el error tiraba la operación con todo dentro.
    #[cfg(unix)]
    #[test]
    fn un_lote_que_falla_a_mitad_deja_deshacer_lo_aplicado() {
        use std::os::unix::fs::PermissionsExt;
        let (_d, mut lib) = biblioteca();
        let a = mete(&mut lib, "a");
        let b = mete(&mut lib, "b");
        let dir_b = lib.item_dir(&b);
        std::fs::set_permissions(&dir_b, std::fs::Permissions::from_mode(0o555)).unwrap();
        let r = lib.editar(&[a.clone(), b.clone()], "estrellas", |_| Parche::estrellas(5), &nada);
        std::fs::set_permissions(&dir_b, std::fs::Permissions::from_mode(0o755)).unwrap();
        if r.is_ok() {
            return; // como root se escribe igual
        }
        assert_eq!(lib.load_item(&a).unwrap().stars, 5, "el que se pudo, cambió");
        assert_eq!(lib.load_item(&b).unwrap().stars, 0);
        assert!(lib.que_se_deshace().is_some(), "lo aplicado tiene que poder volver");
        lib.deshacer(&nada).unwrap();
        assert_eq!(lib.load_item(&a).unwrap().stars, 0);
    }

    /// Borrar una carpeta cuyo lote falla no guarda el árbol sin ella: la
    /// carpeta sigue y lo desenganchado se puede volver a enganchar.
    #[cfg(unix)]
    #[test]
    fn borrar_una_carpeta_que_falla_a_mitad_no_la_pierde() {
        use std::os::unix::fs::PermissionsExt;
        let (_d, mut lib) = biblioteca();
        let carpeta = lib.create_folder("cajón", None).unwrap();
        let a = mete(&mut lib, "a");
        let b = mete(&mut lib, "b");
        lib.update_item_folders(&[a.clone(), b.clone()], std::slice::from_ref(&carpeta), &[], &nada)
            .unwrap();
        let dir_b = lib.item_dir(&b);
        std::fs::set_permissions(&dir_b, std::fs::Permissions::from_mode(0o555)).unwrap();
        let r = lib.delete_folder(&carpeta);
        std::fs::set_permissions(&dir_b, std::fs::Permissions::from_mode(0o755)).unwrap();
        if r.is_ok() {
            return;
        }
        assert!(lib.folders().existe(&carpeta), "en memoria");
        let releida = Library::open(lib.root()).unwrap();
        assert!(releida.folders().existe(&carpeta), "en disco");
        drop(releida);
        lib.deshacer(&nada).unwrap();
        assert_eq!(lib.load_item(&a).unwrap().folders, vec![carpeta.clone()]);
        assert_eq!(lib.load_item(&b).unwrap().folders, vec![carpeta]);
    }

    /// Una carpeta inteligente guardada da lo mismo al cerrar y volver a abrir
    /// la biblioteca: se guarda el texto y se vuelve a leer igual.
    #[test]
    fn una_busqueda_guardada_da_lo_mismo_al_reabrir() {
        let (_d, mut lib) = biblioteca();
        let raiz = lib.root().to_path_buf();
        let ahora = crate::time::now_ms();
        let hace_un_mes = ahora - 30 * 24 * 3600 * 1000;
        let mut esperados = Vec::new();
        for (i, (tag, estrellas, cuando)) in [
            ("gato", 5, ahora),
            ("gato", 4, ahora),
            ("gato", 3, ahora),
            ("perro", 5, ahora),
            ("gato", 5, hace_un_mes),
        ]
        .into_iter()
        .enumerate()
        {
            let mut it = Item::new(
                crate::id::new_id(),
                format!("foto {i}"),
                "jpg".into(),
                10,
                OriginMode::Copy,
            );
            it.tags = vec![tag.into()];
            it.stars = estrellas;
            it.imported_at = crate::time::to_rfc3339(cuando);
            lib.save_item(&it).unwrap();
            if tag == "gato" && estrellas >= 4 && cuando == ahora {
                esperados.push(it.id);
            }
        }
        let consulta = "#gato estrellas:>=4 fecha:7d";
        crate::busquedas::anadir(&raiz, "gatos buenos", consulta).unwrap();

        let buscar = |lib: &Library| {
            let guardada = crate::busquedas::cargar(lib.root()).unwrap();
            assert_eq!(guardada.len(), 1);
            let (q, avisos) = crate::filtro::parsear(&guardada[0].consulta, Query::default());
            assert!(avisos.is_empty(), "{avisos:?}");
            let mut ids: Vec<String> = lib.index().search(&q).unwrap().into_iter().map(|h| h.id).collect();
            ids.sort();
            ids
        };
        let antes = buscar(&lib);
        drop(lib);
        let lib = Library::open(&raiz).unwrap();
        let despues = buscar(&lib);
        esperados.sort();
        assert_eq!(antes, esperados);
        assert_eq!(despues, antes);
    }

    #[test]
    fn el_cerrojo_de_la_app_solo_cuenta_si_su_proceso_vive() {
        let (_d, lib) = biblioteca();
        let raiz = lib.root();
        assert_eq!(abierta_por_la_app(raiz), None, "sin cerrojo");

        let maquina = nombre_de_maquina().unwrap_or_default();
        let yo = std::process::id();
        std::fs::write(raiz.join(CERROJO_APP), format!("{yo}\ngrimorio\n{maquina}\n")).unwrap();
        assert_eq!(abierta_por_la_app(raiz), Some(yo));

        // Un pid que no existe: un cierre a lo bruto, no cuenta.
        #[cfg(unix)]
        {
            std::fs::write(raiz.join(CERROJO_APP), format!("{}\ngrimorio\n{maquina}\n", i32::MAX - 1))
                .unwrap();
            assert_eq!(abierta_por_la_app(raiz), None);
        }

        // De otra máquina no se puede comprobar: se respeta.
        std::fs::write(raiz.join(CERROJO_APP), "1\ngrimorio\notra-maquina-que-no-es-esta\n").unwrap();
        assert_eq!(abierta_por_la_app(raiz), Some(1));
    }
}
