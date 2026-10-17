//! Pilas de versiones: la misma imagen con distinto peso, enseñada como una.
//!
//! Una pila es una principal y sus versiones. La principal es la que se ve en
//! la malla, la que lleva las etiquetas y las carpetas; las versiones apuntan
//! a ella con `Item::pila` y solo salen al abrir la pila. Las pilas son
//! planas: apilar algo que ya tenía versiones se las lleva consigo a la nueva
//! principal.
//!
//! Nada de aquí escribe un `item.json` por su cuenta. Todo pasa por
//! [`Library::editar`] con un parche por elemento, así que apilar, desapilar,
//! cambiar la principal y tirar una pila se deshacen como cualquier otra cosa,
//! de una vez.
//!
//! Y nada de aquí toca un original. Crear una versión decodifica el original,
//! escribe un archivo **nuevo** y lo importa como un elemento más de la pila.

use crate::error::{Error, Result};
use crate::import::{self, ImportOptions};
use crate::item::{Item, OriginMode, Receta, Tramo};
use crate::library::Library;
use crate::registro::{Operacion, Parche};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Formatos que sabe escribir una receta.
pub const FORMATOS: &[&str] = &["jpg", "png", "webp"];

impl Library {
    /// La principal de la pila de `id`: él mismo si no es versión de nadie, o
    /// si la principal a la que apunta ya no está.
    pub fn principal_de(&self, id: &str) -> Result<String> {
        let item = self.load_item(id)?;
        Ok(match item.pila {
            Some(p) if self.load_item(&p).is_ok_and(|pr| !pr.trashed) => p,
            _ => item.id,
        })
    }

    /// Mete `ids` en la pila de `principal`, con sus propias versiones si
    /// las tenían. Si `principal` es a su vez versión, la pila es la de su
    /// principal: nunca se forman pilas dentro de pilas.
    ///
    /// Devuelve cuántos cambiaron de pila.
    pub fn apilar(&mut self, principal: &str, ids: &[String]) -> Result<usize> {
        self.apilar_grupos(&[(principal.to_string(), ids.to_vec())])
    }

    /// Varias pilas de una vez, como **una** cosa que se deshace: es lo que
    /// hace «apilar todas» al importar diez fotos que ya estaban en otro peso.
    pub fn apilar_grupos(&mut self, grupos: &[(String, Vec<String>)]) -> Result<usize> {
        let mut parches: HashMap<String, Parche> = HashMap::new();
        for (principal, ids) in grupos {
            let principal = self.principal_de(principal)?;
            if self.load_item(&principal)?.trashed {
                return Err(Error::Invalid("la principal está en la papelera".into()));
            }
            for id in ids {
                if *id == principal {
                    continue;
                }
                parches.insert(id.clone(), Parche::pila(&principal));
                for v in self.index().versiones_de(id)? {
                    if v != principal {
                        parches.insert(v, Parche::pila(&principal));
                    }
                }
            }
            // La que hace de principal no puede quedar a la vez como versión
            // de otra por venir en otro grupo.
            parches.remove(&principal);
        }
        if parches.is_empty() {
            return Ok(0);
        }
        let titulo = if parches.len() == 1 {
            "apilar como versión".to_string()
        } else {
            format!("apilar {} versiones", parches.len())
        };
        self.editar_con(&parches, &titulo)
    }

    /// Saca de su pila a cada uno de `ids`: vuelven a la malla como elementos
    /// sueltos. Desapilar una principal suelta a todas sus versiones.
    pub fn desapilar(&mut self, ids: &[String]) -> Result<usize> {
        let mut salen: Vec<String> = Vec::new();
        for id in ids {
            let item = self.load_item(id)?;
            if item.pila.is_some() {
                if !salen.contains(id) {
                    salen.push(id.clone());
                }
            } else {
                for v in self.index().versiones_de(id)? {
                    if !salen.contains(&v) {
                        salen.push(v);
                    }
                }
            }
        }
        if salen.is_empty() {
            return Ok(0);
        }
        let (tocados, _) =
            self.editar(&salen, "sacar de la pila", |_| Parche::pila(""), &|_, _| {})?;
        Ok(tocados)
    }

    /// Hace de `id` la principal de su pila.
    ///
    /// La nueva principal hereda lo que es de la pila y no del archivo —las
    /// etiquetas, las carpetas, las estrellas, la nota, su sitio en el orden a
    /// mano—, porque es lo que la gente puso mirando la pila desde la malla.
    /// La antigua principal se queda con lo suyo, por si un día se desapila.
    pub fn hacer_principal(&mut self, id: &str) -> Result<()> {
        let nueva = self.load_item(id)?;
        let principal = self.principal_de(id)?;
        if principal == nueva.id {
            return Ok(());
        }
        let vieja = self.load_item(&principal)?;
        let mut parches: HashMap<String, Parche> = HashMap::new();
        parches.insert(nueva.id.clone(), Self::heredar(&vieja));
        parches.insert(vieja.id.clone(), Parche::pila(&nueva.id));
        for v in self.index().versiones_de(&vieja.id)? {
            if v != nueva.id {
                parches.insert(v, Parche::pila(&nueva.id));
            }
        }
        self.editar_con(&parches, "cambiar la principal de la pila")?;
        Ok(())
    }

