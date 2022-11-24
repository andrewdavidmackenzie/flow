#![deny(missing_docs)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::indexing_slicing)]
//! `flowrlib` is the runtime library for flow execution. This can be used to produce a flow runner,
//! such as the `flowr` command line runner.
//!
//! It is responsible for reading a flow's compiled [flowcore::model::flow_manifest::FlowManifest],
//! loading the required libraries from `LibraryManifest` files and then coordinating the execution by dispatching `Jobs`
//! to be executed by `Function` `Implementations`, providing them the `Inputs` required to run and
//! gathering the `Outputs` produced and passing those `Outputs` to other connected `Functions` in
//! the network of `Functions`.

/// Provides [block::Block] that represents a block imposed on a function due to destination being busy
pub mod block;

/// Provides [coordinator::Coordinator] responsible for coordinating the execution of flows submitted to it
pub mod coordinator;

#[cfg(feature = "debugger")]
/// Provides the [debug_command::DebugCommand] enum for commands from debug client to debug server
pub mod debug_command;

/// Provides [dispatcher::Dispatcher] that dispatches [job::Job]s for execution
pub mod dispatcher;

/// Holds all [errors::Error] types, and other modules in this crate will `use errors::*;`
/// to get access to everything `error_chain` creates.
pub mod errors;

/// Provides [executor::Executor] that receives jobs for execution, executes them and returns results
pub mod executor;

/// Provides methods to get information about this library
pub mod info;

/// Provides [job::Job] that holds jobs before and after their execution
pub mod job;

/// Provides a number of traits that define methods used in protocols between server and clients
/// that a client must implement. Such as [protocols::DebuggerProtocol] and [protocols::SubmissionProtocol]
pub mod protocols;

/// Provides [run_state::RunState] that tracks the current runtime state
pub mod run_state;

/// Provides well-known service names used across multiple binary crates
pub mod services;

#[cfg(feature = "debugger")]
mod debugger;

/// `wasmtime` module contains a number of implementations of the wasm execution
mod wasm;

#[cfg(debug_assertions)]
mod checks;
