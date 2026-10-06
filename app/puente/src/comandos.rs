//! Los comandos que la interfaz manda al núcleo.
//!
//! Un enum con `tag = "cmd"`: el JSON que llega de QML se convierte en una
//! variante o en un error legible, nunca en un comportamiento a medias.
//!
//! Cada comando termina en un evento, siempre. Un botón que no responde es peor
//! que un botón que da un error.

use crate::eventos;
use crate::vista::Vista;
use crate::Aviso;
use grimorio_core::import::{self, ImportOptions};
use grimorio_core::{filtro, Error, Item, Library, Operacion, Parche, Query, Result, SortBy};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Comando {
    /// Rehace la vista. Es el comando más frecuente: cambiar de carpeta,
    /// escribir en el buscador, cambiar el orden.
    Consulta {
        #[serde(default)]
        q: Query,
        /// Lo que hay escrito en el buscador, tal cual.
        ///
        /// Se lee aquí y no en QML porque el lenguaje de filtros tiene pruebas
        /// en Rust y porque el día que exista una carpeta inteligente será
        /// exactamente esta cadena guardada.
        #[serde(default)]
        filtro: Option<String>,
    },
    /// Vuelve a mandar el árbol de carpetas con sus conteos.
    Carpetas,
    CrearCarpeta {
        nombre: String,
        #[serde(default)]
        padre: Option<String>,
    },
    /// Guarda la línea del buscador como carpeta inteligente.
    GuardarBusqueda {
        nombre: String,
        consulta: String,
    },
    RenombrarBusqueda {
        id: String,
        nombre: String,
    },
    BorrarBusqueda {
        id: String,
    },
    RenombrarCarpeta {
        id: String,
        nombre: String,
    },
    /// Color de la carpeta en "#rrggbb"; vacío o sin poner, se lo quita.
    ColorCarpeta {
        id: String,
        #[serde(default)]
        color: Option<String>,
    },
    MoverCarpeta {
        id: String,
        #[serde(default)]
        padre: Option<String>,
        /// La hermana delante de la cual queda. Sin ella, la última.
        #[serde(default)]
        antes_de: Option<String>,
    },
    BorrarCarpeta {
        id: String,
    },
    /// Añade o quita carpetas a una selección sin tocar el resto. Es lo que
    /// hace arrastrar elementos a la barra lateral.
    CarpetasDeElementos {
        ids: Vec<String>,
        #[serde(default)]
        anadir: Vec<String>,
        #[serde(default)]
        quitar: Vec<String>,
    },
    Estrellas {
        ids: Vec<String>,
        valor: u8,
    },
    Etiquetar {
        ids: Vec<String>,
        #[serde(default)]
        anadir: Vec<String>,
        #[serde(default)]
        quitar: Vec<String>,
    },
    Nota {
        id: String,
        texto: String,
    },
    Renombrar {
        id: String,
        nombre: String,
    },
    /// Manda a la papelera o saca de ella.
    Papelera {
        ids: Vec<String>,
        #[serde(default)]
        dentro: bool,
    },
    /// Pide la lista entera de etiquetas para el gestor.
    TodasEtiquetas,
    /// Pone `id` entre `antes` y `despues` en el orden a mano de `carpeta`
    /// (sin carpeta, el de «Todo»). Cualquiera de los dos vecinos puede
    /// faltar: es el principio o el final.
    Reordenar {
        id: String,
        #[serde(default)]
        carpeta: Option<String>,
        #[serde(default)]
        antes: Option<String>,
        #[serde(default)]
        despues: Option<String>,
    },
    /// Cambia el nombre de una etiqueta en todos los elementos que la llevan.
    /// Si la nueva ya existe, quedan fusionadas.
    RenombrarEtiqueta {
        vieja: String,
        nueva: String,
    },
    /// Quita una etiqueta de todos los elementos.
    BorrarEtiqueta {
        nombre: String,
    },
    CrearGrupo {
        nombre: String,
        color: String,
    },
    RenombrarGrupo {
        id: String,
        nombre: String,
    },
    ColorGrupo {
        id: String,
        color: String,
    },
    BorrarGrupo {
        id: String,
    },
    /// Mete una etiqueta en un grupo; sin grupo, la deja suelta.
    AgruparEtiqueta {
        etiqueta: String,
        #[serde(default)]
        grupo: Option<String>,
    },
    /// Renombra varios con un patrón (ver grimorio_core::patron). El contador
    /// sigue el orden de `ids`, empezando en `inicio`.
    RenombrarEnLote {
        ids: Vec<String>,
        patron: String,
        #[serde(default = "uno")]
        inicio: u64,
    },
    /// De cada grupo de copias exactas deja la importada primero y manda las
    /// demás a la papelera. Solo exactas: las casi iguales pueden ser variantes
    /// que se quieren (un recorte, otra compresión) y esas se revisan a mano.
    QuitarCopiasExactas,
    /// Marca o desmarca como contenido adulto.
    Adulto {
        ids: Vec<String>,
        #[serde(default)]
        si: bool,
    },
    /// Borra de verdad lo que hay en la papelera. No tiene vuelta.
    VaciarPapelera,
    Deshacer,
    Rehacer,
    /// Pide las etiquetas que empiezan por un prefijo, para autocompletar.
    Etiquetas {
        #[serde(default)]
        prefijo: String,
        #[serde(default = "tope_etiquetas")]
        tope: usize,
    },
    Importar {
        rutas: Vec<String>,
        #[serde(default)]
        carpeta: Option<String>,
        #[serde(default)]
        etiquetas: Vec<String>,
        /// La URL de la que viene, para lo pegado o descargado.
        #[serde(default)]
        origen: Option<String>,
        /// Mover el archivo en vez de seguir el modo de la biblioteca. Para
        /// lo que llega por el portapapeles o la red: es un archivo temporal
        /// que nadie más quiere.
        #[serde(default)]
        mover: bool,
        /// Copiar siempre, aunque la biblioteca esté en «mover»: lo que llega
        /// de una carpeta vigilada sigue siendo de esa carpeta.
        #[serde(default)]
        copiar: bool,
    },
    /// Empieza a vigilar una carpeta del disco; lo nuevo entra en `carpeta`.
    VigilarCarpeta {
        ruta: String,
        #[serde(default)]
        carpeta: Option<String>,
    },
    DejarDeVigilar {
        id: String,
    },
    /// Lo que el vigía ha encontrado nuevo en una carpeta vigilada. `hasta`
    /// es cuándo empezó a mirar: lo cambiado antes ya está visto.
    ImportarVigilada {
        id: String,
        rutas: Vec<String>,
        hasta: u64,
    },
    /// Pide la ficha completa de un elemento, para el panel de detalle.
    Ficha {
        id: String,
    },
    /// Abre el original con el visor del sistema.
    AbrirFuera {
        id: String,
    },
}

