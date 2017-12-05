const VERSION: &'static str = env!("CARGO_PKG_VERSION");

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