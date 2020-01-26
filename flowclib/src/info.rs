//! Info module provides methods to get additional information about the flowclib library

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// return the version number of the library as a string of the form "M.m.p"
///
/// - M is a one or two digit Major version number
/// - m is a one or two digit Minor version number
/// - p is a one or two digit Patch version number
pub fn version() -> &'static str {
    VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_get_version() {
        assert!(!version().is_empty());
    }
}