/// Cuántas etiquetas se ofrecen si nadie dice otra cosa. Un desplegable más
/// largo que esto no se lee, se cierra.
fn tope_etiquetas() -> usize {
    40
}

fn uno() -> u64 {
    1
}

/// Los ids de lo que lleva una etiqueta, fuera de la papelera también: una
/// etiqueta renombrada tiene que seguir siéndolo al sacar algo de ahí.
fn ids_con_etiqueta(lib: &Library, etiqueta: &str) -> Result<Vec<String>> {
    let mut ids = Vec::new();
    for papelera in [false, true] {
        let q = Query {
            tags: vec![etiqueta.to_string()],
            papelera,
            limit: 0,
            ..Default::default()
        };
        ids.extend(lib.index().search(&q)?.into_iter().map(|h| h.id));
    }
    Ok(ids)
}

/// Las posiciones nuevas para poner `id` entre `antes` y `despues` en el
/// orden a mano de `ctx`.
///
/// Casi siempre es una sola: el punto medio entre los dos vecinos. Pero lo que
/// no tiene posición puesta va por su fecha de importación, y lo importado de
/// una vez comparte milisegundo: entre dos vecinos con la misma posición no
/// hay punto medio. Lo mismo después de partir muchas veces el mismo hueco.
/// Entonces se reparte un tramo alrededor —con posiciones espaciadas entre los
/// límites del tramo—, y el tramo crece hasta que entre sus límites hay sitio.
/// Todo va en la misma operación: se deshace de una vez.
fn posiciones_al_mover(
    lib: &Library,
    ctx: &str,
    id: &str,
    antes: Option<&str>,
    despues: Option<&str>,
) -> Result<std::collections::BTreeMap<String, f64>> {
    let q = Query {
        sort: SortBy::Manual,
        folder: if ctx.is_empty() { None } else { Some(ctx.to_string()) },
        limit: 0,
        ..Default::default()
    };
    let hits = lib.index().search(&q)?;
    let puestas = lib.index().posiciones(ctx)?;
    let mut orden: Vec<(String, f64)> = hits
        .into_iter()
        .filter(|h| h.id != id)
        .map(|h| {
            let p = puestas
                .get(&h.id)
                .copied()
                .unwrap_or(-(h.imported_at_ms as f64));
            (h.id, p)
        })
        .collect();
    let donde = |v: &str| orden.iter().position(|(x, _)| x == v);
    let k = match (antes.and_then(donde), despues.and_then(donde)) {
        (Some(a), _) => a + 1,
        (None, Some(d)) => d,
        (None, None) => return Ok(Default::default()),
    };
    orden.insert(k, (id.to_string(), f64::NAN));
    let n = orden.len();

    // El límite de un lado: la posición del vecino, o una más allá del final.
    let limite = |i: isize| -> f64 {
        if i < 0 {
            orden.iter().filter(|x| !x.1.is_nan()).map(|x| x.1).fold(f64::INFINITY, f64::min) - 1000.0
        } else if i as usize >= n {
            orden.iter().filter(|x| !x.1.is_nan()).map(|x| x.1).fold(f64::NEG_INFINITY, f64::max) + 1000.0
        } else {
            orden[i as usize].1
        }
    };
    let mut radio = 0isize;
    loop {
        let desde = k as isize - radio;
        let hasta = k as isize + radio; // incluido
        let lo = limite(desde - 1);
        let hi = limite(hasta + 1);
        let cuantos = (hasta - desde + 1) as f64;
        let paso = (hi - lo) / (cuantos + 1.0);
        // Hueco de sobra para que el punto no se confunda con sus vecinos.
        let cabe = hi > lo && paso > (hi.abs().max(lo.abs()) * 1e-12).max(1e-9);
        if cabe || (desde <= 0 && hasta as usize >= n - 1) {
            let mut nuevas = std::collections::BTreeMap::new();
            for i in desde.max(0)..=hasta.min(n as isize - 1) {
                let destino = lo + paso * ((i - desde) as f64 + 1.0);
                let (ref vid, viejo) = orden[i as usize];
                if viejo.is_nan() || (viejo - destino).abs() > f64::EPSILON {
                    nuevas.insert(vid.clone(), destino);
                }
            }
            return Ok(nuevas);
        }
        radio = if radio == 0 { 1 } else { radio * 2 };
    }
}

