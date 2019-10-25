#![deny(missing_docs)]
//! `flowstdlib` is a standard library of functions for `flow` programs to use.
//! It can be compiled and linked natively to a runtime, or each function can be
//! compiled to WebAssembly and loaded from file by the runtime.

/// Use serde_json for data representations of Values passed to/from functions
#[macro_use]
extern crate serde_json;

/// Control functions
pub mod control;
/// Data functions
pub mod data;
/// Formatting functions
pub mod fmt;
/// Imaging functions
pub mod img;
/// Maths functions
pub mod math;

