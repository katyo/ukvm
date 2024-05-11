/// Unified result type
pub type Result<T> = std::result::Result<T, Error>;

/// Unified error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Address error: {0}")]
    Addr(#[from] std::net::AddrParseError),
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] core::str::Utf8Error),
    #[error("Integer error: {0}")]
    Int(#[from] core::num::ParseIntError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[cfg(feature = "postcard")]
    #[error("Postcard error: {0}")]
    Postcard(#[from] postcard::Error),
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
    #[cfg(feature = "zbus")]
    #[error("DBus error: {0}")]
    DBus(#[from] zbus::Error),
    #[error("Other error: {0}")]
    Other(String),
}

impl AsRef<str> for Error {
    fn as_ref(&self) -> &str {
        match self {
            Error::Io(_) => "IO error",
            Error::Addr(_) => "Address error",
            Error::Utf8(_) => "UTF-8 error",
            Error::Int(_) => "Integer error",
            Error::Json(_) => "JSON error",
            #[cfg(feature = "postcard")]
            Error::Postcard(_) => "Postcard error",
            Error::Toml(_) => "TOML error",
            #[cfg(feature = "zbus")]
            Error::DBus(_) => "DBus error",
            Error::Other(_) => "Other error",
        }
    }
}

impl From<&str> for Error {
    fn from(error: &str) -> Self {
        Self::Other(error.into())
    }
}

impl From<String> for Error {
    fn from(error: String) -> Self {
        Self::Other(error)
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for Error {
    fn from(error: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::Other(error.to_string())
    }
}