pub fn ejecutar(
    lib: &mut Library,
    cmd: &Comando,
    id_cmd: u64,
    aviso: &Aviso,
    vista: &Arc<Mutex<Arc<Vista>>>,
) -> Result<()> {
    match cmd {
        Comando::Consulta { q, filtro } => {
            let (q, avisos) = match filtro {
                Some(t) => filtro::parsear(t, q.clone()),
                None => (q.clone(), Vec::new()),
            };
            republicar(lib, q, id_cmd, aviso, vista)?;
            // Los avisos del buscador van por el mismo canal que cualquier otro
            // mensaje corto: no bloquean la búsqueda, solo la explican.
            if !avisos.is_empty() {
                eventos::hecho(aviso, id_cmd, avisos.join("; "));
            }
        }

        Comando::Carpetas => eventos::carpetas(aviso, lib),

        Comando::CrearCarpeta { nombre, padre } => {
            let nuevo = lib.create_folder(nombre, padre.as_deref())?;
            eventos::carpetas(aviso, lib);
            eventos::registro(aviso, lib);
            eventos::hecho_con_id(aviso, id_cmd, format!("carpeta «{}» creada", nombre.trim()), nuevo);
        }

        Comando::GuardarBusqueda { nombre, consulta } => {
            let id = grimorio_core::busquedas::anadir(lib.root(), nombre, consulta)?;
            eventos::busquedas(aviso, lib);
            eventos::hecho_con_id(
                aviso,
                id_cmd,
                format!("«{}» guardada como carpeta inteligente", nombre.trim()),
                id,
            );
        }

        Comando::RenombrarBusqueda { id, nombre } => {
            grimorio_core::busquedas::renombrar(lib.root(), id, nombre)?;
            eventos::busquedas(aviso, lib);
            eventos::hecho(aviso, id_cmd, String::new());
        }

        Comando::BorrarBusqueda { id } => {
            grimorio_core::busquedas::borrar(lib.root(), id)?;
            eventos::busquedas(aviso, lib);
            eventos::hecho(aviso, id_cmd, String::new());
        }

        Comando::RenombrarCarpeta { id, nombre } => {
            lib.rename_folder(id, nombre)?;
            eventos::carpetas(aviso, lib);
            eventos::registro(aviso, lib);
            eventos::hecho(aviso, id_cmd, String::new());
        }

        Comando::ColorCarpeta { id, color } => {
            lib.color_folder(id, color.as_deref())?;
            eventos::carpetas(aviso, lib);
            eventos::registro(aviso, lib);
            eventos::hecho(aviso, id_cmd, String::new());
        }

        Comando::MoverCarpeta {
            id,
            padre,
            antes_de,
        } => {
            lib.move_folder(id, padre.as_deref(), antes_de.as_deref())?;
            eventos::carpetas(aviso, lib);
            eventos::registro(aviso, lib);
            eventos::hecho(aviso, id_cmd, String::new());
        }

        Comando::BorrarCarpeta { id } => {
            let sueltos = lib.delete_folder(id)?;
            eventos::carpetas(aviso, lib);
            eventos::registro(aviso, lib);
            // La vista puede estar enseñando justo esa carpeta: hay que
            // rehacerla o se queda enseñando algo que ya no existe.
            let q = consulta_actual(vista);
            republicar(lib, q, id_cmd, aviso, vista)?;
            eventos::hecho(
                aviso,
                id_cmd,
                format!("{sueltos} elemento{} sin carpeta", plural(sueltos)),
            );
        }

        Comando::CarpetasDeElementos {
            ids,
            anadir,
            quitar,
        } => {
            // Las carpetas que no existan se descartan aquí y no dentro del
            // lote: comprobarlo una vez en vez de diez mil, y así el cierre no
            // necesita la biblioteca.
            let ponen: Vec<String> = anadir
                .iter()
                .filter(|f| lib.folders().existe(f))
                .cloned()
                .collect();
            // Las dos cosas a la vez son un movimiento, y así se llama: es lo
            // que va a leerse en «deshacer …» un segundo después.
            let titulo = match (ponen.is_empty(), quitar.is_empty()) {
                (false, false) => "mover de carpeta",
                (false, true) => "meter en la carpeta",
                _ => "sacar de la carpeta",
            };
            let (tocados, _) = en_lote(
                lib,
                ids,
                id_cmd,
                aviso,
                vista,
                titulo,
                Toca::carpetas(),
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
            )?;
            eventos::carpetas(aviso, lib);
            eventos::hecho(
                aviso,
                id_cmd,
                format!(
                    "{tocados} elemento{} movido{}",
                    plural(tocados),
                    plural(tocados)
                ),
            );
        }

        Comando::Estrellas { ids, valor } => {
            let v = (*valor).min(5);
            let titulo = if v == 0 {
                "quitar las estrellas".to_string()
            } else {
                format!("poner {v} estrella{}", plural(v as usize))
            };
            en_lote(
                lib,
                ids,
                id_cmd,
                aviso,
                vista,
                &titulo,
                Toca::estrellas(),
                |_| Parche::estrellas(v),
            )?;
            eventos::hecho(aviso, id_cmd, String::new());
        }

        Comando::Etiquetar {
            ids,
            anadir,
            quitar,
        } => {
            let titulo = match (anadir.first(), quitar.first()) {
                (Some(t), _) => format!("etiquetar como «{t}»"),
                (None, Some(t)) => format!("quitar «{t}»"),
                _ => "etiquetar".to_string(),
            };
            let (tocados, _) = en_lote(
                lib,
                ids,
                id_cmd,
                aviso,
                vista,
                &titulo,
                Toca::etiquetas(),
                |item| {
                    let mut v = item.tags.clone();
                    for t in anadir {
                        let t = t.trim();
                        if !t.is_empty() && !v.iter().any(|x| x == t) {
                            v.push(t.to_string());
                        }
                    }
                    v.retain(|t| !quitar.iter().any(|x| x == t));
                    Parche::etiquetas(v)
                },
            )?;
            // Las etiquetas cambiaron, así que el autocompletado también.
            eventos::etiquetas(
                aviso,
                id_cmd,
                lib.index().tags_con_prefijo("", tope_etiquetas())?,
            );
            eventos::hecho(
                aviso,
                id_cmd,
                format!(
                    "{tocados} elemento{} etiquetado{}",
                    plural(tocados),
                    plural(tocados)
                ),
            );
        }

        Comando::Nota { id, texto } => {
            let titulo = if texto.trim().is_empty() {
                "borrar la nota"
            } else {
                "escribir una nota"
            };
            en_lote(
                lib,
                std::slice::from_ref(id),
                id_cmd,
                aviso,
                vista,
                titulo,
                Toca::nota(),
                |_| Parche::nota(texto),
            )?;
            eventos::hecho(aviso, id_cmd, String::new());
        }

        Comando::Renombrar { id, nombre } => {
            if nombre.trim().is_empty() {
                return Err(Error::Invalid(
                    "un elemento sin nombre no se encuentra".into(),
                ));
            }
            en_lote(
                lib,
                std::slice::from_ref(id),
                id_cmd,
                aviso,
                vista,
                "renombrar",
                Toca::nombre(),
                |_| Parche::nombre(nombre),
            )?;
            eventos::hecho(aviso, id_cmd, String::new());
        }

        Comando::VigilarCarpeta { ruta, carpeta } => {
            if let Some(c) = carpeta {
                if !lib.folders().existe(c) {
                    return Err(Error::Invalid(format!("no existe la carpeta {c}")));
                }
            }
            let id = grimorio_core::vigiladas::anadir(lib.root(), ruta, carpeta.as_deref())?;
            eventos::vigiladas(aviso, lib);
            eventos::hecho_con_id(aviso, id_cmd, format!("vigilando {ruta}"), id);
        }

        Comando::DejarDeVigilar { id } => {
            grimorio_core::vigiladas::quitar(lib.root(), id)?;
            eventos::vigiladas(aviso, lib);
            eventos::hecho(aviso, id_cmd, "ya no se vigila".to_string());
        }

        Comando::ImportarVigilada { id, rutas, hasta } => {
            let todas = grimorio_core::vigiladas::cargar(lib.root())?;
            // Si se dejó de vigilar mientras tanto, lo encontrado no entra.
            let Some(v) = todas.into_iter().find(|v| v.id == *id) else {
                return Ok(());
            };
            // La carpeta de destino puede haberse borrado: entonces entra suelto.
            let carpeta = v.carpeta.filter(|c| lib.folders().existe(c));
            let importar = Comando::Importar {
                rutas: rutas.clone(),
                carpeta,
                etiquetas: Vec::new(),
                origen: None,
                mover: false,
                copiar: true,
            };
            ejecutar(lib, &importar, id_cmd, aviso, vista)?;
            grimorio_core::vigiladas::marcar(lib.root(), id, *hasta)?;
            eventos::vigiladas(aviso, lib);
        }

        Comando::Reordenar {
            id,
            carpeta,
            antes,
            despues,
        } => {
            let ctx = carpeta.clone().unwrap_or_default();
            let nuevas = posiciones_al_mover(lib, &ctx, id, antes.as_deref(), despues.as_deref())?;
            if nuevas.is_empty() {
                return Ok(());
            }
            let ids: Vec<String> = nuevas.keys().cloned().collect();
            en_lote(
                lib,
                &ids,
                id_cmd,
                aviso,
                vista,
                "cambiar el orden",
                Toca::orden(),
                |it| {
                    let mut mapa = it.orden.clone();
                    if let Some(p) = nuevas.get(&it.id) {
                        mapa.insert(ctx.clone(), *p);
                    }
                    Parche::orden(mapa)
                },
            )?;
            eventos::hecho(aviso, id_cmd, String::new());
        }

        Comando::TodasEtiquetas => {
            eventos::todas_etiquetas(aviso, lib);
            eventos::grupos(aviso, lib);
        }

        Comando::RenombrarEtiqueta { vieja, nueva } => {
            let nueva = nueva.trim();
            if nueva.is_empty() || nueva == vieja {
                return Err(Error::Invalid("hace falta un nombre nuevo".into()));
            }
            let ids = ids_con_etiqueta(lib, vieja)?;
            let titulo = format!("renombrar «{vieja}» a «{nueva}»");
            let (tocados, _) = en_lote(lib, &ids, id_cmd, aviso, vista, &titulo, Toca::etiquetas(), |it| {
                let mut v: Vec<String> = Vec::with_capacity(it.tags.len());
                for t in &it.tags {
                    let t = if t == vieja { nueva } else { t.as_str() };
                    if !v.iter().any(|x| x == t) {
                        v.push(t.to_string());
                    }
                }
                Parche::etiquetas(v)
            })?;
            grimorio_core::grupos::renombrar_etiqueta(lib.root(), vieja, nueva)?;
            eventos::grupos(aviso, lib);
            eventos::etiquetas(aviso, id_cmd, lib.index().tags_con_prefijo("", tope_etiquetas())?);
            eventos::todas_etiquetas(aviso, lib);
            eventos::hecho(aviso, id_cmd, format!("«{vieja}» → «{nueva}» en {tocados}"));
        }

        Comando::BorrarEtiqueta { nombre } => {
            let ids = ids_con_etiqueta(lib, nombre)?;
            let titulo = format!("quitar «{nombre}» de todo");
            let (tocados, _) = en_lote(lib, &ids, id_cmd, aviso, vista, &titulo, Toca::etiquetas(), |it| {
                Parche::etiquetas(it.tags.iter().filter(|t| *t != nombre).cloned().collect())
            })?;
            grimorio_core::grupos::quitar_etiqueta(lib.root(), nombre)?;
            eventos::grupos(aviso, lib);
            eventos::etiquetas(aviso, id_cmd, lib.index().tags_con_prefijo("", tope_etiquetas())?);
            eventos::todas_etiquetas(aviso, lib);
            eventos::hecho(aviso, id_cmd, format!("«{nombre}» quitada de {tocados}"));
        }

        Comando::CrearGrupo { nombre, color } => {
            let id = grimorio_core::grupos::crear(lib.root(), nombre, color)?;
            eventos::grupos(aviso, lib);
            eventos::hecho_con_id(aviso, id_cmd, format!("grupo «{}» creado", nombre.trim()), id);
        }
        Comando::RenombrarGrupo { id, nombre } => {
            grimorio_core::grupos::renombrar(lib.root(), id, nombre)?;
            eventos::grupos(aviso, lib);
            eventos::hecho(aviso, id_cmd, String::new());
        }
        Comando::ColorGrupo { id, color } => {
            grimorio_core::grupos::colorear(lib.root(), id, color)?;
            eventos::grupos(aviso, lib);
            eventos::hecho(aviso, id_cmd, String::new());
        }
        Comando::BorrarGrupo { id } => {
            grimorio_core::grupos::borrar(lib.root(), id)?;
            eventos::grupos(aviso, lib);
            eventos::hecho(aviso, id_cmd, String::new());
        }
        Comando::AgruparEtiqueta { etiqueta, grupo } => {
            grimorio_core::grupos::agrupar(lib.root(), etiqueta, grupo.as_deref())?;
            eventos::grupos(aviso, lib);
            eventos::hecho(aviso, id_cmd, String::new());
        }

        Comando::RenombrarEnLote { ids, patron, inicio } => {
            let orden: std::collections::HashMap<&str, u64> = ids
                .iter()
                .enumerate()
                .map(|(i, id)| (id.as_str(), *inicio + i as u64))
                .collect();
            let titulo = format!("renombrar {} elemento{}", ids.len(), plural(ids.len()));
            let (tocados, _) = en_lote(
                lib,
                ids,
                id_cmd,
                aviso,
                vista,
                &titulo,
                Toca::nombre(),
                |it| {
                    let n = orden.get(it.id.as_str()).copied().unwrap_or(*inicio);
                    Parche::nombre(&grimorio_core::patron::aplicar(
                        patron,
                        &it.name,
                        n,
                        it.imported_at_ms(),
                    ))
                },
            )?;
            eventos::hecho(
                aviso,
                id_cmd,
                format!("{tocados} renombrado{}", plural(tocados)),
            );
        }

        Comando::QuitarCopiasExactas => {
            let grupos = lib.index().duplicados(0)?.grupos;
            let mut sobran: Vec<String> = Vec::new();
            for g in grupos.iter().filter(|g| g.exacto) {
                // La que se queda es la más antigua: la que estaba primero.
                let mut con_fecha: Vec<(u64, &String)> = g
                    .ids
                    .iter()
                    .map(|id| {
                        let t = lib.load_item(id).map(|it| it.imported_at_ms()).unwrap_or(u64::MAX);
                        (t, id)
                    })
                    .collect();
                con_fecha.sort();
                sobran.extend(con_fecha.iter().skip(1).map(|(_, id)| (*id).clone()));
            }
            if sobran.is_empty() {
                eventos::hecho(aviso, id_cmd, "no hay copias exactas".to_string());
                return Ok(());
            }
            let titulo = format!("quitar {} copia{} exacta{}", sobran.len(), plural(sobran.len()), plural(sobran.len()));
            let (tocados, _) = en_lote(
                lib,
                &sobran,
                id_cmd,
                aviso,
                vista,
                &titulo,
                Toca::papelera(),
                |_| Parche::papelera(true),
            )?;
            eventos::carpetas(aviso, lib);
            eventos::hecho(
                aviso,
                id_cmd,
                format!("{tocados} copia{} a la papelera (se puede deshacer)", plural(tocados)),
            );
        }

        Comando::Papelera { ids, dentro } => {
            let titulo = if *dentro {
                "mandar a la papelera"
            } else {
                "sacar de la papelera"
            };
            // `en_lote` ya rehace la vista aunque sea un solo elemento: tirar
            // algo lo saca de lo que se está mirando, y dejarlo en la malla
            // sería enseñar algo que ya no está ahí.
            let (tocados, _) = en_lote(
                lib,
                ids,
                id_cmd,
                aviso,
                vista,
                titulo,
                Toca::papelera(),
                |_| Parche::papelera(*dentro),
            )?;
            eventos::carpetas(aviso, lib);
            eventos::hecho(
                aviso,
                id_cmd,
                if *dentro {
                    format!("{tocados} a la papelera")
                } else {
                    format!("{tocados} recuperado{}", plural(tocados))
                },
            );
        }

        Comando::Adulto { ids, si } => {
            let titulo = if *si {
                "marcar como +18"
            } else {
                "quitar la marca de +18"
            };
            let (tocados, _) = en_lote(
                lib,
                ids,
                id_cmd,
                aviso,
                vista,
                titulo,
                Toca::adulto(),
                |_| Parche::adulto(*si),
            )?;
            // El recuento de +18 viaja con las carpetas, y de él cuelga el
            // botón del modo seguro. Sin esto, marcar el primero no lo hacía
            // aparecer y desmarcar el último no lo quitaba hasta reiniciar.
            eventos::carpetas(aviso, lib);
            eventos::hecho(
                aviso,
                id_cmd,
                if *si {
                    format!("{tocados} marcado{}", plural(tocados))
                } else {
                    format!("{tocados} sin marcar")
                },
            );
        }

        Comando::VaciarPapelera => {
            let a = Mutex::new(*aviso);
            let n = lib.vaciar_papelera(&|hechos, total| {
                if let Ok(av) = a.lock() {
                    eventos::progreso(&av, id_cmd, "vaciar la papelera", hechos, total);
                }
            })?;
            let q = consulta_actual(vista);
            republicar(lib, q, id_cmd, aviso, vista)?;
            eventos::carpetas(aviso, lib);
            eventos::registro(aviso, lib);
            eventos::hecho(
                aviso,
                id_cmd,
                format!(
                    "{n} elemento{} borrado{} para siempre",
                    plural(n),
                    plural(n)
                ),
            );
        }

        Comando::Deshacer | Comando::Rehacer => {
            let atras = matches!(cmd, Comando::Deshacer);
            let a = Mutex::new(*aviso);
            let que = if atras { "deshacer" } else { "rehacer" };
            let progreso = |hechos: usize, total: usize| {
                if let Ok(av) = a.lock() {
                    eventos::progreso(&av, id_cmd, que, hechos, total);
                }
            };
            let titulo = if atras {
                lib.deshacer(&progreso)?
            } else {
                lib.rehacer(&progreso)?
            };
            match titulo {
                Some(op) => {
                    // Se aplica la misma regla que en cualquier otro cambio, y
                    // por el mismo motivo: deshacer una estrella en una
                    // biblioteca de cien mil no puede costar una consulta
                    // entera solo porque la haya pedido el menú de deshacer.
                    let parches = if atras { &op.antes } else { &op.despues };
                    let toca = Toca::de(parches);
                    let arbol = op.carpetas_antes.is_some();
                    let q = consulta_actual(vista);
                    if arbol || parches.len() > LOTE_MENUDO || cambia_la_vista(&q, toca) {
                        republicar(lib, q, id_cmd, aviso, vista)?;
                    } else {
                        for (id, _) in parches {
                            if let Ok(item) = lib.load_item(id) {
                                eventos::item(aviso, id_cmd, &item);
                            }
                        }
                    }
                    if arbol || toca.carpetas || toca.adulto || toca.papelera {
                        eventos::carpetas(aviso, lib);
                    }
                    if toca.etiquetas {
                        eventos::etiquetas(
                            aviso,
                            id_cmd,
                            lib.index().tags_con_prefijo("", tope_etiquetas())?,
                        );
                    }
                    eventos::hecho(
                        aviso,
                        id_cmd,
                        format!(
                            "{}: {}",
                            if atras { "deshecho" } else { "rehecho" },
                            op.titulo
                        ),
                    );
                }
                None => eventos::hecho(aviso, id_cmd, format!("no hay nada que {que}")),
            }
            eventos::registro(aviso, lib);
        }

        Comando::Etiquetas { prefijo, tope } => {
            let lista = lib
                .index()
                .tags_con_prefijo(prefijo, (*tope).clamp(1, 500))?;
            eventos::etiquetas(aviso, id_cmd, lista);
        }

        Comando::Importar {
            rutas,
            carpeta,
            etiquetas,
            origen,
            mover,
            copiar,
        } => {
            if let Some(c) = carpeta {
                if !lib.folders().existe(c) {
                    return Err(Error::Invalid(format!("no existe la carpeta {c}")));
                }
            }
            let fuentes: Vec<PathBuf> = rutas.iter().map(PathBuf::from).collect();
            let opts = ImportOptions {
                mode: if *mover {
                    grimorio_core::item::OriginMode::Move
                } else if *copiar {
                    grimorio_core::item::OriginMode::Copy
                } else {
                    lib.meta().import_mode
                },
                folder: carpeta.clone(),
                tags: etiquetas.clone(),
                source: origen.clone(),
                ..Default::default()
            };
            // El progreso lo llaman los hilos de importación, varios a la vez.
            // El cerrojo no está por rendimiento —son unos pocos avisos por
            // segundo— sino para poder prometer algo concreto al otro lado: el
            // callback puede llegar desde cualquier hilo, pero nunca desde dos
            // a la vez.
            let a = Mutex::new(*aviso);
            let informe = import::import(lib, &fuentes, &opts, &move |hechos, total| {
                if let Ok(av) = a.lock() {
                    eventos::progreso(&av, id_cmd, "importar", hechos, total);
                }
            })?;
            // Lo contrario de importar es mandar lo importado a la papelera, no
            // borrarlo: deshacer nunca debería poder perder un archivo.
            if !informe.ids.is_empty() {
                let mut op = Operacion::nueva(format!(
                    "importar {} elemento{}",
                    informe.ids.len(),
                    plural(informe.ids.len())
                ));
                for id in &informe.ids {
                    op.antes.push((id.clone(), Parche::papelera(true)));
                    op.despues.push((id.clone(), Parche::papelera(false)));
                }
                lib.anotar(op);
            }
            let q = consulta_actual(vista);
            republicar(lib, q, id_cmd, aviso, vista)?;
            eventos::carpetas(aviso, lib);
            eventos::etiquetas(
                aviso,
                id_cmd,
                lib.index().tags_con_prefijo("", tope_etiquetas())?,
            );
            eventos::registro(aviso, lib);
            eventos::hecho(aviso, id_cmd, resumen_import(&informe));
        }

        Comando::Ficha { id } => {
            let it = lib.load_item(id)?;
            eventos::ficha(aviso, id_cmd, lib, &it);
        }

        Comando::AbrirFuera { id } => {
            let item = lib.load_item(id)?;
            let ruta = lib.original_path(&item);
            if !ruta.exists() {
                return Err(Error::Invalid(format!(
                    "el archivo original no está en {}",
                    ruta.display()
                )));
            }
            grimorio_core::externo::abrir_fuera(&ruta).map_err(|e| {
                Error::Invalid(format!("no pude abrirlo con el programa del sistema: {e}"))
            })?;
            eventos::hecho(aviso, id_cmd, String::new());
        }
    }
    Ok(())
}

