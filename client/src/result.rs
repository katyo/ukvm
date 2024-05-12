/// Unified result type
pub type Result<T> = std::result::Result<T, Error>;

/// Unified error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] core::str::Utf8Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[cfg(feature = "postcard")]
    #[error("Postcard error: {0}")]
    Postcard(#[from] postcard::Error),
    #[cfg(feature = "zbus")]
    #[error("DBus error: {0}")]
    DBus(#[from] zbus::Error),
    #[error("DBus FDO error: {0}")]
    DBusFdo(#[from] zbus::fdo::Error),
    #[error("Other error: {0}")]
    Other(String),
}

/*impl AsRef<str> for Error {
    fn as_ref(&self) -> &str {
        match self {
            Error::Io(_) => "IO error",
            Error::Utf8(_) => "UTF-8 error",
            Error::Json(_) => "JSON error",
            #[cfg(feature = "postcard")]
            Error::Postcard(_) => "Postcard error",
            #[cfg(feature = "zbus")]
            Error::DBus(_) => "DBus error",
            Error::Other(_) => "Other error",
        }
    }
}*/

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

#[cfg(feature = "zbus")]
impl From<Error> for zbus::fdo::Error {
    fn from(error: Error) -> Self {
        use zbus::fdo::Error::*;
        match error {
            Error::Io(e) => IOError(e.to_string()),
            Error::Utf8(e) => Failed(e.to_string()),
            Error::Json(e) => Failed(e.to_string()),
            #[cfg(feature = "postcard")]
            Error::Postcard(e) => Failed(e.to_string()),
            Error::DBus(e) => ZBus(e),
            Error::DBusFdo(e) => e,
            Error::Other(e) => Failed(e),
        }
    }
}

#[cfg(feature = "zbus")]
impl From<Error> for zbus::Error {
    fn from(error: Error) -> Self {
        use zbus::Error::*;
        match error {
            Error::Io(e) => InputOutput(std::sync::Arc::new(e)),
            Error::Utf8(e) => Failure(e.to_string()),
            Error::Json(e) => Failure(e.to_string()),
            #[cfg(feature = "postcard")]
            Error::Postcard(e) => Failure(e.to_string()),
            Error::DBus(e) => e,
            Error::DBusFdo(e) => FDO(Box::new(e)),
            Error::Other(e) => Failure(e),
        }
    }
}
