//! Content provider trait. It defines methods for for getting content of flows from files, http or library references.
pub mod provider;
/// The Content Provider for File contents
pub mod file_provider;
/// The Content Provider for Http contents
mod http_provider;
/// The Content Provider for library contents ("lib://" schema) that uses `FLOW_LIB_PATH` to determine
/// where actual library contents can be found and then resolves the url to be either a File or Http
/// schema url
mod lib_provider;