    /// Manda a la papelera —o saca de ella— teniendo en cuenta las pilas.
    ///
    /// Al tirar una principal que tiene versiones hay dos cosas razonables y
    /// la decide quien pide, con `con_versiones`:
    ///
    /// - `true`: la pila entera va a la papelera.
    /// - `false`: solo la principal. La versión más pesada —la de más calidad
    ///   que queda— pasa a ser la principal y hereda lo de la pila. Así
    ///   tirar la principal nunca deja versiones huérfanas fuera de la vista.
    ///
    /// Todo va en **una** operación: deshacer devuelve la principal a su sitio
    /// y a las versiones bajo ella. Sacar de la papelera no arrastra a nadie:
    /// lo que se tiró por separado se recupera por separado.
    pub fn tirar_pilas(
        &mut self,
        ids: &[String],
        dentro: bool,
        con_versiones: bool,
        progreso: &(dyn Fn(usize, usize) + Sync),
    ) -> Result<(usize, usize)> {
        if !dentro {
            return self.editar(
                ids,
                "sacar de la papelera",
                |_| Parche::papelera(false),
                progreso,
            );
        }
        let mut orden: Vec<String> = ids.to_vec();
        let mut parches: HashMap<String, Parche> = ids
            .iter()
            .map(|id| (id.clone(), Parche::papelera(true)))
            .collect();

        // Solo las principales arrastran algo. La pregunta al índice es una
        // sola para toda la selección: tirar diez mil sueltos no puede costar
        // diez mil consultas para descubrir que ninguno tiene versiones.
        let pilas = self.index().pilas()?;
        for id in ids {
            if !pilas.contains_key(id) {
                continue;
            }
            let quedan: Vec<Item> = self
                .index()
                .versiones_de(id)?
                .into_iter()
                .filter(|v| !parches.contains_key(v))
                .filter_map(|v| self.load_item(&v).ok())
                .filter(|v| !v.trashed)
                .collect();
            if quedan.is_empty() {
                continue;
            }
            if con_versiones {
                for v in quedan {
                    parches.insert(v.id.clone(), Parche::papelera(true));
                    orden.push(v.id);
                }
                continue;
            }
            let Ok(vieja) = self.load_item(id) else {
                continue;
            };
            // La más pesada es la que más se parece al original: es la que
            // uno querría ver en la malla si el original se va.
            let nueva = quedan
                .iter()
                .max_by_key(|v| (v.size, std::cmp::Reverse(v.id.clone())))
                .map(|v| v.id.clone())
                .unwrap_or_default();
            for v in &quedan {
                let p = if v.id == nueva {
                    Self::heredar(&vieja)
                } else {
                    Parche::pila(&nueva)
                };
                parches.insert(v.id.clone(), p);
                orden.push(v.id.clone());
            }
        }

        let titulo = "mandar a la papelera";
        self.editar(
            &orden,
            titulo,
            |item| parches.get(&item.id).cloned().unwrap_or_default(),
            progreso,
        )
    }

    /// Saca de un original una versión nueva con `receta` y la mete en su
    /// pila. Devuelve el id de la versión.
    ///
    /// El original no se toca: se lee, se decodifica y se escribe un archivo
    /// nuevo que entra por la importación de siempre, con su miniatura, su
    /// huella y su paleta. Se deshace mandándola a la papelera, igual que
    /// cualquier importación.
    pub fn crear_version(&mut self, id: &str, receta: &Receta) -> Result<String> {
        let (fuente, formato, salida) = self.preparar_version(id, receta)?;
        // El archivo se escribe dentro de la biblioteca y entra **moviéndolo**:
        // es un temporal que nadie más quiere, como lo pegado del portapapeles.
        let tmp = self.temporal_de_version(&formato)?;
        std::fs::write(&tmp, &salida).map_err(|e| Error::io(&tmp, e))?;
        let receta = Receta {
            formato,
            ..receta.clone()
        };
        let r = self.anadir_version(&fuente.id, &tmp, &receta);
        let _ = std::fs::remove_file(&tmp);
        r
    }

    /// Un trozo de un vídeo o de un sonido como versión nueva en su pila.
    ///
    /// Rápido por defecto: ffmpeg copia los paquetes del tramo sin
    /// decodificarlos (`-c copy`), así que tarda lo que tarda leerlos del
    /// disco aunque el vídeo sea de una hora. A cambio, el vídeo empieza en el
    /// fotograma clave de antes del punto pedido, unas décimas antes. Con
    /// `exacto` se recodifica y corta justo donde se pidió.
    pub fn recortar_tramo(&mut self, id: &str, tramo: Tramo) -> Result<String> {
        let fuente = self.load_item(id)?;
        if !matches!(fuente.kind, crate::medio::Familia::Video | crate::medio::Familia::Audio) {
            return Err(Error::Invalid("solo se recortan vídeos y sonidos".into()));
        }
        if tramo.hasta_ms <= tramo.desde_ms {
            return Err(Error::Invalid("el tramo acaba antes de empezar".into()));
        }
        let original = self.original_path(&fuente);
        let ext = fuente.ext.to_ascii_lowercase();
        let tmp = crate::derivado::cortar_tramo(self.root(), &original, tramo)?;
        let receta = Receta {
            formato: ext,
            calidad: 100,
            lado: 0,
            recorte: None,
            tramo: Some(tramo),
        };
        let r = self.anadir_version(id, &tmp, &receta);
        let _ = std::fs::remove_file(&tmp);
        r
    }

