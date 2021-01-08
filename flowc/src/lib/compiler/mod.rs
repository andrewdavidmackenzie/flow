//! Compile module that loads flow definition, compiles flows and tables and then generates JSON manifest of processes
pub mod loader;
pub mod compile;
pub mod compile_wasm;
mod connector;
mod gatherer;
mod checker;
mod optimizer;