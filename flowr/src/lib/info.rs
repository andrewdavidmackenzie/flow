const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Return the version number of the `flowrlib` library
#[must_use]
pub fn version() -> &'static str {
    VERSION
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use super::*;

    #[test]
    fn can_get_version() {
        assert!(!version().is_empty());
    }
}
