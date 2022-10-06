//! Content provider trait. It defines methods for for getting content of flows from files, http or library references.
/// The Content Provider for File contents
#[cfg(feature = "file_provider")]
pub mod file_provider;
/// The Content Provider for Http contents
#[cfg(feature = "http_provider")]
pub mod http_provider;
/// The Content Provider for contents fetched over the p2p network of flow peers
#[cfg(feature = "p2p_provider")]
pub mod p2p_provider;
