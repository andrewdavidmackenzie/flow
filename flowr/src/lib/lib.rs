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
/// `flowruntime` module implements the executor/server side of the runtime functions and appears
/// to user code like a library
mod flowruntime;
/// `info` offers methods to get information about this library
pub mod info;
/// `loader` is responsible for loading a flow from it's manifest and loading libraries it uses
pub mod loader;

/// `client_server` module contains a number of implementations of the communications between the
/// runtime client, debug client and the runtime server and debug server.
#[allow(unused_attributes)]
#[cfg_attr(feature = "distributed", path = "client_server/message_queue.rs")]
#[cfg_attr(not(feature = "distributed"), path = "client_server/channels.rs")]
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
mod wasm;

/// We'll put our errors in an `errors` module, and other modules in this crate will `use errors::*;`
/// to get access to everything `error_chain!` creates.
pub mod errors;

/// 'metrics' defines a struct for tracking metrics gathered during flow execution
#[cfg(feature = "metrics")]
pub mod metrics;
