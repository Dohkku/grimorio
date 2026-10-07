//! Índice SQLite. **Derivado**: se puede borrar y reconstruir desde los
//! `item.json` y el pack de miniaturas (ver [`Index::rebuild_from_disk`]).
//!
//! Aquí solo vive lo que hace falta para responder rápido: filtrar, ordenar y
//! paginar. Nada de lógica de producto.

use crate::error::Result;
use crate::item::{shard_dir, Item};
use crate::query::{Orientation, Query, QueryHit, SortBy};
use crate::thumbs::ThumbRef;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use std::collections::HashSet;
use std::path::Path;

/// Versión 4: la tabla del orden a mano (`item_orden`). Entró en el lote de
/// creación sin subir este número, y un índice que ya estaba en la 3 salía de
/// `migrate` antes de crearla: reordenar arrastrando daba «no such table» en
/// toda biblioteca que existiera de antes.
pub const USER_VERSION: i32 = 4;

/// Cuántos `item.json` se leen a la vez al reconstruir el índice. Todos van a
/// la misma transacción; la tanda solo acota la memoria.
const LOTE_REINDEX: usize = 8192;

/// La tabla de texto.
///
/// `remove_diacritics 2`: buscar "diseno" encuentra "diseño".
/// Sin columna `id`: la fila de FTS comparte rowid con la de `items`, así
/// borrar y unir son O(1). Con un `id UNINDEXED` cada borrado escanearía la
/// tabla entera y la importación se volvía cuadrática.
///
/// Está aquí aparte porque la crean dos sitios: `migrate` y la reconstrucción,
/// que la tira y la vuelve a hacer en vez de vaciarla (ver
/// `Index::reconstruir_en`).
const CREAR_FTS: &str = r#"CREATE VIRTUAL TABLE IF NOT EXISTS items_fts USING fts5(
                text,
                tokenize = "unicode61 remove_diacritics 2"
            )"#;

/// Los índices secundarios de `items` y de las tablas de unión, tal y como los
/// crea `migrate`. La reconstrucción los tira antes de cargar y los vuelve a
/// crear al final; una prueba comprueba que esta lista y `migrate` dicen lo
/// mismo, para que añadir uno en un sitio y olvidarlo en el otro no pase
/// callado.
const INDICES_SECUNDARIOS: &[(&str, &str)] = &[
    ("idx_items_ext", "CREATE INDEX IF NOT EXISTS idx_items_ext    ON items(ext)    WHERE trashed = 0"),
    ("idx_items_stars", "CREATE INDEX IF NOT EXISTS idx_items_stars  ON items(stars)  WHERE trashed = 0"),
    ("idx_items_size", "CREATE INDEX IF NOT EXISTS idx_items_size   ON items(size)   WHERE trashed = 0"),
    ("idx_items_dims", "CREATE INDEX IF NOT EXISTS idx_items_dims   ON items(width, height)"),
    ("idx_items_hash", "CREATE INDEX IF NOT EXISTS idx_items_hash   ON items(hash)"),
    ("idx_items_bucket", "CREATE INDEX IF NOT EXISTS idx_items_bucket ON items(color_bucket) WHERE trashed = 0"),
    ("idx_items_kind", "CREATE INDEX IF NOT EXISTS idx_items_kind ON items(kind) WHERE trashed = 0"),
    ("idx_items_adult", "CREATE INDEX IF NOT EXISTS idx_items_adult ON items(adult) WHERE trashed = 0"),
    ("idx_if_folder", "CREATE INDEX IF NOT EXISTS idx_if_folder ON item_folders(folder_id, item_id)"),
    ("idx_it_tag", "CREATE INDEX IF NOT EXISTS idx_it_tag ON item_tags(tag_id, item_id)"),
];

/// Lo que dejó una reconstrucción del índice.
#[derive(Debug, Default)]
pub struct Reindexado {
    /// Cuántos elementos entraron.
    pub vistos: u64,
    /// Los `item.json` que se saltaron y por qué: ilegibles, a medio
    /// escribir, de una versión más nueva o con un id repetido.
    pub rotos: Vec<(std::path::PathBuf, String)>,
}

pub struct Index {
    pub conn: Connection,
    /// Los grupos de repetidos ya calculados, para `repetidos:si`.
    ///
    /// Calcularlos es recorrer todas las huellas de la biblioteca —decenas de
    /// milisegundos con cien mil elementos— y la carpeta inteligente de
    /// repetidos lo pedía en **cada** búsqueda y en cada recuento de la barra
    /// lateral, aunque no hubiera cambiado nada. Ver [`CacheRepetidos`].
    repetidos: std::cell::RefCell<Option<CacheRepetidos>>,
    /// Cuántas veces ha escrito esta conexión. Lo sube todo lo que puede
    /// cambiar qué está repetido; la caché se compara contra él.
    generacion: std::cell::Cell<u64>,
}

/// Lo que se guarda de la última búsqueda de repetidos y con qué se hizo.
///
/// Vale mientras no cambien dos números. `generacion` cuenta las escrituras de
/// esta misma conexión. `data_version` es el contador que SQLite sube cuando
/// **otra** conexión confirma algo en el archivo —el `grim` de la terminal
/// importando con la ventana abierta—, y es justo lo que la cuenta propia no
/// puede ver. Preguntarlo es leer un entero de la cabecera, no una consulta.
struct CacheRepetidos {
    generacion: u64,
    data_version: i64,
    grupos: std::sync::Arc<Vec<Grupo>>,
}

impl Index {
    pub fn open(path: &Path) -> Result<Index> {
        let conn = Connection::open(path)?;
        Self::tune(&conn)?;
        let mut idx = Index::con(conn);
        idx.migrate()?;
        Ok(idx)
    }

    pub fn open_in_memory() -> Result<Index> {
        let conn = Connection::open_in_memory()?;
        Self::tune(&conn)?;
        let mut idx = Index::con(conn);
        idx.migrate()?;
        Ok(idx)
    }

    fn con(conn: Connection) -> Index {
        Index {
            conn,
            repetidos: std::cell::RefCell::new(None),
            generacion: std::cell::Cell::new(0),
        }
    }

    /// Avisa de que el contenido cambió: lo calculado antes ya no vale.
    fn tocado(&self) {
        self.generacion.set(self.generacion.get() + 1);
    }

    fn data_version(&self) -> Result<i64> {
        Ok(self.conn.query_row("PRAGMA data_version", [], |r| r.get(0))?)
    }

    /// Los grupos de repetidos ordenados como los enseña la vista —los
    /// exactos primero y los grandes antes—, calculados una vez y reusados
    /// hasta que algo cambie.
    fn grupos_repetidos(&self) -> Result<std::sync::Arc<Vec<Grupo>>> {
        let dv = self.data_version()?;
        let gen = self.generacion.get();
        if let Some(c) = self.repetidos.borrow().as_ref() {
            if c.generacion == gen && c.data_version == dv {
                return Ok(c.grupos.clone());
            }
        }
        let mut grupos = self.duplicados(3)?.grupos;
        grupos.sort_by(|a, b| b.exacto.cmp(&a.exacto).then(b.ids.len().cmp(&a.ids.len())));
        let grupos = std::sync::Arc::new(grupos);
        *self.repetidos.borrow_mut() = Some(CacheRepetidos {
            generacion: gen,
            data_version: dv,
            grupos: grupos.clone(),
        });
        Ok(grupos)
    }

