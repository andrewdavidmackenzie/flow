//! Error types for the `flowrclib` compiler library.

pub use flowcore::bail;
pub use flowcore::errors::ResultExt;
use thiserror::Error;

/// The error type for `flowrclib` operations.
#[derive(Debug, Error)]
pub enum Error {
    /// An I/O error
    #[error("{0}")]
    Io(#[from] std::io::Error),
    /// A URL parsing error
    #[error("{0}")]
    Url(#[from] url::ParseError),
    /// An error from the flowcore provider
    #[error("{0}")]
    Provider(#[from] flowcore::errors::Error),
    /// A glob pattern error
    #[error("{0}")]
    GlobWalk(#[from] globwalk::GlobError),
    /// A general error message
    #[error("{0}")]
    Msg(String),
}

/// A `Result` type alias using our [`Error`] type.
pub type Result<T> = std::result::Result<T, Error>;

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Msg(s)
    }
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::Msg(s.to_string())
    }
}
