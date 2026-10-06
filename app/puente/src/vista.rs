//! Una vista: el resultado de una consulta, congelado, más el pack de
//! miniaturas que hace falta para pintarlo.
//!
//! Es inmutable a propósito. El hilo trabajador no modifica la que está
//! publicada: construye otra entera y la sustituye. Así la interfaz puede leerla
//! sin cerrojos mientras dure el fotograma, y una consulta lenta no deja la
//! malla a medio pintar.

use grimorio_core::thumbs::{PackReader, ThumbRef};
use grimorio_core::{Library, Query, QueryHit, Result};
use std::collections::HashMap;
use std::sync::Arc;

pub struct Vista {
    hits: Vec<QueryHit>,
    /// De id a posición. La interfaz pregunta por id cuando cambia un elemento
    /// suelto, y buscarlo a mano sobre 100.000 sería un recorrido por cada
    /// estrella que alguien pulse.
    por_id: HashMap<String, usize>,
    pack: Option<Arc<PackReader>>,
    consulta: Query,
    raiz: std::path::PathBuf,
}

impl Vista {
    /// Construye una vista nueva reaprovechando el pack ya mapeado si lo hay.
    ///
    /// Volver a mapear el archivo en cada consulta funcionaría, pero invalida
    /// el caché de páginas del sistema y la primera pantalla después de buscar
    /// se pintaría desde disco.
    ///
    /// Salvo que el pack haya crecido, que es lo que pasa al importar con el
    /// programa abierto: entonces hay que mapearlo otra vez o las miniaturas
    /// recién llegadas no existen para nadie hasta reiniciar. El mapa viejo no
    /// se tira: la vista anterior lo sigue teniendo tomado mientras quede una
    /// celda decodificándose.
    pub fn nueva(lib: &Library, q: &Query, pack: Option<Arc<PackReader>>) -> Result<Vista> {
        let hits = lib.index().search(q)?;
        let mut por_id = HashMap::with_capacity(hits.len());
        for (i, h) in hits.iter().enumerate() {
            por_id.insert(h.id.clone(), i);
        }
        let pack = match pack {
            Some(p) if !p.crecio() => Some(p),
            _ => lib.pack_reader().ok().map(Arc::new),
        };
        Ok(Vista {
            hits,
            por_id,
            pack,
            consulta: q.clone(),
            raiz: lib.root().to_path_buf(),
        })
    }

    pub fn desde(lib: &mut Library, q: &Query) -> Result<Vista> {
        Vista::nueva(lib, q, None)
    }

    pub fn len(&self) -> usize {
        self.hits.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hits.is_empty()
    }

    pub fn hits(&self) -> &[QueryHit] {
        &self.hits
    }

    pub fn hit(&self, i: usize) -> Option<&QueryHit> {
        self.hits.get(i)
    }

    pub fn indice_de(&self, id: &str) -> Option<usize> {
        self.por_id.get(id).copied()
    }

    pub fn consulta(&self) -> &Query {
        &self.consulta
    }

    pub fn pack(&self) -> Option<Arc<PackReader>> {
        self.pack.clone()
    }

    pub fn thumb(&self, i: usize) -> Option<&[u8]> {
        let h = self.hits.get(i)?;
        let (offset, len) = (h.thumb_off?, h.thumb_len?);
        self.pack.as_ref()?.get(ThumbRef { offset, len })
    }

    /// Ruta de la previsualización de 1024 px, tal y como la escribe la
    /// importación. Vacía si el índice no existe.
    pub fn ruta_previa(&self, i: usize) -> Option<String> {
        let h = self.hits.get(i)?;
        Some(
            grimorio_core::preview_path(&self.raiz, &h.id)
                .to_string_lossy()
                .into_owned(),
        )
    }

