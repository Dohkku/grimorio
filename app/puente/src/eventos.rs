//! Los eventos que el núcleo manda hacia la interfaz.
//!
//! Todos salen del hilo trabajador, así que quien los recibe tiene que
//! reenviarlos a su hilo de interfaz antes de tocar nada de la ventana.
//!
//! Van en JSON por la misma razón que los comandos: se pueden leer en un
//! registro, se pueden falsificar en una prueba, y el día que un plugin quiera
//! escuchar no hay que inventar un segundo idioma.

use crate::Aviso;
use grimorio_core::Library;
use serde::Serialize;

#[derive(Serialize)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum Evento<'a> {
    /// La biblioteca está abierta y el hilo en marcha.
    Listo {
        raiz: &'a str,
        nombre: &'a str,
        total: u64,
    },
    /// La biblioteca se ha renombrado.
    Nombre { nombre: &'a str },
    /// Hay una vista nueva publicada: la malla tiene que recogerla entera.
    Vista { cmd: u64, n: usize },
    /// El árbol de carpetas ha cambiado, con cuántos elementos hay en cada una.
    Carpetas {
        arbol: Vec<CarpetaJson<'a>>,
        conteos: std::collections::HashMap<String, u64>,
        /// Cuántos hay en la papelera. Viaja con el árbol porque se pinta en el
        /// mismo sitio y cambia por los mismos motivos.
        papelera: u64,
        /// Cuántos hay en la biblioteca sin contar la papelera.
        ///
        /// Viaja aquí y no solo en «listo» porque si no, el «Todo 100.000» de
        /// la barra lateral se queda con el número del arranque: tiras tres
        /// elementos y sigue diciendo cien mil.
        total: u64,
        /// Cuántos hay marcados como adultos. Viaja aquí por lo mismo: cambia
        /// cuando cambia el resto y se mira desde la misma barra.
        adultos: u64,
        /// Cuánto hay de cada familia y cuánto pesa: el pie de la barra
        /// lateral. Cambia con lo mismo que el total, así que viaja con él.
        reparto: Vec<RepartoJson>,
    },
    /// Todas las etiquetas con su número, para el gestor. Va aparte de
    /// `etiquetas`, que es la del autocompletado y se pide por prefijo.
    TodasEtiquetas { lista: Vec<EtiquetaJson> },
    /// Las carpetas del disco que se importan solas.
    Vigiladas {
        lista: &'a [grimorio_core::vigiladas::Vigilada],
    },
    /// Los grupos de etiquetas, con su color y sus etiquetas.
    Grupos {
        lista: &'a [grimorio_core::grupos::Grupo],
    },
    /// Las carpetas inteligentes, con cuántos da cada una ahora.
    Busquedas { lista: Vec<BusquedaJson<'a>> },
    /// Un solo elemento ha cambiado, con su estado nuevo dentro.
    ///
    /// Lleva los datos y no solo el id a propósito: la vista publicada es de
    /// antes del cambio, así que avisar "el elemento X cambió" sin decir a qué
    /// dejaba la estrella sin aparecer hasta la siguiente consulta. Y volver a
    /// consultar entera la biblioteca por cada estrella son ochenta y cinco
    /// milisegundos por pulsación.
    Item {
        cmd: u64,
        id: &'a str,
        nombre: &'a str,
        estrellas: u8,
        etiquetas: &'a [String],
        nota: Option<&'a str>,
        carpetas: &'a [String],
        adulto: bool,
    },
    /// Algo largo va por la mitad.
    Progreso {
        cmd: u64,
        que: &'a str,
        hechos: usize,
        total: usize,
    },
    /// La ficha completa de un elemento.
    ///
    /// Va en un evento aparte y no dentro de la vista porque la vista lleva
    /// solo lo que la malla necesita para pintar —cien mil de esos tienen que
    /// caber en memoria— y el panel de detalle enseña una sola cosa cada vez.
    Ficha {
        cmd: u64,
        id: &'a str,
        nombre: &'a str,
        ext: &'a str,
        familia: &'a str,
        peso: u64,
        ancho: u32,
        alto: u32,
        duracion_s: Option<u64>,
        paginas: Option<u32>,
        triangulos: Option<u64>,
        medidas_mm: Option<[f32; 3]>,
        estrellas: u8,
        etiquetas: &'a [String],
        nota: Option<&'a str>,
        carpetas: &'a [String],
        importado: &'a str,
        modificado: &'a str,
        adulto: bool,
        origen: &'a str,
        ruta: String,
        fuente: Option<&'a str>,
        papelera: bool,
        /// La paleta, de más a menos presente: `{color: "#rrggbb", peso}`.
        paleta: Vec<MuestraJson>,
        /// De dónde se importó, tal como estaba en el disco entonces.
        procedencia: Option<&'a str>,
    },
    /// Las etiquetas que hay, para el autocompletado.
    Etiquetas { cmd: u64, lista: Vec<EtiquetaJson> },
    /// Qué se puede deshacer y qué se puede rehacer, con su nombre.
    ///
    /// Va con el título dentro para que el menú pueda decir «Deshacer poner 4
    /// estrellas» en vez de «Deshacer» a secas: saber qué va a pasar antes de
    /// pulsar es la mitad de para qué sirve un deshacer.
    Registro {
        deshacer: Option<&'a str>,
        rehacer: Option<&'a str>,
    },
    /// Un comando terminó bien.
    /// `id` va aparte del mensaje: el mensaje se enseña en la barra de estado,
    /// y antes llevaba ahí el id de la carpeta recién creada —un churro de
    /// letras para quien lo leía—.
    Hecho {
        cmd: u64,
        mensaje: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    /// Un comando terminó mal. Nunca es fatal: la interfaz lo enseña y sigue.
    Error { cmd: u64, mensaje: &'a str },
}

#[derive(Serialize)]
pub struct EtiquetaJson {
    pub nombre: String,
    pub n: u64,
}

#[derive(Serialize)]
pub struct MuestraJson {
    pub color: String,
    pub peso: f32,
}

#[derive(Serialize)]
pub struct RepartoJson {
    pub familia: String,
    pub n: u64,
    pub bytes: u64,
}

#[derive(Serialize)]
pub struct CarpetaJson<'a> {
    pub id: &'a str,
    pub nombre: &'a str,
    pub padre: Option<&'a str>,
    pub color: Option<&'a str>,
    pub pos: i64,
}

fn manda(aviso: &Aviso, e: &Evento<'_>) {
    match serde_json::to_vec(e) {
        Ok(bytes) => aviso.manda(&bytes),
        // Serializar un evento no puede fallar con estos tipos, pero si algún
        // día falla, callarse sería peor que un mensaje feo.
        Err(err) => {
            let apaño = format!(
                r#"{{"tipo":"error","cmd":0,"mensaje":"evento no serializable: {}"}}"#,
                err.to_string().replace('"', "'")
            );
            aviso.manda(apaño.as_bytes());
        }
    }
}

pub fn listo(aviso: &Aviso, lib: &Library) {
    let total = lib.index().count().unwrap_or(0);
    manda(
        aviso,
        &Evento::Listo {
            raiz: &lib.root().to_string_lossy(),
            nombre: &lib.meta().name,
            total,
        },
    );
}

pub fn nombre(aviso: &Aviso, lib: &Library) {
    manda(aviso, &Evento::Nombre { nombre: &lib.meta().name });
}

pub fn vista(aviso: &Aviso, cmd: u64, n: &usize) {
    manda(aviso, &Evento::Vista { cmd, n: *n });
}

pub fn todas_etiquetas(aviso: &Aviso, lib: &Library) {
    let lista = lib
        .index()
        .tags_con_prefijo("", 100_000)
        .unwrap_or_default()
        .into_iter()
        .map(|(nombre, n)| EtiquetaJson { nombre, n })
        .collect();
    manda(aviso, &Evento::TodasEtiquetas { lista });
}

pub fn vigiladas(aviso: &Aviso, lib: &Library) {
    let lista = grimorio_core::vigiladas::cargar(lib.root()).unwrap_or_default();
    manda(aviso, &Evento::Vigiladas { lista: &lista });
}

pub fn grupos(aviso: &Aviso, lib: &Library) {
    let lista = grimorio_core::grupos::cargar(lib.root()).unwrap_or_default();
    manda(aviso, &Evento::Grupos { lista: &lista });
}

#[derive(Serialize)]
pub struct BusquedaJson<'a> {
    id: &'a str,
    nombre: &'a str,
    consulta: &'a str,
    cuenta: u64,
}

/// Las carpetas inteligentes y su número. Va con las carpetas —se manda cada
/// vez que se mandan ellas— porque cambia por los mismos motivos: importar,
/// tirar, mover.
pub fn busquedas(aviso: &Aviso, lib: &Library) {
    let todas = grimorio_core::busquedas::cargar(lib.root()).unwrap_or_default();
    let base = grimorio_core::Query {
        limit: 0,
        ..Default::default()
    };
    let lista = todas
        .iter()
        .map(|b| {
            let (q, _) = grimorio_core::filtro::parsear(&b.consulta, base.clone());
            BusquedaJson {
                id: &b.id,
                nombre: &b.nombre,
                consulta: &b.consulta,
                cuenta: lib.index().contar(&q).unwrap_or(0),
            }
        })
        .collect();
    manda(aviso, &Evento::Busquedas { lista });
}

pub fn carpetas(aviso: &Aviso, lib: &Library) {
    let conteos = lib.index().folder_counts().unwrap_or_default();
    let arbol: Vec<CarpetaJson> = lib
        .folders()
        .folders
        .iter()
        .map(|f| CarpetaJson {
            id: &f.id,
            nombre: &f.name,
            padre: f.parent.as_deref(),
            color: f.color.as_deref(),
            pos: f.pos,
        })
        .collect();
    let papelera = lib.index().contar_papelera().unwrap_or(0);
    let total = lib.index().count().unwrap_or(0);
    let adultos = lib.index().contar_adultos().unwrap_or(0);
    let reparto = lib
        .index()
        .reparto()
        .unwrap_or_default()
        .into_iter()
        .map(|(familia, n, bytes)| RepartoJson { familia, n, bytes })
        .collect();
    manda(
        aviso,
        &Evento::Carpetas {
            arbol,
            conteos,
            papelera,
            total,
            adultos,
            reparto,
        },
    );
    busquedas(aviso, lib);
}

pub fn item(aviso: &Aviso, cmd: u64, it: &grimorio_core::Item) {
    manda(
        aviso,
        &Evento::Item {
            cmd,
            id: &it.id,
            nombre: &it.name,
            estrellas: it.stars,
            etiquetas: &it.tags,
            nota: it.note.as_deref(),
            carpetas: &it.folders,
            adulto: it.adult,
        },
    );
}

pub fn ficha(aviso: &Aviso, cmd: u64, lib: &Library, it: &grimorio_core::Item) {
    manda(
        aviso,
        &Evento::Ficha {
            cmd,
            id: &it.id,
            nombre: &it.name,
            ext: &it.ext,
            familia: it.kind.as_str(),
            peso: it.size,
            ancho: it.width,
            alto: it.height,
            duracion_s: it.duration_ms.map(|ms| ms / 1000),
            paginas: it.pages,
            triangulos: it.triangles,
            medidas_mm: it.size_mm,
            estrellas: it.stars,
            etiquetas: &it.tags,
            nota: it.note.as_deref(),
            carpetas: &it.folders,
            importado: &it.imported_at,
            modificado: &it.modified_at,
            adulto: it.adult,
            origen: it.origin.mode.as_str(),
            ruta: lib.original_path(it).to_string_lossy().into_owned(),
            fuente: it.source.as_deref(),
            papelera: it.trashed,
            paleta: it
                .palette
                .iter()
                .map(|p| MuestraJson {
                    color: format!("#{:02x}{:02x}{:02x}", p.rgb[0], p.rgb[1], p.rgb[2]),
                    peso: p.w,
                })
                .collect(),
            procedencia: it.origin.path.as_deref(),
        },
    );
}

pub fn etiquetas(aviso: &Aviso, cmd: u64, lista: Vec<(String, u64)>) {
    manda(
        aviso,
        &Evento::Etiquetas {
            cmd,
            lista: lista
                .into_iter()
                .map(|(nombre, n)| EtiquetaJson { nombre, n })
                .collect(),
        },
    );
}

/// Se manda **después de cada cambio**, no solo cuando se deshace algo: el
/// menú tiene que enterarse de que ahora hay algo que deshacer sin preguntar.
pub fn registro(aviso: &Aviso, lib: &Library) {
    manda(
        aviso,
        &Evento::Registro {
            deshacer: lib.que_se_deshace(),
            rehacer: lib.que_se_rehace(),
        },
    );
}

pub fn progreso(aviso: &Aviso, cmd: u64, que: &str, hechos: usize, total: usize) {
    manda(
        aviso,
        &Evento::Progreso {
            cmd,
            que,
            hechos,
            total,
        },
    );
}

pub fn hecho(aviso: &Aviso, cmd: u64, mensaje: String) {
    manda(aviso, &Evento::Hecho { cmd, mensaje, id: None });
}

/// Hecho, y con el id de lo que se acaba de crear.
pub fn hecho_con_id(aviso: &Aviso, cmd: u64, mensaje: String, id: String) {
    manda(aviso, &Evento::Hecho { cmd, mensaje, id: Some(id) });
}

pub fn error(aviso: &Aviso, cmd: u64, mensaje: &str) {
    manda(aviso, &Evento::Error { cmd, mensaje });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_json_de_un_evento_lleva_su_tipo_delante() {
        let e = Evento::Vista { cmd: 7, n: 100 };
        let s = serde_json::to_string(&e).unwrap();
        assert_eq!(s, r#"{"tipo":"vista","cmd":7,"n":100}"#);
    }

    #[test]
    fn un_error_con_comillas_no_rompe_el_json() {
        let e = Evento::Error {
            cmd: 1,
            mensaje: r#"no encuentro «foto "buena".jpg»"#,
        };
        let s = serde_json::to_string(&e).unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).expect("tiene que seguir siendo JSON");
        assert_eq!(v["tipo"], "error");
        assert!(v["mensaje"].as_str().unwrap().contains("buena"));
    }
}