    fn tune(conn: &Connection) -> Result<()> {
        // WAL: lecturas de la UI mientras la importación escribe.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        // NORMAL basta: si se corta la luz podemos perder la última transacción
        // del índice, y el índice es reconstruible. Los datos reales están en JSON.
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "temp_store", "MEMORY")?;
        conn.pragma_update(None, "cache_size", -64_000)?; // 64 MB
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(())
    }

    fn migrate(&mut self) -> Result<()> {
        let v: i32 = self
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap_or(0);
        if v >= USER_VERSION {
            return Ok(());
        }
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS meta (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS items (
                id           TEXT PRIMARY KEY,
                name         TEXT NOT NULL,
                ext          TEXT NOT NULL,
                size         INTEGER NOT NULL,
                width        INTEGER NOT NULL DEFAULT 0,
                height       INTEGER NOT NULL DEFAULT 0,
                hash         TEXT NOT NULL DEFAULT '',
                phash        INTEGER,
                imported_at  INTEGER NOT NULL,
                modified_at  INTEGER NOT NULL,
                source       TEXT,
                stars        INTEGER NOT NULL DEFAULT 0,
                note         TEXT,
                origin_mode  TEXT NOT NULL DEFAULT 'copy',
                origin_path  TEXT,
                thumb_off    INTEGER,
                thumb_len    INTEGER,
                dom_r        INTEGER,
                dom_g        INTEGER,
                dom_b        INTEGER,
                color_bucket INTEGER,
                trashed      INTEGER NOT NULL DEFAULT 0
            );

            -- El orden por defecto (importación descendente) sale del propio id
            -- ULID, así que no necesita índice extra.
            CREATE INDEX IF NOT EXISTS idx_items_ext    ON items(ext)    WHERE trashed = 0;
            CREATE INDEX IF NOT EXISTS idx_items_stars  ON items(stars)  WHERE trashed = 0;
            CREATE INDEX IF NOT EXISTS idx_items_size   ON items(size)   WHERE trashed = 0;
            CREATE INDEX IF NOT EXISTS idx_items_dims   ON items(width, height);
            CREATE INDEX IF NOT EXISTS idx_items_hash   ON items(hash);
            CREATE INDEX IF NOT EXISTS idx_items_bucket ON items(color_bucket) WHERE trashed = 0;

            CREATE TABLE IF NOT EXISTS folders (
                id     TEXT PRIMARY KEY,
                name   TEXT NOT NULL,
                parent TEXT,
                color  TEXT,
                pos    INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_folders_parent ON folders(parent);

            CREATE TABLE IF NOT EXISTS item_folders (
                item_id   TEXT NOT NULL,
                folder_id TEXT NOT NULL,
                PRIMARY KEY (item_id, folder_id)
            );
            CREATE INDEX IF NOT EXISTS idx_if_folder ON item_folders(folder_id, item_id);

            -- El orden a mano: una posición por elemento y sitio ("" es Todo).
            CREATE TABLE IF NOT EXISTS item_orden (
                item_id TEXT NOT NULL,
                ctx     TEXT NOT NULL,
                pos     REAL NOT NULL,
                PRIMARY KEY (item_id, ctx)
            );

            CREATE TABLE IF NOT EXISTS tags (
                id   INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE
            );

            CREATE TABLE IF NOT EXISTS item_tags (
                item_id TEXT NOT NULL,
                tag_id  INTEGER NOT NULL,
                PRIMARY KEY (item_id, tag_id)
            );
            CREATE INDEX IF NOT EXISTS idx_it_tag ON item_tags(tag_id, item_id);

            "#,
        )?;
        self.conn.execute_batch(&format!("{CREAR_FTS};"))?;
        // Versión 2: la familia del archivo. Va como paso aparte y no dentro
        // del `CREATE TABLE` para que valga igual para un índice recién hecho y
        // para uno de la versión anterior; en el segundo caso, `reindex` le
        // pone a cada fila la suya, y mientras tanto «imagen» es lo correcto
        // porque hasta la versión 2 solo entraban imágenes.
        let hay_kind: bool = self
            .conn
            .prepare("SELECT 1 FROM pragma_table_info('items') WHERE name = 'kind'")?
            .exists([])?;
        if !hay_kind {
            self.conn.execute_batch(
                "ALTER TABLE items ADD COLUMN kind TEXT NOT NULL DEFAULT 'imagen';
                 ALTER TABLE items ADD COLUMN duration_ms INTEGER;
                 ALTER TABLE items ADD COLUMN pages INTEGER;
                 CREATE INDEX IF NOT EXISTS idx_items_kind ON items(kind) WHERE trashed = 0;",
            )?;
        }

        // Versión 3: la marca de contenido adulto. Igual que la anterior: un
        // paso aparte que vale para un índice nuevo y para uno viejo, y cuyo
        // valor por defecto —«no»— es el correcto para todo lo ya importado.
        let hay_adult: bool = self
            .conn
            .prepare("SELECT 1 FROM pragma_table_info('items') WHERE name = 'adult'")?
            .exists([])?;
        if !hay_adult {
            self.conn.execute_batch(
                "ALTER TABLE items ADD COLUMN adult INTEGER NOT NULL DEFAULT 0;
                 CREATE INDEX IF NOT EXISTS idx_items_adult ON items(adult) WHERE trashed = 0;",
            )?;
        }

        self.conn
            .pragma_update(None, "user_version", USER_VERSION)?;
        Ok(())
    }

    // ------------------------------------------------------------- escritura

    /// Mete o actualiza muchos elementos en **una sola transacción**.
    ///
    /// Toma un iterador de referencias y no un vector de elementos por una
    /// razón medida: la edición en lote de diez mil elementos ya los tiene
    /// cargados, y clonarlos otra vez solo para pasarlos aquí era copiar de
    /// balde varios megabytes por tanda.
    ///
    /// Un `thumb` en `None` **conserva** la miniatura que ya hubiera: el
    /// `COALESCE` de abajo es lo que permite guardar un cambio de etiquetas sin
    /// tener que preguntar antes por el offset del pack.
    pub fn upsert_many<'a, I>(&mut self, rows: I) -> Result<()>
    where
        I: IntoIterator<Item = (&'a Item, Option<ThumbRef>)>,
    {
        self.tocado();
        let tx = self.conn.transaction()?;
        escribir_filas(&tx, rows, false)?;
        tx.commit()?;
        Ok(())
    }

    pub fn upsert(&mut self, item: &Item, thumb: Option<ThumbRef>) -> Result<()> {
        self.upsert_many([(item, thumb)])
    }

    // -------------------------------------------------------------- carpetas

    /// Vuelca el árbol de `folders.json` a la tabla del índice.
    ///
    /// Se rehace entera en vez de calcular diferencias: son decenas de filas,
    /// y así no hay forma de que la copia se quede desincronizada del archivo,
    /// que es la verdad.
    pub fn replace_folders(&mut self, folders: &[crate::folder::Folder]) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM folders", [])?;
        {
            let mut ins = tx.prepare(
                "INSERT INTO folders (id,name,parent,color,pos) VALUES (?1,?2,?3,?4,?5)",
            )?;
            for f in folders {
                ins.execute(params![f.id, f.name, f.parent, f.color, f.pos])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Ids de los elementos que están en alguna de esas carpetas.
    pub fn items_in_folders(&self, folders: &[String]) -> Result<Vec<String>> {
        if folders.is_empty() {
            return Ok(Vec::new());
        }
        let marks = vec!["?"; folders.len()].join(",");
        let mut st = self.conn.prepare(&format!(
            "SELECT DISTINCT item_id FROM item_folders WHERE folder_id IN ({marks})"
        ))?;
        let filas = st.query_map(params_from_iter(folders.iter()), |r| r.get::<_, String>(0))?;
        let mut out = Vec::new();
        for f in filas {
            out.push(f?);
        }
        Ok(out)
    }

    /// Cuántos elementos hay en cada carpeta, sin contar los de sus hijas.
    /// La barra lateral suma los descendientes por su cuenta, que ya tiene el
    /// árbol en memoria.
    pub fn folder_counts(&self) -> Result<std::collections::HashMap<String, u64>> {
        let mut st = self.conn.prepare(
            "SELECT f.folder_id, COUNT(*)
               FROM item_folders f JOIN items i ON i.id = f.item_id
              WHERE i.trashed = 0
              GROUP BY f.folder_id",
        )?;
        let filas = st.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64))
        })?;
        let mut out = std::collections::HashMap::new();
        for f in filas {
            let (k, v) = f?;
            out.insert(k, v);
        }
        Ok(out)
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO meta (key,value) VALUES (?1,?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_meta(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM meta WHERE key = ?1", params![key], |r| {
                r.get(0)
            })
            .optional()?)
    }

    // -------------------------------------------------------------- lectura

    pub fn count(&self) -> Result<u64> {
        let n: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM items WHERE trashed = 0", [], |r| {
                    r.get(0)
                })?;
        Ok(n as u64)
    }

    pub fn total_size(&self) -> Result<u64> {
        let n: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(size),0) FROM items WHERE trashed = 0",
            [],
            |r| r.get(0),
        )?;
        Ok(n as u64)
    }

    pub fn known_hashes(&self) -> Result<HashSet<String>> {
        let mut st = self
            .conn
            .prepare("SELECT hash FROM items WHERE hash <> '' AND trashed = 0")?;
        let rows = st.query_map([], |r| r.get::<_, String>(0))?;
        let mut out = HashSet::new();
        for h in rows {
            out.insert(h?);
        }
        Ok(out)
    }

    pub fn get(&self, id: &str) -> Result<Option<QueryHit>> {
        Ok(self
            .conn
            .query_row(
                &format!("SELECT {SELECT_COLS} FROM items i WHERE i.id = ?1"),
                params![id],
                hit_from_row,
            )
            .optional()?)
    }

    /// Busca elementos cuyo id empiece por `prefijo`. Devuelve como mucho
    /// `tope` coincidencias.
    ///
    /// Existe porque los listados enseñan los ocho primeros caracteres del id y
    /// sería absurdo obligar a teclear veintiséis. La comparación es por rango,
    /// no `LIKE`, para que use el índice de la clave primaria.
    pub fn ids_con_prefijo(&self, prefijo: &str, tope: usize) -> Result<Vec<String>> {
        let desde = prefijo.to_ascii_uppercase();
        if desde.is_empty() {
            return Ok(Vec::new());
        }
        // 0x7F queda por encima de cualquier carácter del alfabeto de un ULID.
        let hasta = format!("{desde}\u{7f}");
        let mut st = self
            .conn
            .prepare("SELECT id FROM items WHERE id >= ?1 AND id < ?2 ORDER BY id LIMIT ?3")?;
        let filas = st.query_map(params![desde, hasta, tope as i64], |r| {
            r.get::<_, String>(0)
        })?;
        let mut out = Vec::new();
        for f in filas {
            out.push(f?);
        }
        Ok(out)
    }

    /// Busca por el final del id. Los ocho últimos caracteres de un ULID son
    /// azar puro, así que identifican de verdad; los ocho primeros son reloj y
    /// se repiten entre todo lo importado en el mismo cuarto de segundo.
    ///
    /// Esto sí recorre la tabla (no hay índice por sufijo), pero son unos pocos
    /// milisegundos con 100.000 elementos y solo ocurre al teclear un id a mano.
    pub fn ids_con_sufijo(&self, sufijo: &str, tope: usize) -> Result<Vec<String>> {
        let s = sufijo.to_ascii_uppercase();
        if s.is_empty() {
            return Ok(Vec::new());
        }
        let mut st = self
            .conn
            .prepare("SELECT id FROM items WHERE id LIKE '%' || ?1 ORDER BY id LIMIT ?2")?;
        let filas = st.query_map(params![s, tope as i64], |r| r.get::<_, String>(0))?;
        let mut out = Vec::new();
        for f in filas {
            out.push(f?);
        }
        Ok(out)
    }

    /// Etiquetas que empiezan por `prefijo`, de más usada a menos.
    ///
    /// Es lo que alimenta el autocompletado. Ordena por uso y no alfabéti-
    /// camente porque al escribir «ti» lo que se quiere es la etiqueta que ya
    /// se ha puesto cien veces, no la que se puso una vez por error.
    ///
    /// La comparación es sin distinguir mayúsculas ni el prefijo vacío: sin
    /// prefijo devuelve las más usadas, que es lo correcto para un desplegable
    /// recién abierto.
    pub fn tags_con_prefijo(&self, prefijo: &str, tope: usize) -> Result<Vec<(String, u64)>> {
        let mut st = self.conn.prepare(
            "SELECT t.name, COUNT(*) c FROM item_tags it
             JOIN tags t ON t.id = it.tag_id
             JOIN items i ON i.id = it.item_id AND i.trashed = 0
             WHERE ?1 = '' OR t.name LIKE ?2 ESCAPE '\\'
             GROUP BY t.id ORDER BY c DESC, t.name COLLATE NOCASE ASC LIMIT ?3",
        )?;
        // Escapar los comodines: buscar una etiqueta que se llame «100%» no
        // puede acabar devolviendo todas.
        let patron = format!(
            "{}%",
            prefijo
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        );
        let rows = st.query_map(params![prefijo, patron, tope as i64], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Grupos de elementos repetidos o casi iguales.
    ///
    /// Dos pasadas. La primera es exacta: mismo `blake3`, mismo archivo byte a
    /// byte, sin discusión. La segunda es perceptual y usa el `dHash` de 64
    /// bits, que sobrevive a un reescalado o a una recompresión.
    ///
    /// El problema de la segunda es que comparar todos contra todos son cinco
    /// mil millones de parejas con cien mil elementos. El truco es la ley del
    /// palomar: se parte la huella en cuatro bandas de dieciséis bits, y dos
    /// huellas que difieren en `distancia <= 3` bits **tienen que** coincidir
    /// exactamente en al menos una banda, porque tres bits no pueden repartirse
    /// entre cuatro bandas sin dejar una intacta. Así solo se comparan las
    /// parejas que comparten banda, que son unas pocas.
    ///
    /// Por eso la distancia está limitada a 3: con cuatro bits el razonamiento
    /// deja de valer y habría que partir en cinco bandas.
    pub fn duplicados(&self, distancia: u32) -> Result<Duplicados> {
        let distancia = distancia.min(3);
        let mut grupos: Vec<Grupo> = Vec::new();
        let mut saltados = 0usize;

        // --- exactos, por contenido
        let mut st = self.conn.prepare(
            "SELECT hash, GROUP_CONCAT(id) FROM items
             WHERE trashed = 0 AND hash <> ''
             GROUP BY hash HAVING COUNT(*) > 1",
        )?;
        let filas = st.query_map([], |r| Ok(r.get::<_, String>(1)?))?;
        let mut ya: HashSet<String> = HashSet::new();
        for f in filas {
            let ids: Vec<String> = f?.split(',').map(|s| s.to_string()).collect();
            for id in &ids {
                ya.insert(id.clone());
            }
            grupos.push(Grupo { ids, exacto: true });
        }

        if distancia == 0 {
            return Ok(Duplicados { grupos, saltados });
        }

        // --- parecidos, por huella perceptual
        let mut st = self
            .conn
            .prepare("SELECT id, phash FROM items WHERE trashed = 0 AND phash IS NOT NULL")?;
        let filas = st.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64))
        })?;
        let mut todos: Vec<(String, u64)> = Vec::new();
        for f in filas {
            let (id, h) = f?;
            if !ya.contains(&id) {
                todos.push((id, h));
            }
        }

        // Cuatro bandas de dieciséis bits: cada una manda a los que la comparten.
        let mut cubos: std::collections::HashMap<(u8, u16), Vec<usize>> =
            std::collections::HashMap::new();
        for (i, (_, h)) in todos.iter().enumerate() {
            for banda in 0..4u8 {
                let trozo = ((h >> (banda * 16)) & 0xFFFF) as u16;
                cubos.entry((banda, trozo)).or_default().push(i);
            }
        }

        let mut union = Union::nueva(todos.len());
        for (_, miembros) in cubos {
            // Un cubo enorme es una huella degenerada —imágenes planas, casi
            // todas iguales— y compararlo entero volvería a ser cuadrático. El
            // tope es alto (cuatro mil son ocho millones de comparaciones, unos
            // pocos milisegundos) y lo que se salta **se cuenta**: un tope que
            // recorta en silencio deja creer que se ha mirado todo.
            if miembros.len() > TOPE_CUBO {
                saltados += 1;
                continue;
            }
            for a in 0..miembros.len() {
                for b in a + 1..miembros.len() {
                    let (i, j) = (miembros[a], miembros[b]);
                    if (todos[i].1 ^ todos[j].1).count_ones() <= distancia {
                        union.unir(i, j);
                    }
                }
            }
        }

        let mut por_raiz: std::collections::HashMap<usize, Vec<String>> =
            std::collections::HashMap::new();
        for (i, (id, _)) in todos.iter().enumerate() {
            por_raiz.entry(union.raiz(i)).or_default().push(id.clone());
        }
        for (_, ids) in por_raiz {
            if ids.len() > 1 {
                grupos.push(Grupo { ids, exacto: false });
            }
        }
        Ok(Duplicados { grupos, saltados })
    }

    /// Cuántos hay en la papelera. Cuenta en SQL en vez de traerse los ids:
    /// la barra lateral solo quiere el número, y con la papelera llena de miles
    /// de elementos traérselos todos para hacer `len()` sería tonto.
    pub fn contar_papelera(&self) -> Result<u64> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM items WHERE trashed = 1", [], |r| {
                r.get::<_, i64>(0)
            })? as u64)
    }

    /// Cuánto hay de cada familia y cuánto pesa, sin contar la papelera:
    /// `(familia, elementos, bytes)`, de la que más elementos tiene a la que
    /// menos. Es el pie de la barra lateral; una sola pasada agrupada sobre la
    /// tabla, sin tocar ningún `item.json`.
    pub fn reparto(&self) -> Result<Vec<(String, u64, u64)>> {
        let mut st = self.conn.prepare(
            "SELECT kind, COUNT(*), COALESCE(SUM(size), 0) FROM items
             WHERE trashed = 0 GROUP BY kind ORDER BY COUNT(*) DESC",
        )?;
        let filas = st.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)? as u64,
                r.get::<_, i64>(2)? as u64,
            ))
        })?;
        Ok(filas.collect::<std::result::Result<_, _>>()?)
    }

    /// Cuántos elementos hay marcados como adultos, sin contar la papelera.
    ///
    /// Lo pregunta la ventana para decidir si el botón del modo seguro tiene
    /// algo que hacer: en una biblioteca sin nada marcado, ese botón sería un
    /// interruptor que no enciende nada.
    pub fn contar_adultos(&self) -> Result<u64> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM items WHERE adult = 1 AND trashed = 0",
            [],
            |r| r.get::<_, i64>(0),
        )? as u64)
    }

    /// Los identificadores de lo que está en la papelera, de lo más reciente a
    /// lo más antiguo.
    pub fn ids_en_papelera(&self) -> Result<Vec<String>> {
        let mut st = self
            .conn
            .prepare("SELECT id FROM items WHERE trashed = 1 ORDER BY id DESC")?;
        let filas = st.query_map([], |r| r.get::<_, String>(0))?;
        let mut out = Vec::new();
        for f in filas {
            out.push(f?);
        }
        Ok(out)
    }

    /// Saca elementos del índice. Solo lo llama el borrado de verdad: el
    /// índice es derivado, así que quitar una fila de aquí sin borrar su
    /// `item.json` no borra nada, solo lo esconde hasta el siguiente reindex.
    ///
    /// Se lleva también lo que colgaba del elemento en otras tablas —su sitio
    /// en el orden a mano— y las etiquetas que se quedan sin nadie. Ninguna
    /// de las dos se ve —las consultas unen con `items`—, pero se quedaban en
    /// el archivo para siempre: borrar de verdad era el único momento de
    /// limpiarlas y era justo donde no se hacía, así que solo un reindex las
    /// quitaba.
    pub fn borrar(&mut self, ids: &[String]) -> Result<()> {
        self.tocado();
        let tx = self.conn.transaction()?;
        {
            let mut sel = tx.prepare("SELECT rowid FROM items WHERE id = ?1")?;
            let mut del_fts = tx.prepare("DELETE FROM items_fts WHERE rowid = ?1")?;
            let mut del_tags = tx.prepare("DELETE FROM item_tags WHERE item_id = ?1")?;
            let mut del_fold = tx.prepare("DELETE FROM item_folders WHERE item_id = ?1")?;
            let mut del_orden = tx.prepare("DELETE FROM item_orden WHERE item_id = ?1")?;
            let mut del = tx.prepare("DELETE FROM items WHERE id = ?1")?;
            for id in ids {
                if let Some(rowid) = sel
                    .query_row(params![id], |r| r.get::<_, i64>(0))
                    .optional()?
                {
                    del_fts.execute(params![rowid])?;
                }
                del_tags.execute(params![id])?;
                del_fold.execute(params![id])?;
                del_orden.execute(params![id])?;
                del.execute(params![id])?;
            }
        }
        tx.execute(
            "DELETE FROM tags WHERE NOT EXISTS
               (SELECT 1 FROM item_tags it WHERE it.tag_id = tags.id)",
            [],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn tag_cloud(&self, limit: usize) -> Result<Vec<(String, u64)>> {
        let mut st = self.conn.prepare(
            "SELECT t.name, COUNT(*) c FROM item_tags it
             JOIN tags t ON t.id = it.tag_id
             JOIN items i ON i.id = it.item_id AND i.trashed = 0
             GROUP BY t.id ORDER BY c DESC LIMIT ?1",
        )?;
        let rows = st.query_map(params![limit as i64], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn ext_histogram(&self) -> Result<Vec<(String, u64)>> {
        let mut st = self.conn.prepare(
            "SELECT ext, COUNT(*) c FROM items WHERE trashed = 0 GROUP BY ext ORDER BY c DESC",
        )?;
        let rows = st.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Lo que tiene repetido, grupo a grupo: los exactos primero y los grupos
    /// grandes antes, que es por donde se empieza a limpiar. El resto de la
    /// consulta (carpeta, tipo, texto) sigue valiendo: deja fuera de cada grupo
    /// lo que no encaja, y un grupo que se queda con uno solo ya no es grupo.
    fn buscar_repetidos(&self, q: &Query) -> Result<Vec<QueryHit>> {
        let grupos = self.grupos_repetidos()?;

        let mut resto = q.clone();
        resto.repetidos = false;
        resto.limit = 0;
        resto.offset = 0;
        let encajan: std::collections::HashMap<String, QueryHit> = self
            .search(&resto)?
            .into_iter()
            .map(|h| (h.id.clone(), h))
            .collect();

        let mut out = Vec::new();
        let mut encajan = encajan;
        for g in grupos.iter() {
            let dentro: Vec<QueryHit> = g.ids.iter().filter_map(|id| encajan.remove(id)).collect();
            if dentro.len() > 1 {
                out.extend(dentro);
            }
        }
        let desde = q.offset.min(out.len());
        let mut out = out.split_off(desde);
        if q.limit > 0 {
            out.truncate(q.limit);
        }
        Ok(out)
    }

    /// Las posiciones a mano puestas en un sitio ("" es Todo).
    pub fn posiciones(&self, ctx: &str) -> Result<std::collections::HashMap<String, f64>> {
        let mut st = self
            .conn
            .prepare("SELECT item_id, pos FROM item_orden WHERE ctx = ?1")?;
        let filas = st.query_map(params![ctx], |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?)))?;
        let mut out = std::collections::HashMap::new();
        for f in filas {
            let (id, p) = f?;
            out.insert(id, p);
        }
        Ok(out)
    }

    /// Cuántos daría una consulta, sin traerlos. Para el número de cada
    /// carpeta inteligente, que se recalcula a menudo.
    pub fn contar(&self, q: &Query) -> Result<u64> {
        if q.repetidos {
            let mut q = q.clone();
            q.limit = 0;
            q.offset = 0;
            return Ok(self.search(&q)?.len() as u64);
        }
        let mut q = q.clone();
        q.limit = 0;
        q.offset = 0;
        let (sql, args) = build_sql(&q);
        let sql = format!("SELECT COUNT(*) FROM ({sql})");
        let n: i64 = self
            .conn
            .query_row(&sql, params_from_iter(args.iter()), |r| r.get(0))?;
        Ok(n as u64)
    }

    /// Ejecuta una consulta. Devuelve solo lo que la malla necesita para pintar.
    pub fn search(&self, q: &Query) -> Result<Vec<QueryHit>> {
        if q.repetidos {
            return self.buscar_repetidos(q);
        }
        let (sql, args) = build_sql(q);
        let mut st = self.conn.prepare(&sql)?;
        let rows = st.query_map(params_from_iter(args.iter()), hit_from_row)?;
        let mut out = Vec::with_capacity(q.limit.min(4096));
        for r in rows {
            out.push(r?);
        }

        // La cercanía de color se resuelve fuera de SQL: el filtro por cubo ya
        // dejó un candidato pequeño y la distancia real es más honesta en Oklab.
        if let Some(target) = q.color {
            let tope = if q.limit == 0 { out.len() } else { q.limit };
            out = ordenar_por_color(out, target, tope);
        }
        Ok(out)
    }

    /// Reconstruye el índice recorriendo los `item.json` y el pack de miniaturas.
    /// Es la garantía de que `index.sqlite` sea desechable. Devuelve cuántos
    /// elementos entraron; ver [`Index::reconstruir`] para saber además cuáles
    /// no se pudieron leer.
    pub fn rebuild_from_disk(&mut self, lib_root: &Path) -> Result<u64> {
        Ok(self.reconstruir(lib_root)?.vistos)
    }

    /// Reconstruye el índice desde disco y dice qué `item.json` se saltó.
    ///
    /// Dos promesas, y las dos salen de cosas que pasaron:
    ///
    /// **Todo o nada.** Vaciar y rellenar van en una sola transacción. Antes el
    /// `DELETE` iba suelto y los elementos entraban por tandas confirmadas una
    /// a una, así que cualquier error a mitad dejaba un índice a medias —o
    /// vacío— que la ventana enseñaba como si fuera la biblioteca entera. Ahora
    /// un fallo deja el índice de antes exactamente como estaba.
    ///
    /// **Un archivo roto no tumba el resto.** Un `item.json` ilegible, a medio
    /// escribir o de una versión más nueva se cuenta en `rotos` y se sigue: con
    /// cien mil elementos, no poder reindexar por uno es peor que reindexar y
    /// avisar de cuál.
    ///
    /// Y deprisa, que con cien mil elementos tardaba casi un minuto:
    ///
    /// - Los `item.json` se leen y se interpretan en paralelo, por tandas,
    ///   mientras antes los recorría un solo núcleo uno detrás de otro.
    /// - `synchronous = OFF` mientras dura. Con el índice entero dentro de una
    ///   transacción, perder la luz a mitad deja el índice viejo —el WAL sin
    ///   confirmar se descarta— o, en el peor caso, uno que se vuelve a
    ///   reconstruir: es derivado. Lo que no hace falta es esperar al disco
    ///   en cada página.
    /// - La tabla de texto se tira y se vuelve a crear en vez de vaciarse, y
    ///   los índices secundarios se tiran antes de cargar y se rehacen al
    ///   final: construir un árbol de una vez sobre la tabla llena es más
    ///   barato que mantenerlo ordenado fila a fila.
    /// - Una sola transacción en vez de una por tanda: eran ochenta `fsync`
    ///   y unos diez segundos esperando al disco.
    pub fn reconstruir(&mut self, lib_root: &Path) -> Result<Reindexado> {
        let thumbs_path = lib_root.join("thumbs").join("grid.pack");
        let mut thumb_map: std::collections::HashMap<String, ThumbRef> =
            std::collections::HashMap::new();
        if thumbs_path.exists() {
            let reader = crate::thumbs::PackReader::open(&thumbs_path)?;
            for (id, tref) in reader.scan() {
                thumb_map.insert(id, tref); // la última entrada gana: es la vigente
            }
        }

        // Primero las rutas, que son poca cosa: así leer puede ir en paralelo
        // y el índice no se toca hasta saber que el disco se deja recorrer.
        let mut rutas = Vec::new();
        walk_item_jsons(&lib_root.join("items"), &mut |p: &Path| {
            rutas.push(p.to_path_buf());
            Ok(())
        })?;

        self.tocado();
        self.conn.pragma_update(None, "synchronous", "OFF")?;
        let r = self.reconstruir_en(&rutas, &thumb_map);
        // Se vuelve a NORMAL pase lo que pase: una conexión que se quedara en
        // OFF seguiría así para todas las escrituras de la sesión.
        let vuelta = self.conn.pragma_update(None, "synchronous", "NORMAL");
        let informe = r?;
        vuelta?;
        self.conn.execute_batch("ANALYZE")?;
        Ok(informe)
    }

    fn reconstruir_en(
        &mut self,
        rutas: &[std::path::PathBuf],
        thumb_map: &std::collections::HashMap<String, ThumbRef>,
    ) -> Result<Reindexado> {
        use rayon::prelude::*;

        let mut informe = Reindexado::default();
        let tx = self.conn.transaction()?;
        for (nombre, _) in INDICES_SECUNDARIOS {
            tx.execute_batch(&format!("DROP INDEX IF EXISTS {nombre}"))?;
        }
        // La tabla de texto se tira y se crea de nuevo en vez de vaciarse.
        // `DELETE` en una tabla FTS5 no es gratis: deja una marca de borrado
        // por cada término de cada fila, y las inserciones de después tenían
        // que abrirse paso entre ellas al fusionar segmentos. Medido con cien
        // mil elementos, rellenar después de tirarla tarda menos de la mitad.
        tx.execute_batch(&format!(
            "DELETE FROM items; DROP TABLE IF EXISTS items_fts; {CREAR_FTS};
             DELETE FROM item_tags; DELETE FROM item_folders; DELETE FROM item_orden;
             DELETE FROM tags;"
        ))?;

        // Por tandas y no todo de golpe: acumular la biblioteca entera en
        // memoria antes de escribir funcionaba con 100.000 elementos y habría
        // reventado con un millón.
        let mut vistos: HashSet<String> = HashSet::with_capacity(rutas.len());
        for trozo in rutas.chunks(LOTE_REINDEX) {
            let leidos: Vec<std::result::Result<Item, String>> = trozo
                .par_iter()
                .map(|p| Item::read(p).map_err(|e| e.to_string()))
                .collect();
            let mut filas = Vec::with_capacity(leidos.len());
            for (ruta, r) in trozo.iter().zip(leidos) {
                match r {
                    Ok(item) => {
                        // Dos `item.json` con el mismo id —una carpeta copiada
                        // a mano— no pueden contar dos veces. Gana el primero
                        // y el otro se avisa.
                        if !vistos.insert(item.id.clone()) {
                            informe
                                .rotos
                                .push((ruta.clone(), format!("id repetido: {}", item.id)));
                            continue;
                        }
                        let t = thumb_map.get(&item.id).copied();
                        filas.push((item, t));
                    }
                    Err(e) => informe.rotos.push((ruta.clone(), e)),
                }
            }
            escribir_filas(&tx, filas.iter().map(|(i, t)| (i, *t)), true)?;
            informe.vistos += filas.len() as u64;
        }

        for (_, sql) in INDICES_SECUNDARIOS {
            tx.execute_batch(sql)?;
        }
        tx.commit()?;
        Ok(informe)
    }

    pub fn item_json_path(lib_root: &Path, id: &str) -> std::path::PathBuf {
        shard_dir(&lib_root.join("items"), id).join("item.json")
    }
}

/// El trabajo de [`Index::upsert_many`], sin abrir ni cerrar transacción: así
/// la reconstrucción entera puede ir en **una** sola, por tandas.
///
/// `vacio` dice que las tablas se acaban de vaciar. Entonces no hay nada que
/// borrar antes de insertar, y los cuatro `DELETE` por elemento —que con cien
/// mil elementos son cuatrocientas mil búsquedas en vano— se saltan.
///
/// Las sentencias van con `prepare_cached`: se llama una vez por tanda y
/// prepararlas de nuevo cada vez era repetir el mismo análisis de SQL.
fn escribir_filas<'a, I>(conn: &Connection, rows: I, vacio: bool) -> Result<()>
where
    I: IntoIterator<Item = (&'a Item, Option<ThumbRef>)>,
{
    let mut ins = conn.prepare_cached(
        r#"INSERT INTO items
           (id,name,ext,size,width,height,hash,phash,imported_at,modified_at,
            source,stars,note,origin_mode,origin_path,thumb_off,thumb_len,
            dom_r,dom_g,dom_b,color_bucket,trashed,kind,duration_ms,pages,adult)
           VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26)
           ON CONFLICT(id) DO UPDATE SET
             name=excluded.name, ext=excluded.ext, size=excluded.size,
             width=excluded.width, height=excluded.height, hash=excluded.hash,
             phash=excluded.phash, modified_at=excluded.modified_at,
             source=excluded.source, stars=excluded.stars, note=excluded.note,
             origin_mode=excluded.origin_mode, origin_path=excluded.origin_path,
             thumb_off=COALESCE(excluded.thumb_off, items.thumb_off),
             thumb_len=COALESCE(excluded.thumb_len, items.thumb_len),
             dom_r=excluded.dom_r, dom_g=excluded.dom_g, dom_b=excluded.dom_b,
             color_bucket=excluded.color_bucket, trashed=excluded.trashed,
             kind=excluded.kind, duration_ms=excluded.duration_ms,
             pages=excluded.pages, adult=excluded.adult
           RETURNING rowid"#,
    )?;
    let mut del_fts = conn.prepare_cached("DELETE FROM items_fts WHERE rowid = ?1")?;
    let mut ins_fts = conn.prepare_cached("INSERT INTO items_fts (rowid, text) VALUES (?1, ?2)")?;
    let mut del_tags = conn.prepare_cached("DELETE FROM item_tags WHERE item_id = ?1")?;
    let mut ins_tag = conn.prepare_cached("INSERT OR IGNORE INTO tags (name) VALUES (?1)")?;
    let mut get_tag = conn.prepare_cached("SELECT id FROM tags WHERE name = ?1")?;
    let mut link_tag =
        conn.prepare_cached("INSERT OR IGNORE INTO item_tags (item_id, tag_id) VALUES (?1, ?2)")?;
    let mut del_orden = conn.prepare_cached("DELETE FROM item_orden WHERE item_id = ?1")?;
    let mut ins_orden =
        conn.prepare_cached("INSERT INTO item_orden (item_id, ctx, pos) VALUES (?1, ?2, ?3)")?;
    let mut del_folders = conn.prepare_cached("DELETE FROM item_folders WHERE item_id = ?1")?;
    let mut link_folder = conn.prepare_cached(
        "INSERT OR IGNORE INTO item_folders (item_id, folder_id) VALUES (?1, ?2)",
    )?;

    // Caché de etiquetas: sin ella, cada elemento hacía dos consultas
    // por etiqueta. Con 100.000 elementos eso son cientos de miles de
    // idas y vueltas que no aportan nada.
    let mut tag_ids: std::collections::HashMap<String, i64> =
        std::collections::HashMap::new();

    for (item, thumb) in rows {
        let dom = item.palette.first().map(|p| p.rgb);
        let bucket = crate::image_ops::color_bucket(&item.palette);
        let phash = u64::from_str_radix(&item.phash, 16).ok().map(|v| v as i64);
        let rowid: i64 = ins.query_row(
            params![
                item.id,
                item.name,
                item.ext,
                item.size as i64,
                item.width,
                item.height,
                item.hash,
                phash,
                item.imported_at_ms() as i64,
                item.modified_at_ms() as i64,
                item.source,
                item.stars,
                item.note,
                item.origin.mode.as_str(),
                item.origin.path,
                thumb.map(|t| t.offset as i64),
                thumb.map(|t| t.len as i64),
                dom.map(|c| c[0]),
                dom.map(|c| c[1]),
                dom.map(|c| c[2]),
                bucket,
                item.trashed as i32,
                item.kind.as_str(),
                item.duration_ms.map(|d| d as i64),
                item.pages,
                item.adult as i32,
            ],
            |r| r.get(0),
        )?;

        if !vacio {
            del_fts.execute(params![rowid])?;
        }
        ins_fts.execute(params![rowid, item.search_text()])?;
        if !vacio {
            del_orden.execute(params![item.id])?;
        }
        for (ctx, pos) in &item.orden {
            ins_orden.execute(params![item.id, ctx, pos])?;
        }

        if !vacio {
            del_tags.execute(params![item.id])?;
        }
        for t in &item.tags {
            let tid = match tag_ids.get(t) {
                Some(id) => *id,
                None => {
                    ins_tag.execute(params![t])?;
                    let id: i64 = get_tag.query_row(params![t], |r| r.get(0))?;
                    tag_ids.insert(t.clone(), id);
                    id
                }
            };
            link_tag.execute(params![item.id, tid])?;
        }

        if !vacio {
            del_folders.execute(params![item.id])?;
        }
        for f in &item.folders {
            link_folder.execute(params![item.id, f])?;
        }
    }
    Ok(())
}

fn walk_item_jsons(dir: &Path, f: &mut impl FnMut(&Path) -> Result<()>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    let rd = std::fs::read_dir(dir).map_err(|e| crate::Error::io(dir, e))?;
    for entry in rd {
        let entry = entry.map_err(|e| crate::Error::io(dir, e))?;
        let path = entry.path();
        let ft = entry.file_type().map_err(|e| crate::Error::io(&path, e))?;
        if ft.is_dir() {
            walk_item_jsons(&path, f)?;
        } else if path.file_name().map(|n| n == "item.json").unwrap_or(false) {
            f(&path)?;
        }
    }
    Ok(())
}

/// Cuántos elementos puede tener un cubo antes de darlo por degenerado.
const TOPE_CUBO: usize = 4096;

/// Lo que encontró la búsqueda de duplicados.
#[derive(Debug, Default)]
pub struct Duplicados {
    pub grupos: Vec<Grupo>,
    /// Cubos que se dejaron sin comparar por ser demasiado grandes. Si esto no
    /// es cero, la respuesta está incompleta y hay que decirlo.
    pub saltados: usize,
}

/// Un puñado de elementos que son el mismo o casi.
#[derive(Debug, Clone)]
pub struct Grupo {
    pub ids: Vec<String>,
    /// `true` si son idénticos byte a byte; `false` si solo se parecen.
    pub exacto: bool,
}

/// Conjuntos disjuntos, para juntar parejas en grupos.
///
/// Hace falta porque «parecerse» no es transitivo por sí solo: si A se parece a
/// B y B a C, se quieren los tres en el mismo grupo aunque A y C estén a cuatro
/// bits. Al revisar duplicados eso es lo que uno espera ver.
struct Union {
    padre: Vec<usize>,
}

impl Union {
    fn nueva(n: usize) -> Union {
        Union {
            padre: (0..n).collect(),
        }
    }
    fn raiz(&mut self, mut i: usize) -> usize {
        while self.padre[i] != i {
            // Aplastar por el camino: la siguiente consulta es directa.
            self.padre[i] = self.padre[self.padre[i]];
            i = self.padre[i];
        }
        i
    }
    fn unir(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.raiz(a), self.raiz(b));
        if ra != rb {
            self.padre[ra] = rb;
        }
    }
}

/// Siempre cualificadas con el alias `i`: al unir con `items_fts` hay dos
/// columnas `id` y SQLite se queja con razón.
const SELECT_COLS: &str = "i.id,i.name,i.ext,i.size,i.width,i.height,i.stars,\
                           i.thumb_off,i.thumb_len,i.dom_r,i.dom_g,i.dom_b,i.imported_at,\
                           i.kind,i.duration_ms,i.origin_mode,i.origin_path,i.adult";

fn hit_from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<QueryHit> {
    Ok(QueryHit {
        id: r.get(0)?,
        name: r.get(1)?,
        ext: r.get(2)?,
        size: r.get::<_, i64>(3)? as u64,
        width: r.get(4)?,
        height: r.get(5)?,
        stars: r.get(6)?,
        thumb_off: r.get::<_, Option<i64>>(7)?.map(|v| v as u64),
        thumb_len: r.get::<_, Option<i64>>(8)?.map(|v| v as u32),
        dominant: match (
            r.get::<_, Option<u8>>(9)?,
            r.get::<_, Option<u8>>(10)?,
            r.get::<_, Option<u8>>(11)?,
        ) {
            (Some(a), Some(b), Some(c)) => Some([a, b, c]),
            _ => None,
        },
        imported_at_ms: r.get::<_, i64>(12)? as u64,
        kind: crate::medio::Familia::parse(&r.get::<_, String>(13)?).unwrap_or_default(),
        // En segundos y no en milisegundos: lo que se pinta en la celda es
        // «1:23», y un `u32` de segundos llega a los ciento treinta y seis años.
        duracion_s: r
            .get::<_, Option<i64>>(14)?
            .map(|ms| (ms / 1000).clamp(0, u32::MAX as i64) as u32),
        // Solo si de verdad vive fuera. En modo copia la columna también trae
        // una ruta —de dónde salió—, y esa ya no apunta a nada que valga.
        ruta_ref: match r.get::<_, String>(15)?.as_str() {
            "ref" => r.get::<_, Option<String>>(16)?,
            _ => None,
        },
        adulto: r.get::<_, i64>(17)? != 0,
    })
}

fn build_sql(q: &Query) -> (String, Vec<Box<dyn rusqlite::ToSql>>) {
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    let mut sql = format!("SELECT {SELECT_COLS} FROM items i");
    // El orden a mano necesita la posición de cada uno en este sitio. Va lo
    // primero para que su `?` sea el primero de los argumentos.
    if q.sort == SortBy::Manual {
        sql.push_str(" LEFT JOIN item_orden o ON o.item_id = i.id AND o.ctx = ?");
        args.push(Box::new(q.folder.clone().unwrap_or_default()));
    }
    // Lo primero de todo y siempre: o se ve la biblioteca o se ve la papelera,
    // nunca las dos mezcladas. Un elemento tirado que reaparece entre los
    // demás es la forma más rápida de perderle la confianza a un programa.
    let mut wheres: Vec<String> = vec![if q.papelera {
        "i.trashed = 1".into()
    } else {
        "i.trashed = 0".to_string()
    }];
    if let Some(a) = q.adulto {
        wheres.push(format!("i.adult = {}", if a { 1 } else { 0 }));
    }
    if let Some(e) = q.etiquetado {
        wheres.push(format!(
            "{} EXISTS (SELECT 1 FROM item_tags it WHERE it.item_id = i.id)",
            if e { "" } else { "NOT" }
        ));
    }

    let mut hay_texto = false;
    // Un texto hecho solo de signos se queda en nada después de limpiarlo, y
    // es lo mismo que no haber escrito: `MATCH ''` sería un error de sintaxis.
    let fts = q.text.as_deref().map(fts_prefix_query).filter(|t| !t.is_empty());
    if let Some(fts) = fts {
        // La fila de FTS comparte rowid con la de items: unir es una búsqueda
        // por clave primaria, no un escaneo.
        sql.push_str(" JOIN items_fts ON items_fts.rowid = i.rowid");
        wheres.push("items_fts MATCH ?".into());
        args.push(Box::new(fts));
        hay_texto = true;
    }
    if !q.familias.is_empty() {
        let marks = vec!["?"; q.familias.len()].join(",");
        wheres.push(format!("i.kind IN ({marks})"));
        for f in &q.familias {
            args.push(Box::new(f.as_str()));
        }
    }
    if !q.exts.is_empty() {
        let marks = vec!["?"; q.exts.len()].join(",");
        wheres.push(format!("i.ext IN ({marks})"));
        for e in &q.exts {
            args.push(Box::new(e.to_ascii_lowercase()));
        }
    }
    for t in &q.tags {
        wheres.push(
            "i.id IN (SELECT it.item_id FROM item_tags it JOIN tags tg ON tg.id = it.tag_id WHERE tg.name = ?)"
                .into(),
        );
        args.push(Box::new(t.clone()));
    }
    if let Some(f) = &q.folder {
        if q.folder_recursive {
            // Pinchar en una carpeta enseña también lo que hay en sus hijas,
            // que es lo que espera cualquiera que haya usado un explorador de
            // archivos. `UNION` (y no `UNION ALL`) corta en seco un ciclo en la
            // tabla: sin eso, un `folders.json` editado a mano colgaría la
            // consulta en vez de dar un resultado raro.
            wheres.push(
                "i.id IN (
                     WITH RECURSIVE sub(id) AS (
                         SELECT ?
                         UNION SELECT f.id FROM folders f JOIN sub ON f.parent = sub.id
                     )
                     SELECT item_id FROM item_folders WHERE folder_id IN (SELECT id FROM sub)
                 )"
                .into(),
            );
        } else {
            wheres.push("i.id IN (SELECT item_id FROM item_folders WHERE folder_id = ?)".into());
        }
        args.push(Box::new(f.clone()));
    }
    if let Some(s) = q.min_stars {
        wheres.push("i.stars >= ?".into());
        args.push(Box::new(s));
    }
    if let Some(w) = q.min_width {
        wheres.push("i.width >= ?".into());
        args.push(Box::new(w));
    }
    if let Some(h) = q.min_height {
        wheres.push("i.height >= ?".into());
        args.push(Box::new(h));
    }
    if let Some(sz) = q.min_size {
        wheres.push("i.size >= ?".into());
        args.push(Box::new(sz as i64));
    }
    if let Some(s) = q.max_stars {
        wheres.push("i.stars <= ?".into());
        args.push(Box::new(s));
    }
    if let Some(w) = q.max_width {
        wheres.push("i.width <= ?".into());
        args.push(Box::new(w));
    }
    if let Some(h) = q.max_height {
        wheres.push("i.height <= ?".into());
        args.push(Box::new(h));
    }
    if let Some(sz) = q.max_size {
        wheres.push("i.size <= ?".into());
        args.push(Box::new(sz as i64));
    }
    if let Some(t) = q.desde_ms {
        wheres.push("i.imported_at >= ?".into());
        args.push(Box::new(t as i64));
    }
    if let Some(t) = q.hasta_ms {
        wheres.push("i.imported_at <= ?".into());
        args.push(Box::new(t as i64));
    }
    match q.orientation {
        Some(Orientation::Landscape) => wheres.push("i.width > i.height".into()),
        Some(Orientation::Portrait) => wheres.push("i.height > i.width".into()),
        Some(Orientation::Square) => wheres.push("i.width = i.height".into()),
        None => {}
    }
    if let Some(c) = q.color {
        // Preselección por cubo de color y sus vecinos inmediatos.
        let bucket =
            crate::image_ops::color_bucket(&[crate::item::PaletteEntry { rgb: c, w: 1.0 }])
                .unwrap_or(0);
        let mut set = Vec::new();
        for dl in -1..=1i64 {
            for da in -1..=1i64 {
                for db in -1..=1i64 {
                    set.push(bucket + dl * 144 + da * 12 + db);
                }
            }
        }
        let marks = vec!["?"; set.len()].join(",");
        wheres.push(format!("i.color_bucket IN ({marks})"));
        for b in set {
            args.push(Box::new(b));
        }
    }

    sql.push_str(" WHERE ");
    sql.push_str(&wheres.join(" AND "));

    // Orden por defecto: el id ULID ya es cronológico, así no ordenamos nada.
    let order = match q.sort {
        SortBy::ImportedDesc => "i.id DESC",
        SortBy::ImportedAsc => "i.id ASC",
        SortBy::NameAsc => "i.name COLLATE NOCASE ASC",
        SortBy::NameDesc => "i.name COLLATE NOCASE DESC",
        SortBy::SizeDesc => "i.size DESC",
        SortBy::SizeAsc => "i.size ASC",
        SortBy::StarsDesc => "i.stars DESC, i.id DESC",
        SortBy::Random => "",
        // Sin posición puesta, la que le toca por fecha: así pasar a «a mano»
        // no mueve nada, y solo se mueve lo que se arrastra.
        SortBy::Manual => "COALESCE(o.pos, -CAST(i.imported_at AS REAL)) ASC, i.id DESC",
    };
    if q.sort == SortBy::Random {
        sql.push_str(" ORDER BY (i.rowid * 2654435761) % 4294967291");
    } else if hay_texto && q.sort == SortBy::ImportedDesc && q.color.is_none() {
        // Buscando texto, lo esperable es lo más pertinente primero, no lo más
        // reciente. `rank` de FTS5 es bm25 y ordena ascendente (mejor = menor).
        sql.push_str(" ORDER BY items_fts.rank, i.id DESC");
    } else if q.color.is_some() {
        // Con búsqueda por color el orden final lo pone la distancia real;
        // aquí solo acotamos el candidato de forma estable.
        sql.push_str(" ORDER BY i.id DESC");
    } else {
        sql.push_str(" ORDER BY ");
        sql.push_str(order);
    }

    // Con color pedimos más candidatos porque luego reordenamos por distancia.
    // El -1 es el "sin límite" de SQLite, y es lo que hace que `limit: 0`
    // signifique "todos" sin tener que escribir un número mágico grande.
    let limit: i64 = if q.limit == 0 {
        -1
    } else if q.color.is_some() {
        (q.limit * 8).min(20_000) as i64
    } else {
        q.limit as i64
    };
    sql.push_str(" LIMIT ? OFFSET ?");
    args.push(Box::new(limit));
    args.push(Box::new(q.offset as i64));
    (sql, args)
}

