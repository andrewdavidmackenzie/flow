#![deny(missing_docs)]
//! The `flowide` is a prototype of an IDE for `flow` programs. It  is written in rust
//! and compiles to WebAssembly for use with JavaScript  inside an electron app.
//! `main` will link with it and compile to a binary, although it's the `lib.rs` that is
//! compiled ro WebAssembly and linked with JavaScript.

/// Main function for the flowide - not used in the WebAssembly compile
pub fn main() {
    println!("flowide: version = {}", env!("CARGO_PKG_VERSION"));
    println!("flowrlib: version = {}", flowrlib::info::version());
    println!("flowclib: version = {}", flowclib::info::version());
}