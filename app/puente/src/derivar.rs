//! El camino de los derivados: lo que el visor pide para enseñar un elemento.
//!
//! No va por el bus de comandos a propósito. El bus tiene un solo hilo y es el
//! de editar la biblioteca: si convertir un ProRes de diez minutos pasara por
//! él, ponerle una estrella a otra foto esperaría a que terminara. Los
//! derivados no tocan la biblioteca abierta —reciben rutas y devuelven rutas—,
//! así que el lado de Qt los pide desde su propio reparto de hilos.
//!
//! Petición y respuesta en JSON, como el bus, para que añadir uno no cambie la
//! frontera C.

use grimorio_core::derivado;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(tag = "que", rename_all = "snake_case")]
pub enum Peticion {
    /// Copia en H.264 de un vídeo que el reproductor no sabe abrir.
    Proxy {
        raiz: PathBuf,
        id: String,
        ruta: PathBuf,
    },
    /// Fotogramas en rejilla para la vista previa de la barra.
    Tira {
        raiz: PathBuf,
        id: String,
        ruta: PathBuf,
        duracion: f32,
    },
    /// Onda y espectro de un sonido.
    Onda {
        raiz: PathBuf,
        id: String,
        ruta: PathBuf,
    },
    /// Tamaño de cada página de un PDF, en puntos.
    PdfMedidas { ruta: PathBuf },
    /// Una página de un PDF dibujada a un ancho.
    PdfPagina {
        raiz: PathBuf,
        id: String,
        ruta: PathBuf,
        pagina: u32,
        ancho: u32,
    },
    /// La malla de un modelo 3D para el visor.
    Malla {
        raiz: PathBuf,
        id: String,
        ruta: PathBuf,
        ext: String,
    },
}

/// El derivado se guarda con el `id` y se saca de la `ruta`, y nada obligaba a
/// que fueran del mismo elemento. Cuando no lo eran, la copia de un vídeo
/// acababa con el contenido de otro y se quedaba así en la caché: dos vídeos
/// distintos que se reproducían igual. Si el archivo es de dentro de la
/// biblioteca, su carpeta lleva el id, y eso se puede comprobar. Lo de fuera
/// (importado sin copiar) no lo lleva y se deja pasar.
fn mismo_elemento(raiz: &Path, id: &str, ruta: &Path) -> Result<(), String> {
    if !ruta.starts_with(raiz.join("items")) {
        return Ok(());
    }
    let carpeta = ruta.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str());
    if carpeta == Some(id) {
        Ok(())
    } else {
        Err(format!(
            "el archivo {} no es del elemento {id}: no se guarda un derivado cruzado",
            ruta.display()
        ))
    }
}

/// Atiende una petición. Siempre devuelve un JSON con `ok`: un fallo es una
/// respuesta, no un pánico que cruce la frontera C.
pub fn responder(peticion: &[u8]) -> Value {
    let p: Peticion = match serde_json::from_slice(peticion) {
        Ok(p) => p,
        Err(e) => return json!({ "ok": false, "error": format!("petición ilegible: {e}") }),
    };
    let cruce = match &p {
        Peticion::Proxy { raiz, id, ruta }
        | Peticion::Tira { raiz, id, ruta, .. }
        | Peticion::Onda { raiz, id, ruta }
        | Peticion::PdfPagina { raiz, id, ruta, .. }
        | Peticion::Malla { raiz, id, ruta, .. } => mismo_elemento(raiz, id, ruta),
        Peticion::PdfMedidas { .. } => Ok(()),
    };
    if let Err(e) = cruce {
        return json!({ "ok": false, "error": e });
    }
    let r = match p {
        Peticion::Proxy { raiz, id, ruta } => {
            derivado::proxy_video(&raiz, &id, &ruta).map(|r| json!({ "ruta": r }))
        }
        Peticion::Tira {
            raiz,
            id,
            ruta,
            duracion,
        } => derivado::tira_video(&raiz, &id, &ruta, duracion).map(|(r, t)| {
            json!({
                "ruta": r,
                "columnas": t.columnas,
                "filas": t.filas,
                "n": t.n,
                "cadaS": t.cada_s,
            })
        }),
        Peticion::Onda { raiz, id, ruta } => {
            derivado::onda(&raiz, &id, &ruta).map(|r| json!({ "ruta": r }))
        }
        Peticion::PdfMedidas { ruta } => {
            derivado::medidas_pdf(&ruta).map(|m| json!({ "paginas": m }))
        }
        Peticion::PdfPagina {
            raiz,
            id,
            ruta,
            pagina,
            ancho,
        } => derivado::pagina_pdf(&raiz, &id, &ruta, pagina, ancho).map(|r| json!({ "ruta": r })),
        Peticion::Malla {
            raiz,
            id,
            ruta,
            ext,
        } => derivado::malla(&raiz, &id, &ruta, &ext).map(|(r, info)| {
            json!({
                "ruta": r,
                "triangulos": info.triangulos,
                "medidas": info.medidas_mm,
            })
        }),
    };
    match r {
        Ok(mut v) => {
            v["ok"] = json!(true);
            v
        }
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_guarda_el_derivado_de_un_elemento_con_el_archivo_de_otro() {
        // El caso que pasó: el id de un vídeo y el archivo de otro.
        let p = json!({
            "que": "proxy",
            "raiz": "/lib",
            "id": "01CRUCES",
            "ruta": "/lib/items/01/CU/01CUBOS/original.mov",
        });
        let r = responder(p.to_string().as_bytes());
        assert_eq!(r["ok"], false);
        assert!(r["error"].as_str().unwrap().contains("no es del elemento"));

        // Y con lo de fuera de la biblioteca no se mete: no hay id que mirar.
        assert!(mismo_elemento(
            Path::new("/lib"),
            "01CRUCES",
            Path::new("/otra/parte/video.mov")
        )
        .is_ok());
        assert!(mismo_elemento(
            Path::new("/lib"),
            "01CRUCES",
            Path::new("/lib/items/01/CR/01CRUCES/original.mov")
        )
        .is_ok());
    }

    #[test]
    fn una_peticion_rota_contesta_con_un_error() {
        let r = responder(b"{\"que\":\"nada\"}");
        assert_eq!(r["ok"], false);
        assert!(r["error"].as_str().unwrap().contains("ilegible"));
    }

    #[test]
    fn la_malla_de_un_stl_llega_con_sus_datos() {
        let dir = tempfile::tempdir().unwrap();
        let stl = dir.path().join("t.stl");
        // Un triángulo en STL de texto: 10 x 0 x 5 mm.
        std::fs::write(
            &stl,
            "solid t\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 10 0 0\nvertex 0 0 5\nendloop\nendfacet\nendsolid t\n",
        )
        .unwrap();
        let p = json!({
            "que": "malla", "raiz": dir.path(), "id": "01TEST", "ruta": stl, "ext": "stl"
        });
        let r = responder(p.to_string().as_bytes());
        assert_eq!(r["ok"], true, "{r}");
        assert_eq!(r["triangulos"], 1);
        assert_eq!(r["medidas"][0], 10.0);
        assert!(PathBuf::from(r["ruta"].as_str().unwrap()).exists());
    }

    #[test]
    fn un_archivo_que_no_existe_es_un_error_legible() {
        let p = json!({ "que": "malla", "raiz": "/tmp", "id": "01X", "ruta": "/no/existe.stl", "ext": "stl" });
        let r = responder(p.to_string().as_bytes());
        assert_eq!(r["ok"], false);
        assert!(r["error"].as_str().unwrap().contains("/no/existe.stl"));
    }
}