/// Ordena los candidatos por cercanía perceptual al color pedido.
///
/// Tiene su propia función por una razón medida: la primera versión llamaba a
/// `color_distance` **dentro** del comparador, y ordenar 1.600 candidatos
/// significaba unas 34.000 conversiones a Oklab —tres `powf` y tres `cbrt`
/// cada una— por consulta. Eran 25 de los 32 ms que tardaba la búsqueda por
/// color. Aquí la distancia se calcula una vez por elemento y se ordena sobre
/// el número ya calculado.
fn ordenar_por_color(hits: Vec<QueryHit>, target: [u8; 3], limite: usize) -> Vec<QueryHit> {
    let t = crate::image_ops::srgb_to_oklab(target[0], target[1], target[2]);
    let mut claves: Vec<(f32, usize)> = hits
        .iter()
        .enumerate()
        .map(|(i, h)| (h.color_distance2(&t), i))
        .collect();

    // Solo se necesitan los `limite` mejores; el resto no hace falta ni
    // ordenarlo. `select_nth_unstable_by` los separa en tiempo lineal.
    let n = limite.min(claves.len());
    if n < claves.len() {
        claves.select_nth_unstable_by(n, |a, b| a.0.total_cmp(&b.0));
        claves.truncate(n);
    }
    claves.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));

    // Mover en vez de clonar: cada índice aparece una sola vez.
    let mut origen: Vec<Option<QueryHit>> = hits.into_iter().map(Some).collect();
    let mut salida = Vec::with_capacity(n);
    for (_, i) in claves {
        if let Some(h) = origen[i].take() {
            salida.push(h);
        }
    }
    salida
}

