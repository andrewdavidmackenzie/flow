//! Content provider trait. It defines methods for for getting content of flows from files, http or library references.
pub mod provider;
/// The Content Provider for File contents
pub mod file_provider;
/// The Content Provider for Http contents
mod http_provider;

