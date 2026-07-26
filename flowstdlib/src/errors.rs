//! Error types for the `flowstdlib` crate.

pub use flowcore::bail;
pub use flowcore::errors::ResultExt;
use thiserror::Error;

/// The error type for `flowstdlib` operations.
#[derive(Debug, Error)]
pub enum Error {
    /// A URL parsing error
    #[error("{0}")]
    Url(#[from] url::ParseError),
    /// An integer conversion error
    #[error("{0}")]
    Conversion(#[from] std::num::TryFromIntError),
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
