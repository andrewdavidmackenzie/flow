//! Error types for the `flowrgui` binary.

pub use flowcore::errors::ResultExt;
use thiserror::Error;

/// The error type for `flowrgui` operations.
#[derive(Debug, Error)]
pub enum Error {
    /// A URL parsing error
    #[error("{0}")]
    Url(#[from] url::ParseError),
    /// An error from flowcore
    #[error("{0}")]
    FlowCore(#[from] flowcore::errors::Error),
    /// An error from the runtime library
    #[error("{0}")]
    Runtime(#[from] flowrlib::errors::Error),
    /// An I/O error
    #[error("{0}")]
    Io(#[from] std::io::Error),
    /// An Iced GUI error
    #[error("{0}")]
    Iced(#[from] iced::Error),
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
