//! Las carpetas inteligentes: búsquedas con nombre.
//!
//! Una carpeta inteligente **es** una línea del buscador guardada —
//! `tipo:video estrellas:>=4 fecha:30d`—, que es lo que el lenguaje de filtros
//! prometía desde el principio. No se guarda la consulta ya leída sino el
//! texto: así lo relativo (`fecha:7d`) se vuelve a calcular cada vez, y una
//! mejora del lenguaje mejora también las carpetas ya guardadas.
//!
//! Viven en `busquedas.json`, en la raíz de la biblioteca, y viajan con ella.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const ARCHIVO: &str = "busquedas.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Busqueda {
    pub id: String,
    pub nombre: String,
    /// La línea del buscador, tal cual.
    pub consulta: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Archivo {
    schema: u32,
    busquedas: Vec<Busqueda>,
}

/// Las guardadas. Sin archivo, ninguna: una biblioteca de antes no tiene.
pub fn cargar(raiz: &Path) -> Result<Vec<Busqueda>> {
    let ruta = raiz.join(ARCHIVO);
    if !ruta.exists() {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(&ruta).map_err(|e| Error::io(&ruta, e))?;
    let a: Archivo = serde_json::from_slice(&bytes).map_err(|e| Error::json(&ruta, e))?;
    Ok(a.busquedas)
}

pub fn guardar(raiz: &Path, busquedas: &[Busqueda]) -> Result<()> {
    let ruta = raiz.join(ARCHIVO);
    let a = Archivo {
        schema: 1,
        busquedas: busquedas.to_vec(),
    };
    let json = serde_json::to_vec_pretty(&a).map_err(|e| Error::json(&ruta, e))?;
    crate::item::write_atomic(&ruta, &json)
}

/// Añade una y devuelve su id.
pub fn anadir(raiz: &Path, nombre: &str, consulta: &str) -> Result<String> {
    let nombre = nombre.trim();
    let consulta = consulta.trim();
    if nombre.is_empty() || consulta.is_empty() {
        return Err(Error::Invalid(
            "una carpeta inteligente necesita nombre y búsqueda".into(),
        ));
    }
    let mut todas = cargar(raiz)?;
    let id = crate::id::new_id();
    todas.push(Busqueda {
        id: id.clone(),
        nombre: nombre.to_string(),
        consulta: consulta.to_string(),
    });
    guardar(raiz, &todas)?;
    Ok(id)
}

pub fn borrar(raiz: &Path, id: &str) -> Result<()> {
    let mut todas = cargar(raiz)?;
    let antes = todas.len();
    todas.retain(|b| b.id != id);
    if todas.len() == antes {
        return Err(Error::Invalid(format!("no existe la búsqueda {id}")));
    }
    guardar(raiz, &todas)
}

pub fn renombrar(raiz: &Path, id: &str, nombre: &str) -> Result<()> {
    let nombre = nombre.trim();
    if nombre.is_empty() {
        return Err(Error::Invalid("el nombre no puede quedar vacío".into()));
    }
    let mut todas = cargar(raiz)?;
    let Some(b) = todas.iter_mut().find(|b| b.id == id) else {
        return Err(Error::Invalid(format!("no existe la búsqueda {id}")));
    };
    b.nombre = nombre.to_string();
    guardar(raiz, &todas)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn se_guardan_se_renombran_y_se_borran() {
        let d = tempfile::tempdir().unwrap();
        assert!(cargar(d.path()).unwrap().is_empty());
        let a = anadir(d.path(), "vídeos buenos", "tipo:video estrellas:>=4").unwrap();
        let b = anadir(d.path(), "esta semana", "fecha:7d").unwrap();
        renombrar(d.path(), &a, "lo mejor en vídeo").unwrap();
        borrar(d.path(), &b).unwrap();
        let todas = cargar(d.path()).unwrap();
        assert_eq!(todas.len(), 1);
        assert_eq!(todas[0].nombre, "lo mejor en vídeo");
        assert_eq!(todas[0].consulta, "tipo:video estrellas:>=4");
        assert!(anadir(d.path(), "  ", "x").is_err());
        assert!(borrar(d.path(), "no-existe").is_err());
    }
}
