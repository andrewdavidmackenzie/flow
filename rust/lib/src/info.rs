
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn version() -> &'static str {
    VERSION
}