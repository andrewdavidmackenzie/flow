//! Error types for the `flowrlib` runtime library.

pub use flowcore::bail;
use thiserror::Error;

/// The error type for `flowrlib` operations.
#[derive(Debug, Error)]
pub enum Error {
    /// An I/O error
    #[error("{0}")]
    Io(#[from] std::io::Error),
    /// A JSON serialization/deserialization error
    #[error("{0}")]
    Serde(#[from] serde_json::error::Error),
    /// A URL parsing error
    #[error("{0}")]
    Url(#[from] url::ParseError),
    /// An error from flowcore
    #[error("{0}")]
    FlowrCore(#[from] flowcore::errors::Error),
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