    /// Lo que pesaría la versión de `id` con `receta`, sin crearla: el mismo
    /// trabajo que [`Library::crear_version`] hasta tener los bytes, que se
    /// cuentan y se tiran. Es lo que deja ver el peso antes de decidir.
    pub fn estimar_version(&self, id: &str, receta: &Receta) -> Result<Estimacion> {
        let (fuente, _, salida) = self.preparar_version(id, receta)?;
        let (ancho, alto) = image::ImageReader::new(std::io::Cursor::new(&salida))
            .with_guessed_format()
            .map_err(|e| Error::Invalid(e.to_string()))?
            .into_dimensions()?;
        Ok(Estimacion {
            peso: salida.len() as u64,
            ancho,
            alto,
            original: fuente.size,
        })
    }

    /// Mete `archivo` en la pila de `id` como una versión hecha con `receta`.
    ///
    /// Es la puerta para lo que se hace fuera del núcleo —quitar el fondo lo
    /// hace `grim-recorte`, en su propio proceso— y deja el archivo nuevo en
    /// la pila igual que una versión hecha aquí. Lo que viene de la carpeta de
    /// temporales de la biblioteca entra moviéndolo; cualquier otra cosa, copiándola:
    /// el archivo de otro no se le quita de su sitio.
    pub fn anadir_version(&mut self, id: &str, archivo: &Path, receta: &Receta) -> Result<String> {
        let fuente = self.load_item(id)?;
        let principal = self.principal_de(id)?;
        let temporales = self.root().join("cache").join("versiones");
        let mover = archivo.parent().is_some_and(|p| p == temporales);
        let opts = ImportOptions {
            mode: if mover { OriginMode::Move } else { OriginMode::Copy },
            recursive: false,
            // Una versión idéntica byte a byte a otra ya existente sigue siendo
            // lo que se pidió; saltarla dejaría el botón sin hacer nada.
            skip_duplicates: false,
            ..Default::default()
        };
        let informe = import::import(self, &[archivo.to_path_buf()], &opts, &|_, _| {})?;
        let Some(nuevo) = informe.ids.first().cloned() else {
            let motivo = informe
                .failed
                .first()
                .map(|(_, e)| e.clone())
                .unwrap_or_else(|| "la importación no la aceptó".into());
            return Err(Error::Invalid(format!(
                "no se pudo crear la versión: {motivo}"
            )));
        };

        let mut item = self.load_item(&nuevo)?;
        item.name = format!("{} ({})", fuente.name, describir(receta, &item));
        item.pila = Some(principal);
        item.receta = Some(receta.clone());
        item.origin.path = None;
        self.save_item(&item)?;

        let titulo = if receta.recorte.is_some() {
            "quitar el fondo"
        } else if receta.tramo.is_some() {
            "recortar"
        } else {
            "crear una versión"
        };
        let mut op = Operacion::nueva(titulo);
        op.antes.push((nuevo.clone(), Parche::papelera(true)));
        op.despues.push((nuevo.clone(), Parche::papelera(false)));
        self.anotar(op);
        Ok(nuevo)
    }

    /// Un nombre libre en la carpeta de temporales de las versiones, que es la
    /// que [`Library::anadir_version`] sabe que puede vaciar.
    pub fn temporal_de_version(&self, ext: &str) -> Result<PathBuf> {
        let dir = self.root().join("cache").join("versiones");
        std::fs::create_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
        Ok(dir.join(format!("{}.{ext}", crate::id::new_id())))
    }

    /// Decodifica el original de `id` y lo codifica con `receta`.
    fn preparar_version(&self, id: &str, receta: &Receta) -> Result<(Item, String, Vec<u8>)> {
        let formato = receta.formato.to_ascii_lowercase();
        if !FORMATOS.contains(&formato.as_str()) {
            return Err(Error::Invalid(format!(
                "no sé escribir «{}»; las versiones salen en jpg, png o webp",
                receta.formato
            )));
        }
        let fuente = self.load_item(id)?;
        if !crate::image_ops::is_supported(&fuente.ext) {
            return Err(Error::Invalid(format!(
                "por ahora solo se sacan versiones de imágenes jpg, png, webp, gif o bmp, y esto es {}",
                fuente.ext
            )));
        }
        let ruta = self.original_path(&fuente);
        let bytes = std::fs::read(&ruta).map_err(|e| Error::io(&ruta, e))?;
        let img = crate::image_ops::decode(&bytes)?;
        let salida = codificar(&img, &formato, receta)?;
        Ok((fuente, formato, salida))
    }

    /// Lo que una principal nueva toma de la vieja, más salir de la pila.
    fn heredar(vieja: &Item) -> Parche {
        Parche {
            pila: Some(String::new()),
            etiquetas: Some(vieja.tags.clone()),
            carpetas: Some(vieja.folders.clone()),
            estrellas: Some(vieja.stars),
            nota: Some(vieja.note.clone().unwrap_or_default()),
            orden: Some(vieja.orden.clone()),
            adulto: Some(vieja.adult),
            ..Default::default()
        }
    }