/// Cuántos elementos caben en un cambio antes de dejar de avisar de uno en uno.
///
/// Por debajo de esto sale un evento por elemento con su estado nuevo, y la
/// interfaz lo pinta sin volver a consultar nada. Por encima, cien mil eventos
/// ahogarían el hilo de interfaz, así que se manda progreso y al final se
/// republica la vista de una vez: una consulta de ochenta y cinco milisegundos
/// es mucho para una estrella y muy poco para diez mil.
const LOTE_MENUDO: usize = 64;

/// Qué campos toca un cambio. Sirve para una sola decisión, pero es la que
/// separa una interfaz que responde de una que se piensa cada pulsación.
#[derive(Clone, Copy, Default)]
struct Toca {
    papelera: bool,
    adulto: bool,
    carpetas: bool,
    etiquetas: bool,
    estrellas: bool,
    nombre: bool,
    nota: bool,
    orden: bool,
}

impl Toca {
    fn papelera() -> Toca {
        Toca {
            papelera: true,
            ..Default::default()
        }
    }
    fn carpetas() -> Toca {
        Toca {
            carpetas: true,
            ..Default::default()
        }
    }
    fn etiquetas() -> Toca {
        Toca {
            etiquetas: true,
            ..Default::default()
        }
    }
    fn estrellas() -> Toca {
        Toca {
            estrellas: true,
            ..Default::default()
        }
    }
    fn nombre() -> Toca {
        Toca {
            nombre: true,
            ..Default::default()
        }
    }
    fn nota() -> Toca {
        Toca {
            nota: true,
            ..Default::default()
        }
    }
    fn adulto() -> Toca {
        Toca {
            adulto: true,
            ..Default::default()
        }
    }
    fn orden() -> Toca {
        Toca {
            orden: true,
            ..Default::default()
        }
    }