/// Convierte lo que escribe una persona en una consulta FTS5 con prefijo:
/// "gato az" busca gato* AND az*. Escapamos comillas para que nadie pueda
/// inyectar operadores de FTS sin querer.
///
/// Las palabras sin ninguna letra ni cifra —un guion suelto, una comilla, un
/// «·»— se tiran. El tokenizador no saca de ellas ningún término, así que
/// llegaban a FTS como `""*`, que no coincide con nada, y por el `AND` dejaban
/// sin resultados la búsqueda entera: «gato -» no encontraba al gato.
fn fts_prefix_query(input: &str) -> String {
    input
        .split_whitespace()
        .map(|w| w.replace('"', ""))
        .filter(|w| w.chars().any(char::is_alphanumeric))
        .map(|w| format!("\"{w}\"*"))
        .collect::<Vec<_>>()
        .join(" AND ")
}

#[cfg(test)]
mod tests {
    fn hit_de_color(id: &str, rgb: Option<[u8; 3]>) -> QueryHit {
        QueryHit {
            id: id.into(),
            name: id.into(),
            ext: "jpg".into(),
            size: 1,
            width: 10,
            height: 10,
            stars: 0,
            thumb_off: None,
            thumb_len: None,
            dominant: rgb,
            imported_at_ms: 0,
            kind: Default::default(),
            duracion_s: None,
            ruta_ref: None,
            adulto: false,
        }
    }

