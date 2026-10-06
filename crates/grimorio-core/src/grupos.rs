//! Grupos de etiquetas, con su color.
//!
//! Las etiquetas viven en cada elemento (`item.json`); los grupos no son de
//! ningún elemento sino de la biblioteca —«paleta», «técnica», «cliente»—, así
//! que van aparte, en `etiquetas.json`. Una etiqueta está en un grupo o en
//! ninguno, nunca en dos: el color de una etiqueta es el de su grupo, y con dos
//! grupos no habría un color que darle.
//!
//! Renombrar o borrar una etiqueta toca los elementos (eso va por el motor de
//! edición, con deshacer) y además su sitio aquí, que es lo que hacen
//! `renombrar_etiqueta` y `quitar_etiqueta`.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const ARCHIVO: &str = "etiquetas.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Grupo {
    pub id: String,
    pub nombre: String,
    /// `#rrggbb`.
    pub color: String,
    pub etiquetas: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Archivo {
    schema: u32,
    grupos: Vec<Grupo>,
}

pub fn cargar(raiz: &Path) -> Result<Vec<Grupo>> {
    let ruta = raiz.join(ARCHIVO);
    if !ruta.exists() {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(&ruta).map_err(|e| Error::io(&ruta, e))?;
    let a: Archivo = serde_json::from_slice(&bytes).map_err(|e| Error::json(&ruta, e))?;
    Ok(a.grupos)
}

fn guardar(raiz: &Path, grupos: &[Grupo]) -> Result<()> {
    let ruta = raiz.join(ARCHIVO);
    let a = Archivo {
        schema: 1,
        grupos: grupos.to_vec(),
    };
    let json = serde_json::to_vec_pretty(&a).map_err(|e| Error::json(&ruta, e))?;
    crate::item::write_atomic(&ruta, &json)
}

fn color_valido(c: &str) -> bool {
    c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|x| x.is_ascii_hexdigit())
}

fn editar<T>(raiz: &Path, id: &str, f: impl FnOnce(&mut Grupo) -> T) -> Result<T> {
    let mut todos = cargar(raiz)?;
    let Some(g) = todos.iter_mut().find(|g| g.id == id) else {
        return Err(Error::Invalid(format!("no existe el grupo {id}")));
    };
    let r = f(g);
    guardar(raiz, &todos)?;
    Ok(r)
}

pub fn crear(raiz: &Path, nombre: &str, color: &str) -> Result<String> {
    let nombre = nombre.trim();
    if nombre.is_empty() {
        return Err(Error::Invalid("un grupo necesita nombre".into()));
    }
    if !color_valido(color) {
        return Err(Error::Invalid(format!("color no válido: {color}")));
    }
    let mut todos = cargar(raiz)?;
    let id = crate::id::new_id();
    todos.push(Grupo {
        id: id.clone(),
        nombre: nombre.to_string(),
        color: color.to_lowercase(),
        etiquetas: Vec::new(),
    });
    guardar(raiz, &todos)?;
    Ok(id)
}

pub fn renombrar(raiz: &Path, id: &str, nombre: &str) -> Result<()> {
    let nombre = nombre.trim();
    if nombre.is_empty() {
        return Err(Error::Invalid("el nombre no puede quedar vacío".into()));
    }
    editar(raiz, id, |g| g.nombre = nombre.to_string())
}

pub fn colorear(raiz: &Path, id: &str, color: &str) -> Result<()> {
    if !color_valido(color) {
        return Err(Error::Invalid(format!("color no válido: {color}")));
    }
    editar(raiz, id, |g| g.color = color.to_lowercase())
}

/// Borra el grupo. Sus etiquetas no se tocan: se quedan sin grupo.
pub fn borrar(raiz: &Path, id: &str) -> Result<()> {
    let mut todos = cargar(raiz)?;
    let antes = todos.len();
    todos.retain(|g| g.id != id);
    if todos.len() == antes {
        return Err(Error::Invalid(format!("no existe el grupo {id}")));
    }
    guardar(raiz, &todos)
}

