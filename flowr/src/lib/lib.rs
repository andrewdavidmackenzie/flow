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
/// `context` module implements the executor/server side of the runtime functions and appears
/// to user code like a library
mod context;
/// `info` offers methods to get information about this library
pub mod info;
/// `loader` is responsible for loading a flow from it's manifest and loading libraries it uses
pub mod loader;

/// message_queue implementation of the communications between the runtime client, debug client and
/// the runtime server and debug server.
pub mod client_server;

/// 'debug' defines structs passed between the Server and the Client regarding debug events
/// and client responses to them
#[cfg(feature = "debugger")]
pub mod debug_messages;

/// 'runtime_messages' defines messages passed between the Server and the Client during flow execution
pub mod runtime_messages;

/// Structure that defines/tracks the current runtime state
pub mod run_state;

#[cfg(feature = "debugger")]
mod debugger;

mod execution;

/// `wasm` module contains a number of implementations of the wasm execution
#[allow(unused_attributes)]
#[cfg_attr(feature = "wasmtime_runtime", path = "wasm/wasmtime.rs")]
#[cfg_attr(feature = "wasmi_runtime", path = "wasm/wasmi.rs")]
mod wasm;

/// We'll put our errors in an `errors` module, and other modules in this crate will `use errors::*;`
/// to get access to everything `error_chain` creates.
pub mod errors;

/// 'metrics' defines a struct for tracking metrics gathered during flow execution
#[cfg(feature = "metrics")]
pub mod metrics;

mod block;
/// `client_provider` is a special content provider that makes requests to the client to fetch files
mod client_provider;
mod job;