    /// Lo que de verdad cambió, leído de los parches. Lo usa el deshacer, que
    /// no sabe de antemano qué operación le va a tocar revertir.
    fn de(parches: &[(String, Parche)]) -> Toca {
        let mut t = Toca::default();
        for (_, p) in parches {
            t.papelera |= p.papelera.is_some();
            t.carpetas |= p.carpetas.is_some();
            t.etiquetas |= p.etiquetas.is_some();
            t.estrellas |= p.estrellas.is_some();
            t.nombre |= p.nombre.is_some();
            t.nota |= p.nota.is_some();
            t.adulto |= p.adulto.is_some();
            t.orden |= p.orden.is_some();
        }
        t
    }
}

/// ¿Este cambio puede sacar un elemento de lo que se está mirando?
///
/// Si la respuesta es sí hay que rehacer la vista aunque sean tres elementos:
/// dejar en la malla algo que ya no cumple el filtro es enseñar una mentira, y
/// se nota enseguida —quitas la cuarta estrella dentro de «estrellas:>=4» y la
/// foto sigue ahí—. Si es no, basta con avisar de esos elementos, que con cien
/// mil en la vista cuesta mil veces menos que volver a consultar.
fn cambia_la_vista(q: &Query, t: Toca) -> bool {
    // Tirar algo saca de todas las vistas menos de la papelera, y sacarlo de la
    // papelera lo saca de esa. En los dos casos hay que rehacer.
    t.papelera
        || (t.carpetas && q.folder.is_some())
        || (t.etiquetas && (!q.tags.is_empty() || q.text.is_some()))
        || (t.estrellas
            && (q.min_stars.is_some() || q.max_stars.is_some() || q.sort == SortBy::StarsDesc))
        || (t.nombre && (q.text.is_some() || matches!(q.sort, SortBy::NameAsc | SortBy::NameDesc)))
        || (t.nota && q.text.is_some())
        // Marcar como +18 solo saca de la vista si se está mirando justo por
        // esa marca. Con el modo seguro puesto no cambia qué se ve, solo cómo
        // se ve, y eso lo resuelve la celda sola.
        || (t.adulto && q.adulto.is_some())
        // Cambiar el orden a mano solo mueve algo si se mira por ese orden.
        || (t.orden && q.sort == SortBy::Manual)
}