    /// [`Library::editar`] con un parche ya decidido para cada elemento.
    fn editar_con(&mut self, parches: &HashMap<String, Parche>, titulo: &str) -> Result<usize> {
        let ids: Vec<String> = parches.keys().cloned().collect();
        let (tocados, _) = self.editar(
            &ids,
            titulo,
            |item| parches.get(&item.id).cloned().unwrap_or_default(),
            &|_, _| {},
        )?;
        Ok(tocados)
    }
}

/// «web 1600 px», «png», «jpg 80 %»: lo que distingue a una versión de otra en
/// su nombre, para que se reconozcan sin abrirlas.
fn describir(receta: &Receta, hecho: &Item) -> String {
    if receta.recorte.is_some() {
        return "sin fondo".into();
    }
    if let Some(t) = receta.tramo {
        return format!("{}–{}", reloj(t.desde_ms), reloj(t.hasta_ms));
    }
    let mut s = receta.formato.to_ascii_lowercase();
    if receta.lado > 0 {
        s.push_str(&format!(" {} px", hecho.width.max(hecho.height)));
    }
    if con_perdida(receta) {
        s.push_str(&format!(" {} %", receta.calidad.clamp(1, 100)));
    }
    s
}

/// `75 300` ms → `1:15,3`: minutos y segundos con décimas, como se lee un
/// tramo.
fn reloj(ms: u64) -> String {
    let s = ms / 1000;
    let d = (ms % 1000) / 100;
    if s >= 3600 {
        format!("{}:{:02}:{:02},{d}", s / 3600, s / 60 % 60, s % 60)
    } else {
        format!("{}:{:02},{d}", s / 60, s % 60)
    }
}

/// Si la calidad cuenta: en JPEG siempre, en WebP por debajo de 100.
pub fn con_perdida(receta: &Receta) -> bool {
    match receta.formato.to_ascii_lowercase().as_str() {
        "jpg" => true,
        "webp" => receta.calidad < 100,
        _ => false,
    }
}

/// Lo que pesaría una versión antes de crearla.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Estimacion {
    pub peso: u64,
    pub ancho: u32,
    pub alto: u32,
    /// Lo que pesa el original, para compararlo sin otra consulta.
    pub original: u64,
}

/// Reduce y codifica. No amplía nunca: una receta de 1600 px sobre una imagen
/// de 800 deja los 800.
fn codificar(img: &image::DynamicImage, formato: &str, receta: &Receta) -> Result<Vec<u8>> {
    use image::{GenericImageView, ImageEncoder};
    let (w, h) = img.dimensions();
    let reducida;
    let img = if receta.lado > 0 && w.max(h) > receta.lado {
        // Lanczos y no el filtro rápido de las miniaturas: esto es algo que se
        // va a usar, no un vistazo, y se hace una vez.
        reducida = img.resize(
            receta.lado,
            receta.lado,
            image::imageops::FilterType::Lanczos3,
        );
        &reducida
    } else {
        img
    };
    let alfa = img.color().has_alpha();
    let mut out = Vec::new();
    match formato {
        "jpg" => {
            // JPEG no tiene transparencia. Tirar el canal sin más deja lo
            // transparente del color que hubiera debajo, casi siempre negro;
            // sobre blanco es lo que espera quien lo va a pegar en una página.
            let rgb = if alfa {
                sobre_blanco(&img.to_rgba8())
            } else {
                img.to_rgb8()
            };
            let q = receta.calidad.clamp(1, 100);
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, q).write_image(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                image::ExtendedColorType::Rgb8,
            )?;
        }
        "png" => {
            let enc = image::codecs::png::PngEncoder::new_with_quality(
                &mut out,
                image::codecs::png::CompressionType::Best,
                image::codecs::png::FilterType::Adaptive,
            );
            escribir(enc, img, alfa)?;
        }
        "webp" => {
            // Con pérdida salvo a calidad 100: lo que se busca casi siempre es
            // peso, y sin pérdida WebP pesa como un PNG.
            let q = receta.calidad.clamp(1, 100);
            if q >= 100 {
                escribir(
                    image::codecs::webp::WebPEncoder::new_lossless(&mut out),
                    img,
                    alfa,
                )?;
            } else if alfa {
                let px = img.to_rgba8();
                out = webp::Encoder::from_rgba(px.as_raw(), px.width(), px.height())
                    .encode(q as f32)
                    .to_vec();
            } else {
                let px = img.to_rgb8();
                out = webp::Encoder::from_rgb(px.as_raw(), px.width(), px.height())
                    .encode(q as f32)
                    .to_vec();
            }
        }
        otro => return Err(Error::Invalid(format!("formato desconocido: {otro}"))),
    }
    Ok(out)
}

fn escribir(enc: impl image::ImageEncoder, img: &image::DynamicImage, alfa: bool) -> Result<()> {
    if alfa {
        let px = img.to_rgba8();
        enc.write_image(
            px.as_raw(),
            px.width(),
            px.height(),
            image::ExtendedColorType::Rgba8,
        )?;
    } else {
        let px = img.to_rgb8();
        enc.write_image(
            px.as_raw(),
            px.width(),
            px.height(),
            image::ExtendedColorType::Rgb8,
        )?;
    }
    Ok(())
}