    /// La marca de +18 sobrevive al índice y se puede filtrar por ella.
    ///
    /// Es lo que sostiene el modo seguro: si la marca no llegara a la vista, la
    /// celda no tendría con qué decidir si tapa o no.
    #[test]
    fn lo_marcado_como_adulto_se_encuentra_y_se_esquiva() {
        let mut idx = Index::open_in_memory().unwrap();
        let mut marcados = Vec::new();
        for i in 0..4 {
            let mut item = crate::item::Item::new(
                crate::id::new_id(),
                format!("foto {i}"),
                "jpg".into(),
                10,
                crate::item::OriginMode::Copy,
            );
            item.adult = i % 2 == 0;
            if item.adult {
                marcados.push(item.id.clone());
            }
            idx.upsert_many(std::iter::once((&item, None))).unwrap();
        }

        let solo_marcados = Query {
            adulto: Some(true),
            ..Default::default()
        };
        let hits = idx.search(&solo_marcados).unwrap();
        assert_eq!(hits.len(), 2);
        assert!(hits.iter().all(|h| h.adulto));
        for h in &hits {
            assert!(marcados.contains(&h.id));
        }

        let sin_marcar = Query {
            adulto: Some(false),
            ..Default::default()
        };
        let hits = idx.search(&sin_marcar).unwrap();
        assert_eq!(hits.len(), 2);
        assert!(hits.iter().all(|h| !h.adulto));

        // Sin pedir nada salen los cuatro: es una marca, no un filtro puesto.
        assert_eq!(idx.search(&Query::default()).unwrap().len(), 4);
        assert_eq!(idx.contar_adultos().unwrap(), 2);
    }

