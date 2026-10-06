use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no se pudo leer o escribir {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("error de entrada/salida: {0}")]
    RawIo(#[from] std::io::Error),

    #[error("el índice devolvió un error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("JSON inválido en {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("no se pudo decodificar la imagen: {0}")]
    Image(#[from] image::ImageError),

    #[error("{0} no es una biblioteca de Grimorio")]
    NotALibrary(PathBuf),

    #[error("{0} ya existe")]
    AlreadyExists(PathBuf),

    #[error("versión de esquema {found} no soportada (esta build entiende hasta {supported})")]
    SchemaTooNew { found: u32, supported: u32 },

    #[error("{0}")]
    Invalid(String),
}

impl Error {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Error::Io {
            path: path.into(),
            source,
        }
    }

    pub fn json(path: impl Into<PathBuf>, source: serde_json::Error) -> Self {
        Error::Json {
            path: path.into(),
            source,
        }
    }
}
