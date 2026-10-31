use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{que} {path}: {source}", que = crate::idioma::t("no se pudo leer o escribir", "could not read or write"))]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("{que}: {0}", que = crate::idioma::t("error de entrada/salida", "input/output error"))]
    RawIo(#[from] std::io::Error),

    #[error("{que}: {0}", que = crate::idioma::t("el índice devolvió un error", "the index returned an error"))]
    Sqlite(#[from] rusqlite::Error),

    #[error("{que} {path}: {source}", que = crate::idioma::t("JSON inválido en", "invalid JSON in"))]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("{que}: {0}", que = crate::idioma::t("no se pudo decodificar la imagen", "could not decode the image"))]
    Image(#[from] image::ImageError),

    #[error("{0} {que}", que = crate::idioma::t("no es una biblioteca de Grimorio", "is not a Grimorio library"))]
    NotALibrary(PathBuf),

    #[error("{0} {que}", que = crate::idioma::t("ya existe", "already exists"))]
    AlreadyExists(PathBuf),

    #[error("{que}", que = esquema_nuevo(*found, *supported))]
    SchemaTooNew { found: u32, supported: u32 },

    #[error("{0}")]
    Invalid(String),
}

fn esquema_nuevo(found: u32, supported: u32) -> String {
    if crate::idioma::ingles() {
        format!("schema version {found} not supported (this build understands up to {supported})")
    } else {
        format!("versión de esquema {found} no soportada (esta build entiende hasta {supported})")
    }
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