    /// Un índice de antes del orden a mano: versión 3 y sin `item_orden`.
    /// Abrirlo tiene que crear la tabla, no dejar que la primera consulta
    /// ordenada falle con «no such table».
    #[test]
    fn un_indice_de_la_version_3_gana_la_tabla_del_orden() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("index.sqlite");
        {
            let idx = Index::open(&ruta).unwrap();
            idx.conn.execute_batch("DROP TABLE item_orden; PRAGMA user_version = 3;").unwrap();
        }
        let idx = Index::open(&ruta).unwrap();
        let q = Query { sort: SortBy::Manual, ..Default::default() };
        idx.search(&q).expect("con la tabla creada, ordenar a mano funciona");
    }

    #[test]
    fn sin_etiquetar_trae_justo_lo_que_no_tiene_etiquetas() {
        let mut idx = Index::open_in_memory().unwrap();
        for i in 0..3 {
            let mut item = crate::item::Item::new(
                crate::id::new_id(),
                format!("foto {i}"),
                "jpg".into(),
                10,
                crate::item::OriginMode::Copy,
            );
            if i == 0 {
                item.tags = vec!["cartel".into()];
            }
            idx.upsert_many(std::iter::once((&item, None))).unwrap();
        }
        let sin = Query { etiquetado: Some(false), ..Default::default() };
        assert_eq!(idx.search(&sin).unwrap().len(), 2);
        let con = Query { etiquetado: Some(true), ..Default::default() };
        assert_eq!(idx.search(&con).unwrap().len(), 1);
    }

    #[test]
    fn limite_cero_significa_todos() {
        // Contra el índice de verdad, que es donde importa: sin tope tienen que
        // salir los tres, no cero.
        let mut idx = Index::open_in_memory().unwrap();
        for i in 0..3 {
            let item = crate::item::Item::new(
                crate::id::new_id(),
                format!("foto {i}"),
                "jpg".into(),
                10,
                crate::item::OriginMode::Copy,
            );
            idx.upsert(&item, None).unwrap();
        }
        let todos = Query {
            limit: 0,
            ..Default::default()
        };
        assert_eq!(idx.search(&todos).unwrap().len(), 3);
        let dos = Query {
            limit: 2,
            ..Default::default()
        };
        assert_eq!(idx.search(&dos).unwrap().len(), 2);
    }

    #[test]
    fn una_consulta_parcial_hereda_lo_que_falta() {
        let q: Query = serde_json::from_str(r#"{"text":"gato"}"#).unwrap();
        assert_eq!(q.text.as_deref(), Some("gato"));
        assert_eq!(q.limit, 200);
        assert!(q.folder_recursive);
        let q: Query = serde_json::from_str("{}").unwrap();
        assert_eq!(q.sort, SortBy::ImportedDesc);
    }

    #[test]
    fn el_color_ordena_de_cerca_a_lejos_y_recorta() {
        // Objetivo verde. El orden esperado es evidente a ojo, que es justo lo
        // que tiene que devolver una búsqueda por color.
        let objetivo = [0, 200, 0];
        let hits = vec![
            hit_de_color("rojo", Some([220, 0, 0])),
            hit_de_color("verde-claro", Some([40, 210, 40])),
            hit_de_color("sin-paleta", None),
            hit_de_color("verde", Some([0, 198, 4])),
            hit_de_color("azul", Some([0, 0, 220])),
        ];
        let r = ordenar_por_color(hits, objetivo, 3);
        assert_eq!(r.len(), 3, "el límite tiene que recortar");
        let ids: Vec<&str> = r.iter().map(|h| h.id.as_str()).collect();
        assert_eq!(ids, ["verde", "verde-claro", "rojo"]);
    }

    #[test]
    fn un_elemento_sin_paleta_se_va_al_final_y_no_desaparece() {
        let hits = vec![
            hit_de_color("sin-paleta", None),
            hit_de_color("gris", Some([128, 128, 128])),
        ];
        let r = ordenar_por_color(hits, [130, 130, 130], 10);
        assert_eq!(r.len(), 2, "sin límite no se pierde nada");
        assert_eq!(r[0].id, "gris");
        assert_eq!(r[1].id, "sin-paleta");
    }

    #[test]
    fn pedir_mas_de_lo_que_hay_no_revienta() {
        let r = ordenar_por_color(vec![hit_de_color("uno", Some([1, 2, 3]))], [9, 9, 9], 500);
        assert_eq!(r.len(), 1);
        assert!(ordenar_por_color(Vec::new(), [0, 0, 0], 10).is_empty());
    }

    use super::*;
    use crate::item::{Item, OriginMode, PaletteEntry};

    fn item(id: &str, name: &str, ext: &str, w: u32, h: u32) -> Item {
        let mut it = Item::new(id.into(), name.into(), ext.into(), 1000, OriginMode::Copy);
        it.width = w;
        it.height = h;
        it
    }

    fn idx_con_datos() -> Index {
        let mut idx = Index::open_in_memory().unwrap();
        let mut a = item(&crate::id::id_at(1000, 1), "gato azul", "jpg", 1920, 1080);
        a.tags = vec!["felino".into(), "azul".into()];
        a.stars = 5;
        a.palette = vec![PaletteEntry {
            rgb: [20, 40, 200],
            w: 0.8,
        }];
        let mut b = item(&crate::id::id_at(2000, 2), "perro diseño", "png", 800, 1200);
        b.tags = vec!["canino".into()];
        b.note = Some("bocetos para el moodboard".into());
        idx.upsert_many([(&a, None), (&b, None)]).unwrap();
        idx
    }

    #[test]
    fn cuenta_y_recupera() {
        let idx = idx_con_datos();
        assert_eq!(idx.count().unwrap(), 2);
    }

    #[test]
    fn busqueda_por_texto_con_prefijo() {
        let idx = idx_con_datos();
        let q = Query {
            text: Some("gat".into()),
            ..Default::default()
        };
        let hits = idx.search(&q).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "gato azul");
    }

    #[test]
    fn busqueda_ignora_acentos() {
        let idx = idx_con_datos();
        let q = Query {
            text: Some("diseno".into()),
            ..Default::default()
        };
        assert_eq!(idx.search(&q).unwrap().len(), 1);
    }

    #[test]
    fn busca_en_notas_y_etiquetas() {
        let idx = idx_con_datos();
        for term in ["moodboard", "felino"] {
            let q = Query {
                text: Some(term.into()),
                ..Default::default()
            };
            assert_eq!(idx.search(&q).unwrap().len(), 1, "fallo con {term}");
        }
    }

    #[test]
    fn filtra_por_orientacion_y_estrellas() {
        let idx = idx_con_datos();
        let apaisadas = Query {
            orientation: Some(Orientation::Landscape),
            ..Default::default()
        };
        assert_eq!(idx.search(&apaisadas).unwrap().len(), 1);

        let buenas = Query {
            min_stars: Some(4),
            ..Default::default()
        };
        assert_eq!(idx.search(&buenas).unwrap().len(), 1);
    }

    #[test]
    fn el_orden_por_defecto_es_lo_mas_reciente_primero() {
        let idx = idx_con_datos();
        let hits = idx.search(&Query::default()).unwrap();
        assert_eq!(hits[0].name, "perro diseño");
    }

    #[test]
    fn resuelve_ids_por_prefijo() {
        let idx = idx_con_datos();
        let todos = idx.search(&Query::default()).unwrap();
        let id = &todos[0].id;

        // Prefijo corto en minúsculas, como el que se copia de un listado.
        let corto = id[..8].to_lowercase();
        assert_eq!(idx.ids_con_prefijo(&corto, 5).unwrap(), vec![id.clone()]);

        // Un prefijo que no existe no devuelve nada, y el vacío tampoco.
        assert!(idx.ids_con_prefijo("ZZZZZZZZ", 5).unwrap().is_empty());
        assert!(idx.ids_con_prefijo("", 5).unwrap().is_empty());
    }

    #[test]
    fn resuelve_ids_por_sufijo() {
        let idx = idx_con_datos();
        let todos = idx.search(&Query::default()).unwrap();
        let id = &todos[0].id;
        let corto = id[18..].to_lowercase();
        assert_eq!(idx.ids_con_sufijo(&corto, 5).unwrap(), vec![id.clone()]);
        assert!(idx.ids_con_sufijo("", 5).unwrap().is_empty());
    }

    #[test]
    fn un_prefijo_ambiguo_devuelve_varios() {
        let idx = idx_con_datos();
        // Los dos elementos de prueba comparten el arranque del ULID.
        let varios = idx.ids_con_prefijo("0", 5).unwrap();
        assert_eq!(varios.len(), 2);
    }

    #[test]
    fn filtra_por_etiqueta() {
        let idx = idx_con_datos();
        let q = Query {
            tags: vec!["canino".into()],
            ..Default::default()
        };
        assert_eq!(idx.search(&q).unwrap().len(), 1);
    }

    #[test]
    fn las_comillas_no_rompen_la_busqueda() {
        let idx = idx_con_datos();
        let q = Query {
            text: Some("\"gato OR".into()),
            ..Default::default()
        };
        // No debe explotar ni devolver la biblioteca entera.
        let hits = idx.search(&q).unwrap();
        assert!(hits.len() <= 1);
    }
}

