#![deny(missing_docs)]
//! `runtime` is a crate that defines a set of functions for a flow program to interact with the
//! host system, such as files, stdio etc.

/// `manifest` module takes care of creating a `LibraryManifest` for the runtime functions
pub mod manifest;
/// `runtime_client` is a trait for clients connectibe to the runtime must implement
pub mod runtime_client;
/// `args` is a module to interact with a programs arguments
mod args;
/// `file` is a module to interact with the file system
mod file;
/// `stdio` is a module to interact with standard IO
mod stdio;