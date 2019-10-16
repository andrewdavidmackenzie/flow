#![deny(missing_docs)]
//! The `flowide` is a prototype of an IDE for `flow` programs. It  is written in rust
//! and compiles to WebAssembly for use with JavaScript  inside an electron app.

/// Main function for the flowide
pub fn main() {
    println!("flowide: version = {}", env!("CARGO_PKG_VERSION"));
    println!("flowrlib: version = {}", flowrlib::info::version());
    println!("flowclib: version = {}", flowclib::info::version());
}