/// Aplica un cambio a varios elementos y avisa de la forma que corresponda.
///
/// El trabajo lo hace el motor del núcleo, que guarda por tandas y mete cada
/// tanda en el índice de una sola vez; aquí solo se decide **cómo se cuenta**:
/// uno a uno con su estado nuevo si son pocos, o progreso y una vista nueva al
/// final si son muchos.
///
/// El título es el que verá la persona en el menú de deshacer, así que se
/// escribe en cristiano —«poner 4 estrellas»— y sirve además de etiqueta para
/// la barra de progreso.
///
/// Devuelve cuántos cambiaron y cuántos no se pudieron leer.
fn en_lote(
    lib: &mut Library,
    ids: &[String],
    id_cmd: u64,
    aviso: &Aviso,
    vista: &Arc<Mutex<Arc<Vista>>>,
    titulo: &str,
    toca: Toca,
    que: impl Fn(&Item) -> Parche + Sync,
) -> Result<(usize, usize)> {
    let menudo = ids.len() <= LOTE_MENUDO;
    // El mismo cerrojo que en la importación, y por el mismo motivo: el aviso
    // lleva un puntero opaco y el motor pide un cierre que se pueda compartir
    // entre hilos. No está por rendimiento, está para poder prometer que dos
    // avisos nunca se solapan.
    let a = Mutex::new(*aviso);
    let (tocados, ilegibles) = lib.editar(ids, titulo, que, &|hechos, total| {
        if !menudo {
            if let Ok(av) = a.lock() {
                eventos::progreso(&av, id_cmd, titulo, hechos, total);
            }
        }
    })?;

    let q = consulta_actual(vista);
    if menudo && !cambia_la_vista(&q, toca) {
        // Releer sesenta y cuatro `item.json` cuesta menos de un milisegundo y
        // evita tener que sacar los elementos del motor solo para esto.
        for id in ids {
            if let Ok(item) = lib.load_item(id) {
                eventos::item(aviso, id_cmd, &item);
            }
        }
    } else {
        republicar(lib, q, id_cmd, aviso, vista)?;
    }
    eventos::busquedas(aviso, lib);
    eventos::registro(aviso, lib);
    if ilegibles > 0 {
        eventos::error(
            aviso,
            id_cmd,
            &format!("{ilegibles} elemento{} no se pudo leer", plural(ilegibles)),
        );
    }
    Ok((tocados, ilegibles))
}

