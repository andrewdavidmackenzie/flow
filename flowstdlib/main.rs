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
