#![deny(missing_docs)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::indexing_slicing)]
//! `flowrlib` is the runtime library for flow execution. This can be used to produce a flow runner,
//! such as the `flowr` command line runner.
//!
//! It is responsible for reading a flow's compiled [FlowManifest][flowcore::model::flow_manifest::FlowManifest],
//! loading the required libraries using [LibraryManifests][flowcore::model::lib_manifest::LibraryManifest]
//! files and then coordinating the execution of [functions][flowcore::model::runtime_function::RuntimeFunction]
//! by dispatching [Jobs][job::Job] (that include a reference to the function's
//! [Implementations][flowcore::Implementation] and the input values required to run) then
//! gathering the Result and passing the output value to other connected functions in the
//! [flow graph][flowcore::model::flow_manifest::FlowManifest]

/// Provides [Block][block::Block] that represents a block imposed on a function due to destination being busy
pub mod block;

/// Provides [Coordinator][coordinator::Coordinator] responsible for coordinating the execution of flows submitted to it
pub mod coordinator;

#[cfg(feature = "debugger")]
/// Provides the [DebugCommand][debug_command::DebugCommand] enum for commands from debug client to debug server
pub mod debug_command;

/// Provides [Dispatcher][dispatcher::Dispatcher] used by the [Coordinator][coordinator::Coordinator]
/// to dispatch [Jobs][job::Job] for execution by an [Executor][executor::Executor]
pub mod dispatcher;

/// Holds all [Error][errors::Error] types, and other modules in this crate will `use errors::*;`
/// to get access to everything `error_chain` creates.
pub mod errors;

/// Provides [Executor][executor::Executor] that receives jobs for execution, executes them and returns results
pub mod executor;

/// Provides methods to get information about this library
pub mod info;

/// Provides [Job][job::Job] that holds jobs before and after their execution
pub mod job;

/// Provides a number of traits that define methods used in protocols between server and clients
/// that a client must implement. Such as [DebuggerProtocol][protocols::DebuggerProtocol] and
/// [SubmissionProtocol][protocols::SubmissionProtocol]
pub mod protocols;

/// Provides [RunState][run_state::RunState] that tracks the current runtime state,
/// used by [Coordinator][coordinator::Coordinator]
pub mod run_state;

/// Provides well-known service names used across multiple binary crates
pub mod services;

#[cfg(feature = "debugger")]
mod debugger;

/// `wasmtime` module contains a number of implementations of the wasm execution
mod wasm;

#[cfg(debug_assertions)]
mod checks;
