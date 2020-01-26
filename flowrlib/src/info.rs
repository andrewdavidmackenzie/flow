const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Return the version number of the `flowrlib` library
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