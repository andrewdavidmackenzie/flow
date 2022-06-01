#![deny(missing_docs)]
#![warn(clippy::unwrap_used)]
//! Runtime library for flow execution. This will be linked with other code to produce a
//! or runner, such as `flowr` command line runner.
//!
//! It is responsible for reading a flow definition in a `Manifest` file, loading the required
//! libraries from `LibraryManifest` files and then coordinating the execution by dispatching `Jobs`
//! to be executed by `Function` `Implementations`, providing them the `Inputs` required to run and
//! gathering the `Outputs` produced and passing those `Outputs` to other connected `Functions` in
//! the network of `Functions`.

/// `coordinator` is the module that coordinates the execution of flows submitted to it
pub mod coordinator;

/// `info` offers methods to get information about this library
pub mod info;

/// `loader` is responsible for loading a flow from it's manifest and loading libraries it uses
pub mod loader;

/// Structure that defines/tracks the current runtime state
pub mod run_state;

/// Trait for a set of methods a server using the library must supply
pub mod server;

#[cfg(feature = "debugger")]
mod debugger;
#[cfg(feature = "debugger")]
/// struct with spec for specifying Breakpoints to debugger commands
pub mod breakpoint_spec;
#[cfg(feature = "debugger")]
/// `debug_command` provides the `DebugCommand` enum for commands from debug client to debug server
pub mod debug_command;

/// Executor module provides mechanisms for jobs to be executed
pub mod executor;

/// `wasm` module contains a number of implementations of the wasm execution
#[allow(unused_attributes)]
#[cfg_attr(feature = "wasmtime_runtime", path = "wasm/wasmtime.rs")]
#[cfg_attr(feature = "wasmi_runtime", path = "wasm/wasmi.rs")]
mod wasm;

/// We'll put our errors in an `errors` module, and other modules in this crate will `use errors::*;`
/// to get access to everything `error_chain` creates.
pub mod errors;

/// module providing `Block` struct from runtime that is required for debugging and tracing
pub mod block;

/// module providing `Job` struct from runtime that is required for debugging and tracing
pub mod job;

#[cfg(debug_assertions)]
mod checks;
