//! Content provider trait. It defines methods for for getting content of flows from files, http or library references.
pub mod provider;
/// The Content Provider for File contents
pub mod file_provider;
/// The Content Provider for Http contents
mod http_provider;
/// The Content Provider for library contents ("lib://" schema) that uses the Library Search path to determine
/// where actual library contents can be found and then resolves the url to be either a File or Http
/// schema url. The Library Search path is initialized from `FLOW_LIB_PATH` and then any locations passed in via
/// the '-L' option are added to it.
mod lib_provider;

