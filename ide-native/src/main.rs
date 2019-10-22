#![deny(missing_docs)]
//! The `ide-native` is a prototype of a native IDE for `flow` programs.
//! It  is written in rust and called from JavaScript
//! `main` will link with it and compile to a binary, although it's the `lib.rs` that is
//! compiled ro WebAssembly and linked with JavaScript.

mod flow;

/// Main function for ide_native - not used in the build with JavaScript
pub fn main() {
    let flowclib_version = flow::flowclib_version();
    println!("Flowclib: {}", flowclib_version);
}