/// Mete una etiqueta en un grupo (sacándola del que estuviera), o la deja sin
/// grupo con `None`.
pub fn agrupar(raiz: &Path, etiqueta: &str, grupo: Option<&str>) -> Result<()> {
    let mut todos = cargar(raiz)?;
    if let Some(id) = grupo {
        if !todos.iter().any(|g| g.id == id) {
            return Err(Error::Invalid(format!("no existe el grupo {id}")));
        }
    }
    for g in todos.iter_mut() {
        g.etiquetas.retain(|t| t != etiqueta);
        if Some(g.id.as_str()) == grupo {
            g.etiquetas.push(etiqueta.to_string());
        }
    }
    guardar(raiz, &todos)
}

/// Una etiqueta ha cambiado de nombre: su sitio va con ella. Si la nueva ya
/// tenía grupo, manda el de la nueva —es una fusión, y la que se queda es
/// esa—.
pub fn renombrar_etiqueta(raiz: &Path, vieja: &str, nueva: &str) -> Result<()> {
    let mut todos = cargar(raiz)?;
    let nueva_tiene = todos.iter().any(|g| g.etiquetas.iter().any(|t| t == nueva));
    let mut cambio = false;
    for g in todos.iter_mut() {
        if let Some(p) = g.etiquetas.iter().position(|t| t == vieja) {
            if nueva_tiene {
                g.etiquetas.remove(p);
            } else {
                g.etiquetas[p] = nueva.to_string();
            }
            cambio = true;
        }
    }
    if cambio {
        guardar(raiz, &todos)?;
    }
    Ok(())
}

pub fn quitar_etiqueta(raiz: &Path, etiqueta: &str) -> Result<()> {
    let mut todos = cargar(raiz)?;
    let mut cambio = false;
    for g in todos.iter_mut() {
        let antes = g.etiquetas.len();
        g.etiquetas.retain(|t| t != etiqueta);
        cambio |= g.etiquetas.len() != antes;
    }
    if cambio {
        guardar(raiz, &todos)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn una_etiqueta_esta_en_un_grupo_como_mucho() {
        let d = tempfile::tempdir().unwrap();
        let paleta = crear(d.path(), "paleta", "#E05AA8").unwrap();
        let tecnica = crear(d.path(), "técnica", "#2f6fd6").unwrap();
        agrupar(d.path(), "rosa", Some(&paleta)).unwrap();
        agrupar(d.path(), "rosa", Some(&tecnica)).unwrap();
        let g = cargar(d.path()).unwrap();
        assert!(g[0].etiquetas.is_empty());
        assert_eq!(g[1].etiquetas, vec!["rosa"]);
        assert_eq!(g[0].color, "#e05aa8", "el color se guarda en minúsculas");
        agrupar(d.path(), "rosa", None).unwrap();
        assert!(cargar(d.path()).unwrap().iter().all(|g| g.etiquetas.is_empty()));
    }

    #[test]
    fn renombrar_lleva_su_sitio_y_al_fusionar_manda_la_que_queda() {
        let d = tempfile::tempdir().unwrap();
        let a = crear(d.path(), "a", "#111111").unwrap();
        let b = crear(d.path(), "b", "#222222").unwrap();
        agrupar(d.path(), "gato", Some(&a)).unwrap();
        renombrar_etiqueta(d.path(), "gato", "felino").unwrap();
        assert_eq!(cargar(d.path()).unwrap()[0].etiquetas, vec!["felino"]);

        agrupar(d.path(), "minino", Some(&a)).unwrap();
        agrupar(d.path(), "michi", Some(&b)).unwrap();
        renombrar_etiqueta(d.path(), "minino", "michi").unwrap();
        let g = cargar(d.path()).unwrap();
        assert_eq!(g[0].etiquetas, vec!["felino"]);
        assert_eq!(g[1].etiquetas, vec!["michi"]);

        quitar_etiqueta(d.path(), "michi").unwrap();
        assert!(cargar(d.path()).unwrap()[1].etiquetas.is_empty());
    }

    #[test]
    fn colores_y_nombres_raros_se_rechazan() {
        let d = tempfile::tempdir().unwrap();
        assert!(crear(d.path(), "x", "rojo").is_err());
        assert!(crear(d.path(), "  ", "#ffffff").is_err());
        let g = crear(d.path(), "x", "#ffffff").unwrap();
        assert!(colorear(d.path(), &g, "#12345").is_err());
        borrar(d.path(), &g).unwrap();
        assert!(borrar(d.path(), &g).is_err());
    }
}