/// Rehace la vista y la publica. Es lo único que sustituye la vista viva.
fn republicar(
    lib: &mut Library,
    q: Query,
    id_cmd: u64,
    aviso: &Aviso,
    vista: &Arc<Mutex<Arc<Vista>>>,
) -> Result<()> {
    let pack = leer(vista).pack();
    let nueva = Arc::new(Vista::nueva(lib, &q, pack)?);
    let n = nueva.len();
    match vista.lock() {
        Ok(mut v) => *v = nueva,
        Err(envenenado) => *envenenado.into_inner() = nueva,
    }
    eventos::vista(aviso, id_cmd, &n);
    Ok(())
}

fn leer(vista: &Arc<Mutex<Arc<Vista>>>) -> Arc<Vista> {
    match vista.lock() {
        Ok(v) => Arc::clone(&v),
        Err(e) => Arc::clone(&e.into_inner()),
    }
}

fn consulta_actual(vista: &Arc<Mutex<Arc<Vista>>>) -> Query {
    leer(vista).consulta().clone()
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

fn resumen_import(r: &import::ImportReport) -> String {
    let mut partes = vec![format!("{} importado{}", r.imported, plural(r.imported))];
    if r.duplicates > 0 {
        partes.push(format!("{} repetido{}", r.duplicates, plural(r.duplicates)));
    }
    if r.unsupported > 0 {
        partes.push(format!("{} sin soporte", r.unsupported));
    }
    if r.sin_miniatura > 0 {
        partes.push(format!("{} sin miniatura", r.sin_miniatura));
    }
    if !r.failed.is_empty() {
        partes.push(format!("{} con error", r.failed.len()));
    }
    let mut linea = partes.join(", ");
    // El primer aviso va en el mismo mensaje: si faltan las herramientas para
    // ver los vídeos, eso es lo único que hay que leer.
    if let Some(a) = r.avisos.first() {
        linea.push_str(" · ");
        linea.push_str(a);
    }
    linea
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Comando {
        serde_json::from_str(s).expect("el comando debería entenderse")
    }

    #[test]
    fn los_comandos_se_leen_del_json_que_manda_qml() {
        match parse(r#"{"cmd":"estrellas","ids":["a","b"],"valor":4}"#) {
            Comando::Estrellas { ids, valor } => {
                assert_eq!(ids.len(), 2);
                assert_eq!(valor, 4);
            }
            otro => panic!("salió {otro:?}"),
        }
        match parse(r#"{"cmd":"crear_carpeta","nombre":"Rótulos"}"#) {
            Comando::CrearCarpeta { nombre, padre } => {
                assert_eq!(nombre, "Rótulos");
                assert!(padre.is_none(), "sin padre es una carpeta de la raíz");
            }
            otro => panic!("salió {otro:?}"),
        }
    }

    #[test]
    fn una_consulta_sin_campos_es_la_consulta_por_defecto() {
        match parse(r#"{"cmd":"consulta"}"#) {
            Comando::Consulta { q, .. } => {
                assert_eq!(q.limit, 200);
                assert!(q.folder_recursive, "por defecto la carpeta arrastra hijas");
            }
            otro => panic!("salió {otro:?}"),
        }
    }

    #[test]
    fn un_comando_que_no_existe_no_se_inventa_nada() {
        let r: std::result::Result<Comando, _> =
            serde_json::from_str(r#"{"cmd":"formatear_disco"}"#);
        assert!(r.is_err());
        let r: std::result::Result<Comando, _> = serde_json::from_str(r#"{"cmd":"estrellas"}"#);
        assert!(r.is_err(), "faltan campos obligatorios");
    }

    #[test]
    fn el_resumen_de_importacion_habla_en_singular_cuando_toca() {
        let mut r = import::ImportReport {
            imported: 1,
            ..Default::default()
        };
        assert_eq!(resumen_import(&r), "1 importado");
        r.imported = 3;
        r.duplicates = 1;
        assert_eq!(resumen_import(&r), "3 importados, 1 repetido");
    }
}