    /// La ruta del archivo de verdad, el que se abre y el que se reproduce.
    ///
    /// Se deduce del id y la extensión salvo en modo referencia, que es el
    /// único caso en que el archivo no vive dentro de la biblioteca. La misma
    /// cuenta que hace `Library::original_path`, aquí para no tener que cargar
    /// el `item.json` de cada vídeo que se ve.
    pub fn ruta_original(&self, i: usize) -> Option<String> {
        let h = self.hits.get(i)?;
        if let Some(r) = &h.ruta_ref {
            return Some(r.clone());
        }
        let mut p = self.raiz.join("items");
        p.push(&h.id[0..2]);
        p.push(&h.id[2..4]);
        p.push(&h.id);
        p.push(format!("original.{}", h.ext));
        Some(p.to_string_lossy().into_owned())
    }

    pub fn bytes_pack(&self) -> u64 {
        self.pack.as_ref().map(|p| p.size() as u64).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grimorio_core::item::{Item, OriginMode};

    fn biblioteca() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let raiz = dir.path().join("p.grimorio");
        let lib = Library::create(&raiz, "p").unwrap();
        (dir, lib)
    }

    #[test]
    fn una_vista_sabe_donde_esta_cada_id() {
        let (_d, mut lib) = biblioteca();
        let mut ids = Vec::new();
        for n in 0..5 {
            let item = Item::new(
                grimorio_core::id::new_id(),
                format!("foto {n}"),
                "jpg".into(),
                10,
                OriginMode::Copy,
            );
            ids.push(item.id.clone());
            std::fs::create_dir_all(lib.item_dir(&item.id)).unwrap();
            lib.save_item(&item).unwrap();
        }
        let v = Vista::desde(&mut lib, &Query::default()).unwrap();
        assert_eq!(v.len(), 5);
        for id in &ids {
            let i = v.indice_de(id).expect("el id tiene que estar");
            assert_eq!(&v.hit(i).unwrap().id, id);
        }
        assert!(v.indice_de("no-existe").is_none());
    }

    #[test]
    fn sin_pack_la_vista_funciona_igual() {
        let (_d, mut lib) = biblioteca();
        let v = Vista::desde(&mut lib, &Query::default()).unwrap();
        assert!(v.is_empty());
        assert!(v.thumb(0).is_none(), "pedir una miniatura que no hay");
        assert_eq!(v.bytes_pack(), 0);
    }

    /// Importar con el programa abierto: el pack crece por detrás de un mapa
    /// que ya estaba hecho. Antes esto devolvía `None` y la celda se quedaba
    /// con su color de fondo hasta reiniciar.
    #[test]
    fn una_miniatura_recien_importada_se_ve_sin_reiniciar() {
        let (_d, mut lib) = biblioteca();

        fn meter(lib: &mut Library, nombre: &str, miniatura: &[u8]) -> String {
            let item = Item::new(
                grimorio_core::id::new_id(),
                nombre.into(),
                "jpg".into(),
                10,
                OriginMode::Copy,
            );
            let tref = {
                let mut w = lib.pack_writer().unwrap();
                let r = w.append(&item.id, miniatura).unwrap();
                w.flush().unwrap();
                r
            };
            std::fs::create_dir_all(lib.item_dir(&item.id)).unwrap();
            lib.save_item(&item).unwrap();
            lib.index_mut()
                .upsert_many(std::iter::once((&item, Some(tref))))
                .unwrap();
            item.id.clone()
        }

        meter(&mut lib, "la que ya estaba", b"vieja");
        let publicada = Vista::desde(&mut lib, &Query::default()).unwrap();
        assert!(
            publicada.pack().is_some(),
            "el pack tiene que estar mapeado"
        );

        let nuevo = meter(&mut lib, "la que acaba de entrar", b"recien-llegada");

        // Como en `republicar`: se rehace la vista pasándole el pack de la que
        // estaba publicada.
        let despues = Vista::nueva(&lib, &Query::default(), publicada.pack()).unwrap();
        let i = despues.indice_de(&nuevo).expect("el id tiene que estar");
        assert_eq!(
            despues.thumb(i),
            Some(&b"recien-llegada"[..]),
            "la miniatura nueva quedó fuera del mapa viejo"
        );
    }
}
