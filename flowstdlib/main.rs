#![warn(clippy::unwrap_used)]

use lib_path::check_flow_lib_path;

mod lib_path;

pub fn main() {
    println!(
        "'{}' version {}",
        env!("CARGO_CRATE_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    println!("For more details see: {}", env!("CARGO_PKG_HOMEPAGE"));
    println!(
        "'{}' is installed in '{}'",
        env!("CARGO_CRATE_NAME"),
        env!("CARGO_MANIFEST_DIR")
    );
    check_flow_lib_path();
}

#[cfg(test)]
mod test {
    use std::path::Path;

    #[test]
    fn check_manifest() {
        // check the manifest was created
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("manifest.json");
        assert!(manifest.exists());
    }

    #[test]
    fn get_manifest_test() {
        let manifest = flowstdlib::get_manifest().expect("Could not get manifest");
        assert_eq!(manifest.locators.len(), 30);
    }
}
