#![deny(missing_docs)]
#![warn(clippy::unwrap_used)]

//! This is the rust `flow` Compiler Library. It can be linked with other code to produce
//! a flow compiler, such as the `flowc` command line flow compiler.

/// Compiler that reads flow definitions into memory and flattens and connects the model
pub mod compiler;
/// A set of serializers to read flow models from various text formats based on file extension
pub mod deserializers;
/// Dumper can output a human readable version of the model and compiler tables to help debug
/// compiler problems
pub mod dumper;
/// Generator takes care of generating the flow's manifest (for execution) from the compiler tables
pub mod generator;
/// Functions to get information about this version of the flowc library
pub mod info;
/// Model defines the structures for the different data types in a flow for use in memory
pub mod model;

/// We'll put our errors in an `errors` module, and other modules in this crate will `use errors::*;`
/// to get access to everything `error_chain` creates.
pub mod errors;