fn sobre_blanco(rgba: &image::RgbaImage) -> image::RgbImage {
    image::RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let p = rgba.get_pixel(x, y).0;
        let a = p[3] as u32;
        let mezcla = |c: u8| ((c as u32 * a + 255 * (255 - a)) / 255) as u8;
        image::Rgb([mezcla(p[0]), mezcla(p[1]), mezcla(p[2])])
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::{Query, SortBy};

    fn nada(_: usize, _: usize) {}

    fn biblioteca() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let lib = Library::create(&dir.path().join("p.grimorio"), "p").unwrap();
        (dir, lib)
    }

    fn mete(lib: &mut Library, nombre: &str, peso: u64) -> String {
        let item = Item::new(
            crate::id::new_id(),
            nombre.into(),
            "jpg".into(),
            peso,
            OriginMode::Copy,
        );
        std::fs::create_dir_all(lib.item_dir(&item.id)).unwrap();
        lib.save_item(&item).unwrap();
        item.id
    }

    fn malla(lib: &Library) -> Vec<String> {
        let q = Query {
            limit: 0,
            sort: SortBy::NameAsc,
            ..Default::default()
        };
        lib.index()
            .search(&q)
            .unwrap()
            .into_iter()
            .map(|h| h.name)
            .collect()
    }

    #[test]
    fn una_pila_se_ve_como_un_solo_elemento_con_sus_versiones_contadas() {
        let (_d, mut lib) = biblioteca();
        let a = mete(&mut lib, "a", 4000);
        let b = mete(&mut lib, "b", 400);
        let c = mete(&mut lib, "c", 40);
        mete(&mut lib, "suelto", 10);
        assert_eq!(lib.apilar(&a, &[b.clone(), c.clone()]).unwrap(), 2);

        assert_eq!(malla(&lib), vec!["a", "suelto"]);
        assert_eq!(
            lib.index().count().unwrap(),
            2,
            "la barra cuenta lo que se ve"
        );
        let q = Query {
            limit: 0,
            ..Default::default()
        };
        let hits = lib.index().search(&q).unwrap();
        let ha = hits.iter().find(|h| h.id == a).unwrap();
        assert_eq!(ha.versiones, 2);

        // Y la pila entera, para el visor.
        let q = Query {
            pila: Some(a.clone()),
            limit: 0,
            sort: SortBy::SizeDesc,
            ..Default::default()
        };
        let pila: Vec<String> = lib
            .index()
            .search(&q)
            .unwrap()
            .into_iter()
            .map(|h| h.id)
            .collect();
        assert_eq!(pila, vec![a, b, c]);
    }

    #[test]
    fn apilar_en_una_version_va_a_su_principal_y_no_anida() {
        let (_d, mut lib) = biblioteca();
        let a = mete(&mut lib, "a", 3);
        let b = mete(&mut lib, "b", 2);
        let c = mete(&mut lib, "c", 1);
        let d = mete(&mut lib, "d", 1);
        lib.apilar(&c, &[d.clone()]).unwrap();
        lib.apilar(&a, &[b.clone()]).unwrap();
        // Apilar c (con su d) sobre b, que es versión de a: todo va a a.
        lib.apilar(&b, &[c.clone()]).unwrap();
        for v in [&b, &c, &d] {
            assert_eq!(lib.load_item(v).unwrap().pila.as_deref(), Some(a.as_str()));
        }
        assert_eq!(malla(&lib), vec!["a"]);
    }

    #[test]
    fn apilar_y_desapilar_se_deshacen() {
        let (_d, mut lib) = biblioteca();
        let a = mete(&mut lib, "a", 3);
        let b = mete(&mut lib, "b", 2);
        lib.apilar(&a, &[b.clone()]).unwrap();
        lib.deshacer(&nada).unwrap();
        assert_eq!(malla(&lib), vec!["a", "b"]);
        lib.rehacer(&nada).unwrap();
        assert_eq!(malla(&lib), vec!["a"]);
        // Desapilar la principal suelta a todas.
        lib.desapilar(&[a.clone()]).unwrap();
        assert_eq!(malla(&lib), vec!["a", "b"]);
        lib.deshacer(&nada).unwrap();
        assert_eq!(malla(&lib), vec!["a"]);
    }

    #[test]
    fn tirar_la_principal_sola_asciende_a_la_mas_pesada_con_lo_de_la_pila() {
        let (_d, mut lib) = biblioteca();
        let a = mete(&mut lib, "a", 4000);
        let b = mete(&mut lib, "b", 400);
        let c = mete(&mut lib, "c", 40);
        lib.apilar(&a, &[b.clone(), c.clone()]).unwrap();
        lib.editar(
            &[a.clone()],
            "etiquetar",
            |_| Parche::etiquetas(vec!["luz".into()]),
            &nada,
        )
        .unwrap();

        lib.tirar_pilas(&[a.clone()], true, false, &nada).unwrap();
        assert_eq!(malla(&lib), vec!["b"], "b es la más pesada que queda");
        let nb = lib.load_item(&b).unwrap();
        assert!(nb.pila.is_none());
        assert_eq!(
            nb.tags,
            vec!["luz".to_string()],
            "hereda las etiquetas de la pila"
        );
        assert_eq!(lib.load_item(&c).unwrap().pila.as_deref(), Some(b.as_str()));
        assert_eq!(lib.load_item(&a).unwrap().pila, None);

        // Deshacer lo deja todo como estaba, de una vez.
        lib.deshacer(&nada).unwrap();
        assert_eq!(malla(&lib), vec!["a"]);
        assert_eq!(lib.load_item(&b).unwrap().pila.as_deref(), Some(a.as_str()));
        assert!(lib.load_item(&b).unwrap().tags.is_empty());
        assert_eq!(lib.load_item(&c).unwrap().pila.as_deref(), Some(a.as_str()));
    }

    #[test]
    fn tirar_la_pila_entera_se_lleva_las_versiones_y_vuelve_entera() {
        let (_d, mut lib) = biblioteca();
        let a = mete(&mut lib, "a", 4000);
        let b = mete(&mut lib, "b", 400);
        mete(&mut lib, "suelto", 1);
        lib.apilar(&a, &[b.clone()]).unwrap();
        let (tocados, _) = lib.tirar_pilas(&[a.clone()], true, true, &nada).unwrap();
        assert_eq!(tocados, 2);
        assert_eq!(malla(&lib), vec!["suelto"]);
        assert_eq!(lib.index().contar_papelera().unwrap(), 2);
        lib.deshacer(&nada).unwrap();
        assert_eq!(malla(&lib), vec!["a", "suelto"]);
        assert_eq!(lib.index().contar_papelera().unwrap(), 0);
    }

    #[test]
    fn una_version_cuya_principal_no_esta_se_ve_suelta() {
        let (_d, mut lib) = biblioteca();
        let a = mete(&mut lib, "a", 3);
        let b = mete(&mut lib, "b", 2);
        lib.apilar(&a, &[b.clone()]).unwrap();
        // Tirada por la vía de siempre, sin mirar pilas: b no puede perderse.
        lib.editar(&[a.clone()], "x", |_| Parche::papelera(true), &nada)
            .unwrap();
        assert_eq!(malla(&lib), vec!["b"]);
        assert_eq!(lib.principal_de(&b).unwrap(), b);
    }

    #[test]
    fn hacer_principal_intercambia_y_hereda() {
        let (_d, mut lib) = biblioteca();
        let a = mete(&mut lib, "a", 3);
        let b = mete(&mut lib, "b", 2);
        let c = mete(&mut lib, "c", 1);
        lib.apilar(&a, &[b.clone(), c.clone()]).unwrap();
        lib.editar(&[a.clone()], "estrellas", |_| Parche::estrellas(5), &nada)
            .unwrap();
        lib.hacer_principal(&c).unwrap();
        assert_eq!(malla(&lib), vec!["c"]);
        assert_eq!(lib.load_item(&c).unwrap().stars, 5);
        assert_eq!(lib.load_item(&a).unwrap().pila.as_deref(), Some(c.as_str()));
        assert_eq!(lib.load_item(&b).unwrap().pila.as_deref(), Some(c.as_str()));
        lib.deshacer(&nada).unwrap();
        assert_eq!(malla(&lib), vec!["a"]);
    }

    #[test]
    fn crear_una_version_no_toca_el_original_y_pesa_menos() {
        let (d, mut lib) = biblioteca();
        // Una imagen de verdad, con ruido para que comprimir se note.
        let mut img = image::RgbImage::new(1200, 800);
        for (x, y, p) in img.enumerate_pixels_mut() {
            *p = image::Rgb([(x % 251) as u8, (y % 241) as u8, ((x * y) % 239) as u8]);
        }
        let ruta = d.path().join("foto.png");
        img.save(&ruta).unwrap();
        let antes = std::fs::read(&ruta).unwrap();
        let informe = import::import(
            &mut lib,
            &[ruta.clone()],
            &ImportOptions::default(),
            &|_, _| {},
        )
        .unwrap();
        let original = informe.ids[0].clone();
        let bytes_original =
            std::fs::read(lib.original_path(&lib.load_item(&original).unwrap())).unwrap();

        let receta = Receta {
            formato: "jpg".into(),
            calidad: 70,
            lado: 600,
            recorte: None,
            tramo: None,
        };
        let v = lib.crear_version(&original, &receta).unwrap();
        let version = lib.load_item(&v).unwrap();
        assert_eq!(version.pila.as_deref(), Some(original.as_str()));
        assert_eq!(version.receta.as_ref().unwrap().lado, 600);
        assert_eq!(version.width.max(version.height), 600);
        assert_eq!(version.ext, "jpg");
        assert!(
            version.size < bytes_original.len() as u64,
            "la versión pesa menos"
        );
        assert!(version.name.contains("jpg"), "{}", version.name);

        assert_eq!(
            std::fs::read(&ruta).unwrap(),
            antes,
            "el archivo de fuera, intacto"
        );
        let ahora = std::fs::read(lib.original_path(&lib.load_item(&original).unwrap())).unwrap();
        assert_eq!(ahora, bytes_original, "y el de dentro también");
        assert!(
            std::fs::read_dir(lib.root().join("cache/versiones"))
                .unwrap()
                .next()
                .is_none(),
            "el temporal no se queda"
        );

        assert_eq!(malla(&lib).len(), 1, "en la malla sigue siendo una");
        lib.deshacer(&nada).unwrap();
        assert!(
            lib.load_item(&v).unwrap().trashed,
            "deshacer la manda a la papelera"
        );
    }

    #[test]
    fn una_receta_no_amplia_ni_acepta_formatos_raros() {
        let img = image::DynamicImage::ImageRgba8(image::RgbaImage::new(100, 50));
        let r = Receta {
            formato: "webp".into(),
            calidad: 80,
            lado: 4000,
            recorte: None,
            tramo: None,
        };
        let out = codificar(&img, "webp", &r).unwrap();
        let back = image::load_from_memory(&out).unwrap();
        assert_eq!((back.width(), back.height()), (100, 50));
        assert!(back.color().has_alpha(), "WebP conserva la transparencia");

        let (_d, mut lib) = biblioteca();
        let a = mete(&mut lib, "a", 1);
        let r = Receta {
            formato: "bmp".into(),
            calidad: 80,
            lado: 0,
            recorte: None,
            tramo: None,
        };
        assert!(lib.crear_version(&a, &r).is_err());
    }

    /// Una foto con ruido, para que comprimir se note y la huella sea de
    /// verdad, guardada en `dir` con `nombre`.
    fn foto(dir: &Path, nombre: &str, w: u32, h: u32, semilla: u32) -> PathBuf {
        // Un tablero cuyo tamaño de casilla depende de la semilla: dos semillas
        // son dos fotos distintas también para la huella, que mira formas y
        // no ruido. El ruido encima es para que comprimir se note.
        let lado = 40 + semilla * 23;
        let img = image::RgbImage::from_fn(w, h, |x, y| {
            let casilla = ((x / lado) + (y / (lado / 2 + 7))) % 2 == 0;
            let ruido = ((x * 7 + y * 13 + semilla * 31) % 37) as u8;
            let base = if casilla { 200 } else { 40 };
            image::Rgb([base + ruido / 2, base / 2 + ruido, ((x * 255) / w.max(1)) as u8])
        });
        let ruta = dir.join(nombre);
        img.save(&ruta).unwrap();
        ruta
    }

    fn importa(lib: &mut Library, rutas: &[PathBuf]) -> Vec<String> {
        import::import(lib, rutas, &ImportOptions::default(), &|_, _| {})
            .unwrap()
            .ids
    }

    #[test]
    fn la_estimacion_pesa_lo_mismo_que_la_version_y_no_crea_nada() {
        let (d, mut lib) = biblioteca();
        let id = importa(&mut lib, &[foto(d.path(), "a.png", 900, 600, 1)])[0].clone();
        let receta = Receta {
            formato: "jpg".into(),
            calidad: 60,
            lado: 400,
            recorte: None,
            tramo: None,
        };
        let e = lib.estimar_version(&id, &receta).unwrap();
        assert_eq!((e.ancho, e.alto), (400, 267));
        assert_eq!(lib.index().count().unwrap(), 1, "estimar no crea nada");
        let v = lib.crear_version(&id, &receta).unwrap();
        assert_eq!(lib.load_item(&v).unwrap().size, e.peso);
        assert_eq!(e.original, lib.load_item(&id).unwrap().size);
    }

    #[test]
    fn webp_con_perdida_pesa_menos_que_sin_perdida() {
        let (d, mut lib) = biblioteca();
        let id = importa(&mut lib, &[foto(d.path(), "a.png", 800, 800, 2)])[0].clone();
        let con = |q| Receta {
            formato: "webp".into(),
            calidad: q,
            lado: 0,
            recorte: None,
            tramo: None,
        };
        let perdida = lib.estimar_version(&id, &con(75)).unwrap().peso;
        let sin = lib.estimar_version(&id, &con(100)).unwrap().peso;
        assert!(perdida * 2 < sin, "{perdida} contra {sin}");
        let v = lib.crear_version(&id, &con(75)).unwrap();
        assert!(lib.load_item(&v).unwrap().name.contains("webp 75 %"));
    }

    #[test]
    fn una_version_de_fuera_se_copia_y_una_temporal_se_mueve() {
        let (d, mut lib) = biblioteca();
        let id = importa(&mut lib, &[foto(d.path(), "a.png", 300, 200, 3)])[0].clone();
        let receta = Receta {
            formato: "png".into(),
            calidad: 100,
            lado: 0,
            recorte: Some("isnet-general-use".into()),
            tramo: None,
        };
        let fuera = foto(d.path(), "recorte.png", 300, 200, 4);
        let v = lib.anadir_version(&id, &fuera, &receta).unwrap();
        assert!(fuera.exists(), "lo de fuera no se le quita a nadie");
        let it = lib.load_item(&v).unwrap();
        assert_eq!(it.pila.as_deref(), Some(id.as_str()));
        assert!(it.name.ends_with("(sin fondo)"), "{}", it.name);
        assert_eq!(lib.que_se_deshace(), Some("quitar el fondo"));

        let tmp = lib.temporal_de_version("png").unwrap();
        std::fs::copy(&fuera, &tmp).unwrap();
        lib.anadir_version(&id, &tmp, &receta).unwrap();
        assert!(!tmp.exists(), "el temporal entra moviéndolo");
    }

    #[test]
    fn al_importar_se_encuentra_la_misma_foto_en_otro_peso_y_no_otra() {
        let (d, mut lib) = biblioteca();
        let original = foto(d.path(), "grande.png", 1200, 800, 5);
        let a = importa(&mut lib, &[original.clone()])[0].clone();
        let otra = importa(&mut lib, &[foto(d.path(), "otra.png", 1200, 800, 9)])[0].clone();
        // La misma foto, reducida y en JPEG, que es lo que llega de una web.
        let img = image::open(&original).unwrap().resize(600, 600, image::imageops::FilterType::Triangle);
        let pequena = d.path().join("pequena.jpg");
        img.to_rgb8().save(&pequena).unwrap();
        let nuevo = importa(&mut lib, &[pequena])[0].clone();

        let p = lib.index().parecidos(std::slice::from_ref(&nuevo), 3).unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].nuevo, nuevo);
        assert_eq!(p[0].existente, a, "y no con {otra}");

        // Si la grande ya era versión de una pila, lo encontrado es la pila.
        let suelta = importa(&mut lib, &[foto(d.path(), "s.png", 50, 50, 11)])[0].clone();
        lib.apilar(&suelta, &[a.clone()]).unwrap();
        let p = lib.index().parecidos(std::slice::from_ref(&nuevo), 3).unwrap();
        assert_eq!(p[0].existente, suelta);
    }

    #[test]
    fn apilar_varios_grupos_es_un_solo_deshacer() {
        let (_d, mut lib) = biblioteca();
        let a = mete(&mut lib, "a", 3);
        let b = mete(&mut lib, "b", 2);
        let c = mete(&mut lib, "c", 3);
        let e = mete(&mut lib, "e", 2);
        let n = lib
            .apilar_grupos(&[(a.clone(), vec![b.clone()]), (c.clone(), vec![e.clone()])])
            .unwrap();
        assert_eq!(n, 2);
        assert_eq!(malla(&lib), vec!["a", "c"]);
        lib.deshacer(&nada).unwrap();
        assert_eq!(malla(&lib), vec!["a", "b", "c", "e"]);
    }

    /// Lo que dura un archivo según ffprobe, en segundos.
    fn duracion(ruta: &Path) -> f64 {
        let o = std::process::Command::new("ffprobe")
            .args(["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0"])
            .arg(ruta)
            .output()
            .unwrap();
        String::from_utf8_lossy(&o.stdout).trim().parse().unwrap()
    }

    #[test]
    fn un_tramo_de_un_video_entra_como_version_rapido_o_exacto() {
        if !crate::externo::hay("ffmpeg") || !crate::externo::hay("ffprobe") {
            eprintln!("sin ffmpeg o ffprobe: prueba saltada");
            return;
        }
        let (d, mut lib) = biblioteca();
        let mp4 = d.path().join("clip.mp4");
        crate::externo::correr(
            "ffmpeg",
            &[
                "-v", "error", "-f", "lavfi", "-i", "testsrc=size=160x90:rate=25:duration=6",
                "-f", "lavfi", "-i", "sine=frequency=440:duration=6", "-g", "250",
                "-pix_fmt", "yuv420p", "-shortest", "-y", &mp4.to_string_lossy(),
            ],
            std::time::Duration::from_secs(60),
        )
        .unwrap();
        let rep = import::import(&mut lib, &[mp4], &ImportOptions::default(), &|_, _| {}).unwrap();
        let id = rep.ids[0].clone();

        let rapido = lib
            .recortar_tramo(&id, Tramo { desde_ms: 2500, hasta_ms: 4500, exacto: false })
            .unwrap();
        let exacto = lib
            .recortar_tramo(&id, Tramo { desde_ms: 2500, hasta_ms: 4500, exacto: true })
            .unwrap();
        for v in [&rapido, &exacto] {
            let it = lib.load_item(v).unwrap();
            assert_eq!(it.pila.as_deref(), Some(id.as_str()), "va en la pila del original");
            assert_eq!(it.ext, "mp4");
            assert!(it.name.ends_with("(0:02,5–0:04,5)"), "{}", it.name);
        }
        // Los dos duran lo pedido: el rápido lleva desde el fotograma clave de
        // antes, pero su lista de edición lo deja en su sitio.
        let de = |v: &str| duracion(&lib.original_path(&lib.load_item(v).unwrap()));
        assert!((de(&exacto) - 2.0).abs() < 0.15, "{}", de(&exacto));
        assert!((de(&rapido) - 2.0).abs() < 0.3, "{}", de(&rapido));
        // Y se deshace como cualquier versión.
        lib.deshacer(&nada).unwrap();
        assert!(lib.load_item(&exacto).unwrap().trashed);
        assert!(lib.recortar_tramo(&id, Tramo { desde_ms: 3000, hasta_ms: 1000, exacto: false }).is_err());
    }
}
