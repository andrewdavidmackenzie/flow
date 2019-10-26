#![deny(missing_docs)]
//! `flowstdlib` is a standard library of functions for `flow` programs to use.
//! It can be compiled and linked natively to a runtime, or each function can be
//! compiled to WebAssembly and loaded from file by the runtime.

/// Use serde_json for data representations of Values passed to/from functions
extern crate serde_json;

#[cfg(feature = "static")]
/// Control functions
pub mod control;

#[cfg(feature = "static")]
/// Data functions
pub mod data;

#[cfg(feature = "static")]
/// Formatting functions
pub mod fmt;

#[cfg(feature = "static")]
/// Imaging functions
pub mod img;

#[cfg(feature = "static")]
/// Maths functions
pub mod math;

