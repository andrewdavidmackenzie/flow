//! Compile module that loads flow definition, compiles flows and tables and then generates JSON manifest of processes

/// Loader is responsible for parsing flow definitions from text files and creating in memory
pub mod loader;

/// Compile is responsible for connecting outputs to inputs across functions and flows and
/// flattening the model to be just functions, then taking care of manifest generation
pub mod compile;

/// `compile_wasm` has helper functions to compile WASM implementations of libs and supplied functions
pub mod compile_wasm;

mod checker;
mod connector;
mod gatherer;
mod optimizer;