#[cfg(test)]
mod tests_duplicados {
    use super::*;
    use crate::item::{Item, OriginMode};

    fn item(id: &str, hash: &str, phash: u64) -> Item {
        let mut it = Item::new(
            id.to_string(),
            format!("foto {id}"),
            "jpg".into(),
            10,
            OriginMode::Copy,
        );
        it.hash = hash.to_string();
        it.phash = format!("{phash:016x}");
        it
    }

    fn indice(items: &[Item]) -> Index {
        let mut idx = Index::open_in_memory().unwrap();
        idx.upsert_many(items.iter().map(|i| (i, None))).unwrap();
        idx
    }

    #[test]
    fn los_identicos_salen_juntos_y_marcados_como_exactos() {
        let items = vec![
            item("a", "mismo", 0x0000_0000_0000_0000),
            item("b", "mismo", 0x0000_0000_0000_0000),
            item("c", "otro", 0xFFFF_FFFF_FFFF_FFFF),
        ];
        let g = indice(&items).duplicados(3).unwrap().grupos;
        let exactos: Vec<_> = g.iter().filter(|x| x.exacto).collect();
        assert_eq!(exactos.len(), 1);
        assert_eq!(exactos[0].ids.len(), 2);
    }

    #[test]
    fn los_parecidos_salen_aunque_el_archivo_sea_distinto() {
        // Dos bits de diferencia: una recompresión, un reescalado.
        let items = vec![
            item("a", "h1", 0x0F0F_0F0F_0F0F_0F0F),
            item("b", "h2", 0x0F0F_0F0F_0F0F_0F0D),
            item("c", "h3", 0x1234_5678_9ABC_DEF0),
        ];
        let g = indice(&items).duplicados(3).unwrap().grupos;
        let parecidos: Vec<_> = g.iter().filter(|x| !x.exacto).collect();
        assert_eq!(parecidos.len(), 1, "salieron {g:?}");
        assert_eq!(parecidos[0].ids.len(), 2);
    }

    #[test]
    fn una_cadena_de_parecidos_acaba_en_un_solo_grupo() {
        // A se parece a B, B a C, pero A y C están a cuatro bits: aun así los
        // tres son el mismo montón para quien los revisa.
        let items = vec![
            item("a", "h1", 0b0000),
            item("b", "h2", 0b0011),
            item("c", "h3", 0b1111),
        ];
        let g = indice(&items).duplicados(2).unwrap().grupos;
        let parecidos: Vec<_> = g.iter().filter(|x| !x.exacto).collect();
        assert_eq!(parecidos.len(), 1);
        assert_eq!(parecidos[0].ids.len(), 3);
    }

    #[test]
    fn lo_que_no_se_parece_no_se_junta() {
        let items = vec![
            item("a", "h1", 0x0000_0000_0000_0000),
            item("b", "h2", 0xFFFF_FFFF_FFFF_FFFF),
        ];
        let g = indice(&items).duplicados(3).unwrap().grupos;
        assert!(g.is_empty());
    }

    #[test]
    fn un_exacto_no_se_cuenta_dos_veces_como_parecido() {
        let items = vec![item("a", "mismo", 42), item("b", "mismo", 42)];
        let g = indice(&items).duplicados(3).unwrap().grupos;
        assert_eq!(g.len(), 1, "un solo grupo, el exacto: {g:?}");
        assert!(g[0].exacto);
    }

    #[test]
    fn con_distancia_cero_solo_se_miran_los_identicos() {
        let items = vec![item("a", "h1", 0b0000), item("b", "h2", 0b0001)];
        assert!(indice(&items).duplicados(0).unwrap().grupos.is_empty());
    }

    #[test]
    fn el_orden_a_mano_sigue_las_posiciones_y_sin_ellas_la_fecha() {
        let mut items = vec![
            item("a", "ha", 1),
            item("b", "hb", 2),
            item("c", "hc", 3),
        ];
        let orden = |idx: &Index, carpeta: Option<&str>| -> Vec<String> {
            let q = Query {
                sort: SortBy::Manual,
                folder: carpeta.map(str::to_string),
                limit: 0,
                ..Default::default()
            };
            idx.search(&q).unwrap().into_iter().map(|h| h.id).collect()
        };
        // Sin posiciones, como «recientes».
        let recientes: Vec<String> = indice(&items)
            .search(&Query { limit: 0, ..Default::default() })
            .unwrap()
            .into_iter()
            .map(|h| h.id)
            .collect();
        assert_eq!(orden(&indice(&items), None), recientes);

        // «a» delante del todo en Todo; en la carpeta «x» no se ha tocado.
        items[0].orden.insert(String::new(), -1e15);
        for it in items.iter_mut() {
            it.folders = vec!["x".into()];
        }
        let idx = indice(&items);
        assert_eq!(orden(&idx, None)[0], "a");
        let en_x = orden(&idx, Some("x"));
        assert_eq!(en_x, recientes, "cada sitio lleva su orden");
    }

