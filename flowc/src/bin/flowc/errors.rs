//! Error types for the `flowc` compiler binary.

pub use flowcore::bail;
pub use flowcore::errors::ResultExt;
use thiserror::Error;

/// The error type for `flowc` binary operations.
#[derive(Debug, Error)]
pub enum Error {
    /// An error from flowcore
    #[error("{0}")]
    Core(#[from] flowcore::errors::Error),
    /// An error from the compiler library
    #[error("{0}")]
    Compiler(#[from] flowrclib::errors::Error),
    /// An I/O error
    #[error("{0}")]
    Io(#[from] std::io::Error),
    /// A URL parsing error
    #[error("{0}")]
    Url(#[from] url::ParseError),
    /// A TOML deserialization error
    #[error("{0}")]
    Toml(#[from] toml::de::Error),
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
