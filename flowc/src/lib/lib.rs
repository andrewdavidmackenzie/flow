#![deny(missing_docs)]
#![warn(clippy::unwrap_used)]

//! The `flow` Compiler Library
//!
//! Used by `flowc` to produce the command line flow compiler.

/// The main `compiler` module for parsing flow descriptions into memory.
///
/// It reads [flow definitions][flowcore::model::flow_definition] into memory, flattens and connects
/// that into a graph of [runtime functions][flowcore::model::runtime_function::RuntimeFunction]
pub mod compiler;

/// used to output a human-readable version of the model and compiler tables to help debug
/// compiler problems
pub mod dumper;

/// takes care of generating the [flow's manifest][flowcore::model::flow_manifest::FlowManifest]
/// (for later execution by a flow runner) from the [compiler tables][compiler::compile::CompilerTables]
pub mod generator;

/// provides methods to get information about this version of the flowrclib library
pub mod info;

/// contains `errors::Error` that other modules in this crate will `use errors::*;`
/// to get access to everything `error_chain` creates.
pub mod errors;