    #[test]
    fn repetidos_si_trae_solo_los_grupos_y_cada_uno_seguido() {
        let items = vec![
            item("solo", "unico", 0xFFFF_0000_FFFF_0000),
            item("c1", "copia", 7),
            item("otro", "x", 0x0F0F_0F0F_0F0F_0F0F),
            item("c2", "copia", 7),
        ];
        let idx = indice(&items);
        let q = Query {
            repetidos: true,
            limit: 0,
            ..Default::default()
        };
        let ids: Vec<String> = idx.search(&q).unwrap().into_iter().map(|h| h.id).collect();
        assert_eq!(ids.len(), 2, "solo el grupo: {ids:?}");
        assert!(ids.contains(&"c1".to_string()) && ids.contains(&"c2".to_string()));
        assert_eq!(idx.contar(&q).unwrap(), 2);

        // Con otro filtro que deja a uno fuera, el grupo deja de serlo.
        let q = Query {
            repetidos: true,
            text: Some("c1".into()),
            limit: 0,
            ..Default::default()
        };
        assert!(idx.search(&q).unwrap().is_empty());
    }

    #[test]
    fn la_papelera_no_cuenta_para_los_duplicados() {
        let mut items = vec![item("a", "mismo", 1), item("b", "mismo", 1)];
        items[1].trashed = true;
        assert!(indice(&items).duplicados(3).unwrap().grupos.is_empty());
    }
}

#[cfg(test)]
mod tests_reconstruir {
    use super::*;
    use crate::item::{Item, OriginMode};

    fn item(nombre: &str) -> Item {
        Item::new(crate::id::new_id(), nombre.into(), "jpg".into(), 10, OriginMode::Copy)
    }

    #[test]
    fn la_basura_suelta_no_vacia_la_busqueda() {
        assert_eq!(fts_prefix_query("gato -"), "\"gato\"*");
        assert_eq!(fts_prefix_query("\" gato \""), "\"gato\"*");
        assert_eq!(fts_prefix_query("- · \""), "");

        let mut idx = Index::open_in_memory().unwrap();
        idx.upsert(&item("gato pardo"), None).unwrap();
        idx.upsert(&item("perro"), None).unwrap();
        for texto in ["gato -", "gato \"", "gato · -"] {
            let q = Query {
                text: Some(texto.into()),
                ..Default::default()
            };
            let hits = idx.search(&q).unwrap();
            assert_eq!(hits.len(), 1, "«{texto}» tiene que encontrar al gato");
            assert_eq!(hits[0].name, "gato pardo");
        }
        // Solo signos es como no haber escrito nada, no un error de FTS.
        let q = Query {
            text: Some("- \"".into()),
            ..Default::default()
        };
        assert_eq!(idx.search(&q).unwrap().len(), 2);
    }

    /// La lista de índices que la reconstrucción tira y rehace tiene que ser
    /// la misma que crea `migrate`; si no, un índice nuevo desaparecería en el
    /// primer reindex sin que nadie lo notara.
    #[test]
    fn la_lista_de_indices_coincide_con_la_de_migrate() {
        let idx = Index::open_in_memory().unwrap();
        let mut st = idx
            .conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type = 'index'
                 AND sql IS NOT NULL AND tbl_name IN ('items', 'item_tags', 'item_folders')",
            )
            .unwrap();
        let mut de_migrate: Vec<String> = st
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        de_migrate.sort();
        let mut lista: Vec<String> = INDICES_SECUNDARIOS.iter().map(|(n, _)| n.to_string()).collect();
        lista.sort();
        assert_eq!(lista, de_migrate);
    }

    fn biblioteca_en_disco(n: usize) -> (tempfile::TempDir, std::path::PathBuf, Vec<String>) {
        let dir = tempfile::tempdir().unwrap();
        let raiz = dir.path().to_path_buf();
        let mut ids = Vec::new();
        for i in 0..n {
            let mut it = item(&format!("foto {i}"));
            it.tags = vec![format!("t{}", i % 3)];
            it.orden.insert(String::new(), i as f64);
            it.write(&Index::item_json_path(&raiz, &it.id)).unwrap();
            ids.push(it.id);
        }
        (dir, raiz, ids)
    }

    /// Un `item.json` roto se salta y se cuenta; el resto entra igual.
    #[test]
    fn un_item_json_roto_no_deja_el_indice_a_medias() {
        let (_d, raiz, ids) = biblioteca_en_disco(20);
        std::fs::write(Index::item_json_path(&raiz, &ids[7]), b"{ esto no es json").unwrap();
        // Y otro de una versión que esta build no entiende.
        let ruta = Index::item_json_path(&raiz, &ids[11]);
        let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&ruta).unwrap()).unwrap();
        v["schema"] = serde_json::json!(crate::SCHEMA_VERSION + 1);
        std::fs::write(&ruta, serde_json::to_vec(&v).unwrap()).unwrap();

        let mut idx = Index::open(&raiz.join("index.sqlite")).unwrap();
        let r = idx.reconstruir(&raiz).unwrap();
        assert_eq!(r.vistos, 18);
        assert_eq!(r.rotos.len(), 2);
        assert!(r.rotos.iter().any(|(p, _)| p.starts_with(raiz.join("items")) && p == &Index::item_json_path(&raiz, &ids[7])));
        assert_eq!(idx.count().unwrap(), 18);
        // Los índices secundarios vuelven a estar.
        let n: i64 = idx
            .conn
            .query_row("SELECT COUNT(*) FROM sqlite_master WHERE name = 'idx_it_tag'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
        // Y la conexión vuelve a escribir con su sincronía de siempre.
        let sync: i64 = idx.conn.query_row("PRAGMA synchronous", [], |r| r.get(0)).unwrap();
        assert_eq!(sync, 1, "NORMAL");
    }

    /// Reconstruir no deja nada de antes: ni posiciones de elementos que ya
    /// no existen ni etiquetas sin nadie.
    #[test]
    fn reconstruir_no_arrastra_restos() {
        let (_d, raiz, ids) = biblioteca_en_disco(4);
        let mut idx = Index::open(&raiz.join("index.sqlite")).unwrap();
        let mut fantasma = item("fantasma");
        fantasma.tags = vec!["solo-del-fantasma".into()];
        fantasma.orden.insert(String::new(), 1.0);
        idx.upsert(&fantasma, None).unwrap();

        let r = idx.reconstruir(&raiz).unwrap();
        assert_eq!(r.vistos, 4);
        let huerfanas: i64 = idx
            .conn
            .query_row("SELECT COUNT(*) FROM tags WHERE name = 'solo-del-fantasma'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(huerfanas, 0);
        let posiciones = idx.posiciones("").unwrap();
        assert_eq!(posiciones.len(), 4);
        assert!(ids.iter().all(|id| posiciones.contains_key(id)));
    }

    /// Si la reconstrucción falla a mitad, el índice de antes sigue entero.
    #[test]
    fn si_reconstruir_falla_el_indice_de_antes_sigue_entero() {
        let (_d, raiz, _) = biblioteca_en_disco(5);
        let mut idx = Index::open(&raiz.join("index.sqlite")).unwrap();
        idx.reconstruir(&raiz).unwrap();
        assert_eq!(idx.count().unwrap(), 5);
        // Un fallo dentro de la transacción: la tabla de etiquetas rechaza
        // cualquier inserción.
        idx.conn
            .execute_batch(
                "CREATE TRIGGER romper BEFORE INSERT ON tags
                 BEGIN SELECT RAISE(ABORT, 'roto a propósito'); END;",
            )
            .unwrap();
        assert!(idx.reconstruir(&raiz).is_err());
        assert_eq!(idx.count().unwrap(), 5, "el índice de antes, sin tocar");
        let n: i64 = idx
            .conn
            .query_row("SELECT COUNT(*) FROM sqlite_master WHERE name = 'idx_items_hash'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1, "los índices tirados vuelven con el rollback");
    }

    #[test]
    fn borrar_limpia_el_orden_y_las_etiquetas_huerfanas() {
        let mut idx = Index::open_in_memory().unwrap();
        let mut a = item("a");
        a.tags = vec!["compartida".into(), "solo-a".into()];
        a.orden.insert(String::new(), 1.0);
        let mut b = item("b");
        b.tags = vec!["compartida".into()];
        idx.upsert_many([(&a, None), (&b, None)]).unwrap();
        idx.borrar(std::slice::from_ref(&a.id)).unwrap();

        let etiquetas: Vec<String> = idx
            .conn
            .prepare("SELECT name FROM tags ORDER BY name")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(etiquetas, vec!["compartida".to_string()]);
        assert!(idx.posiciones("").unwrap().is_empty());
    }

    /// `repetidos:si` reusa el cálculo mientras nada cambie, y lo rehace en
    /// cuanto cambia algo: desde esta conexión o desde otra.
    #[test]
    fn la_cache_de_repetidos_se_entera_de_los_cambios() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("index.sqlite");
        let mut idx = Index::open(&ruta).unwrap();
        let con_hash = |nombre: &str, h: &str| {
            let mut it = item(nombre);
            it.hash = h.into();
            it
        };
        idx.upsert_many([(&con_hash("a", "h1"), None), (&con_hash("b", "h1"), None), (&con_hash("c", "h2"), None)])
            .unwrap();
        let q = Query {
            repetidos: true,
            limit: 0,
            ..Default::default()
        };
        assert_eq!(idx.search(&q).unwrap().len(), 2);
        let primera = idx.grupos_repetidos().unwrap();
        assert!(std::sync::Arc::ptr_eq(&primera, &idx.grupos_repetidos().unwrap()), "sin cambios, la misma");

        // Desde esta conexión.
        let d = con_hash("d", "h2");
        idx.upsert(&d, None).unwrap();
        assert_eq!(idx.search(&q).unwrap().len(), 4);
        assert_eq!(idx.contar(&q).unwrap(), 4);
        idx.borrar(std::slice::from_ref(&d.id)).unwrap();
        assert_eq!(idx.search(&q).unwrap().len(), 2);

        // Desde otra: la terminal importando con la ventana abierta.
        let mut otra = Index::open(&ruta).unwrap();
        otra.upsert(&con_hash("e", "h2"), None).unwrap();
        assert_eq!(idx.search(&q).unwrap().len(), 4);
    }
}
