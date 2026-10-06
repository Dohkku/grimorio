//! Núcleo de Grimorio.
//!
//! Reglas de esta capa:
//!   * No depende de ningún toolkit gráfico. Compila y se testea sin escritorio.
//!   * Los `item.json` en disco son la verdad; `index.sqlite` es un índice derivado
//!     y reconstruible (ver [`index::rebuild_from_disk`]).
//!   * Nada de aquí bloquea un hilo de UI: la API es síncrona y el llamante decide
//!     en qué hilo la ejecuta.

pub mod audio;
pub mod busquedas;
pub mod derivado;
pub mod diseno;
pub mod error;
pub mod externo;
pub mod filtro;
pub mod grupos;
pub mod folder;
pub mod id;
pub mod image_ops;
pub mod import;
pub mod index;
pub mod item;
pub mod library;
pub mod medio;
pub mod modelo3d;
pub mod patron;
pub mod query;
pub mod raw;
pub mod registro;
pub mod senal;
pub mod thumbs;
pub mod time;
pub mod vigiladas;

pub use error::{Error, Result};
pub use folder::{Folder, Folders};
pub use item::{Item, Origin, OriginMode, PaletteEntry};
pub use library::{preview_path, Library};
pub use medio::{Familia, Lienzo};
pub use query::{Query, QueryHit, SortBy};
pub use registro::{Operacion, Parche, Registro};

/// Versión del esquema de `item.json`. Se sube solo con migración escrita.
pub const SCHEMA_VERSION: u32 = 1;

/// Lado mayor de la miniatura de malla, en píxeles.
pub const THUMB_GRID: u32 = 320;
/// Lado mayor de la previsualización.
pub const THUMB_PREVIEW: u32 = 1024;
