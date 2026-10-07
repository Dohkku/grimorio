//! Consultas. Una carpeta inteligente no es más que una de estas, guardada.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    Landscape,
    Portrait,
    Square,
}

impl Orientation {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "landscape" | "apaisada" | "h" => Some(Orientation::Landscape),
            "portrait" | "vertical" | "v" => Some(Orientation::Portrait),
            "square" | "cuadrada" | "s" => Some(Orientation::Square),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SortBy {
    ImportedDesc,
    ImportedAsc,
    NameAsc,
    NameDesc,
    SizeDesc,
    SizeAsc,
    StarsDesc,
    Random,
    /// El que se ha puesto arrastrando, por carpeta (`Item::orden`).
    Manual,
}

impl SortBy {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "recientes" | "imported-desc" | "nuevo" => Some(SortBy::ImportedDesc),
            "antiguos" | "imported-asc" => Some(SortBy::ImportedAsc),
            "nombre" | "name" | "name-asc" => Some(SortBy::NameAsc),
            "nombre-desc" | "name-desc" => Some(SortBy::NameDesc),
            "peso" | "size" | "size-desc" => Some(SortBy::SizeDesc),
            "size-asc" => Some(SortBy::SizeAsc),
            "estrellas" | "stars" => Some(SortBy::StarsDesc),
            "azar" | "random" => Some(SortBy::Random),
            "manual" | "mano" | "a-mano" | "propio" => Some(SortBy::Manual),
            _ => None,
        }
    }
}

/// `#[serde(default)]` a nivel de estructura: lo que falte sale de
/// `Query::default()`. La interfaz manda consultas parciales todo el rato
/// —solo la carpeta, solo el texto— y exigirle los campos completos convertía
/// cada filtro nuevo en un error de "falta el campo sort".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Query {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exts: Vec<String>,
    /// Familias de archivo: vídeo, tipografía, documento…
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub familias: Vec<crate::medio::Familia>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    /// Si la carpeta arrastra también lo que hay en sus hijas. Por defecto sí:
    /// es lo que espera quien viene de un explorador de archivos.
    pub folder_recursive: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_stars: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_size: Option<u64>,
    // Los topes por arriba existen para que el buscador pueda decir cosas como
    // «estrellas:0» o «peso:<500k». Sin ellos, la mitad del lenguaje de
    // filtros sería una promesa a medias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_stars: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orientation: Option<Orientation>,
    /// Color objetivo en sRGB. Ordena por cercanía perceptual.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<[u8; 3]>,
    /// Enseña la papelera en vez de la biblioteca. No es un filtro más: o una
    /// cosa o la otra, nunca las dos mezcladas.
    pub papelera: bool,
    /// Solo lo marcado como adulto, o solo lo que no lo está. Sin poner, todo.
    ///
    /// Es un filtro y no un permiso: el modo seguro no se apoya en esto. Sirve
    /// para encontrar lo marcado —y para repasar lo que falta por marcar—.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adulto: Option<bool>,
    /// Solo lo que tiene alguna etiqueta, o solo lo que no tiene ninguna. Es la
    /// vista «Sin etiquetar» de la barra lateral: lo que falta por ordenar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etiquetado: Option<bool>,
    /// Importado desde / hasta (ms Unix, los dos incluidos).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desde_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hasta_ms: Option<u64>,
    /// Solo lo que tiene repetido (exacto o casi), con cada grupo seguido.
    #[serde(default)]
    pub repetidos: bool,
    pub sort: SortBy,
    /// Cuántos resultados como mucho. **Cero significa todos**: la malla carga
    /// la biblioteca entera y pedirle que adivine un tope sería inventarse un
    /// número.
    pub limit: usize,
    pub offset: usize,
}

impl Default for Query {
    fn default() -> Self {
        Query {
            text: None,
            exts: Vec::new(),
            familias: Vec::new(),
            tags: Vec::new(),
            folder: None,
            folder_recursive: true,
            min_stars: None,
            min_width: None,
            min_height: None,
            min_size: None,
            max_stars: None,
            max_width: None,
            max_height: None,
            max_size: None,
            orientation: None,
            color: None,
            papelera: false,
            adulto: None,
            etiquetado: None,
            desde_ms: None,
            hasta_ms: None,
            repetidos: false,
            sort: SortBy::ImportedDesc,
            limit: 200,
            offset: 0,
        }
    }
}

/// Lo mínimo que la malla necesita para pintar una celda. Deliberadamente
/// pequeño: 100.000 de estos tienen que caber en memoria sin dolor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryHit {
    pub id: String,
    pub name: String,
    pub ext: String,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub stars: u8,
    pub thumb_off: Option<u64>,
    pub thumb_len: Option<u32>,
    pub dominant: Option<[u8; 3]>,
    pub imported_at_ms: u64,
    /// De qué familia es. Cabe en un byte y hace falta en cada celda: una malla
    /// que enseña un vídeo igual que una foto miente sobre lo que hay.
    pub kind: crate::medio::Familia,
    /// Segundos, si el archivo dura.
    pub duracion_s: Option<u32>,
    /// La ruta del archivo cuando vive fuera de la biblioteca.
    ///
    /// Solo la traen los importados en modo referencia, que son los únicos cuya
    /// ruta no se puede deducir del id y la extensión. Guardarla para todos
    /// serían cien mil cadenas repitiendo la misma carpeta.
    pub ruta_ref: Option<String>,
    /// Marcado como contenido adulto. Viaja en la vista porque lo mira cada
    /// celda al pintarse: preguntarlo elemento a elemento sería una consulta
    /// por celda y por fotograma.
    pub adulto: bool,
}

impl QueryHit {
    /// Distancia perceptual al color objetivo, ya en Oklab.
    pub fn color_distance(&self, target_lab: &[f32; 3]) -> f32 {
        self.color_distance2(target_lab).sqrt()
    }

    /// La misma distancia, sin la raíz cuadrada.
    ///
    /// Para ordenar sirve igual —la raíz es monótona— y se ahorra una operación
    /// por candidato. La conversión a Oklab, que es lo caro, se hace una sola
    /// vez por elemento; nunca dentro de un comparador.
    pub fn color_distance2(&self, target_lab: &[f32; 3]) -> f32 {
        match self.dominant {
            None => f32::MAX,
            Some(c) => {
                let lab = crate::image_ops::srgb_to_oklab(c[0], c[1], c[2]);
                let (x, y, z) = (
                    lab[0] - target_lab[0],
                    lab[1] - target_lab[1],
                    lab[2] - target_lab[2],
                );
                x * x + y * y + z * z
            }
        }
    }

    pub fn aspect(&self) -> f32 {
        if self.height == 0 {
            1.0
        } else {
            self.width as f32 / self.height as f32
        }
    }
}
