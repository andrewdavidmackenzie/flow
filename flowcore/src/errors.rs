//! Error types for the `flowcore` crate.

use std::fmt;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

/// The error type for `flowcore` operations.
#[derive(Debug, Error)]
pub enum Error {
    /// A URL parsing error
    #[error("{0}")]
    Url(#[from] url::ParseError),
    /// An I/O error
    #[error("{0}")]
    Io(#[from] std::io::Error),
    /// A JSON serialization/deserialization error
    #[error("{0}")]
    Serde(#[from] serde_json::error::Error),
    /// An integer conversion error
    #[error("{0}")]
    Conversion(#[from] std::num::TryFromIntError),
    /// A general error message
    #[error("{0}")]
    Msg(String),
}

/// A `Result` type alias using our [`Error`] type.
pub type Result<T> = std::result::Result<T, Error>;

/// Convenience macro for returning an error with a formatted message.
///
/// This replaces the `bail!` macro from `error-chain`.
///
/// Supports both `bail!("literal {}", arg)` and `bail!(expr)` forms.
/// Uses `From<String>` so it works with any error type that implements it.
#[macro_export]
macro_rules! bail {
    ($fmt:literal $(, $arg:expr)* $(,)?) => {
        return Err(format!($fmt $(, $arg)*).into())
    };
    ($msg:expr) => {
        return Err(<_ as From<String>>::from($msg.into()))
    };
}

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

/// In order to send Jobs, containing Results and hence Errors, back and forth between the Client
/// and the Server it must implement Serialize and Deserialize.
impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

struct ErrorVisitor;

impl Visitor<'_> for ErrorVisitor {
    type Value = Error;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an Error string")
    }

    fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Error::Msg(value.to_string()))
    }
}

impl<'de> Deserialize<'de> for Error {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Error, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ErrorVisitor)
    }
}

/// We want to clone Job, which contains Result which contains Error
impl Clone for Error {
    fn clone(&self) -> Self {
        Error::Msg(self.to_string())
    }
}

/// Extension trait for adding context to `Result` types, replacing `error-chain`'s `ResultExt`.
///
/// This provides a `chain_err` method that wraps the original error's display string
/// with additional context, similar to `anyhow::Context`.
pub trait ResultExt<T> {
    /// Wrap the error with additional context provided by a closure.
    ///
    /// # Errors
    ///
    /// Returns an error combining the context message with the original error's display.
    fn chain_err<F, S>(self, context: F) -> Result<T>
    where
        F: FnOnce() -> S,
        S: Into<String>;
}

impl<T, E: fmt::Display> ResultExt<T> for std::result::Result<T, E> {
    fn chain_err<F, S>(self, context: F) -> Result<T>
    where
        F: FnOnce() -> S,
        S: Into<String>,
    {
        self.map_err(|e| Error::Msg(format!("{}: {e}", context().into())))
    }
}

impl<T> ResultExt<T> for Option<T> {
    fn chain_err<F, S>(self, context: F) -> Result<T>
    where
        F: FnOnce() -> S,
        S: Into<String>,
    {
        self.ok_or_else(|| Error::Msg(context().into()))
    }
}
