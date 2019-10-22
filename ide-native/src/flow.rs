#![deny(missing_docs)]
//! `ide_native` is a prototype of a native IDE for `flow` programs.
//! It consists of a libraary written in rust and called from JavaScript
//! We must build it as a dynamic library (without a main())
//! and `lib.rs` is the root of that crate.

extern crate flowclib;

/// `flowc_version` returns the version number string of the `flowclib` library.
/// Currently this is just used as a Proof-of-Concept of linking with one of the
/// flow libraries from this native app.
#[no_mangle]
pub extern fn flowclib_version() -> &'static str {
    flowclib::info::version()
}