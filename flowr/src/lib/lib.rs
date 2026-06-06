#![deny(clippy::unwrap_used, clippy::expect_used)]
//! `flowrlib` is the runtime library for flow execution. This can be used to produce a flow runner,
//! such as the `flowr` command line runner.
//!
//! It is responsible for reading a flow's compiled `FlowManifest``flowcore::model::flow_manifest::FlowManifest`,
//! loading the required libraries using `LibraryManifests``flowcore::model::lib_manifest::LibraryManifest`
//! files and then coordinating the execution of [functions][flowcore::model::runtime_function::RuntimeFunction]
//! by dispatching [Jobs][job::Job] (that include a reference to the function's
//! [Implementations][flowcore::Implementation] and the input values required to run) then
//! gathering the Result and passing the output value to other connected functions in the
//! [flow graph][flowcore::model::flow_manifest::FlowManifest]

/// Re-export [Block][block::Block] from flowcore
pub use flowcore::model::block;

/// Provides [Coordinator][coordinator::Coordinator] responsible for coordinating the execution of flows submitted to it
pub mod coordinator;

#[cfg(feature = "debugger")]
/// Re-export `DebugCommand` from flowcore
pub use flowcore::model::debug_command;

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

/// Re-export [Job][job::Job] from flowcore
pub use flowcore::model::job;

/// The `SubmissionHandler`[`submission_handler::SubmissionHandler`] trait defines  methods a client
/// must implement in order to handle submissions from a client
#[cfg(feature = "submission")]
pub mod submission_handler;

/// The `DebuggerHandler` `debugger_handler::DebuggerHandler` trait defines methods a a client must
/// implement in order to handle the interaction between a client and the debugger (in the Coordinator)
#[cfg(feature = "debugger")]
pub mod debugger_handler;

/// Provides [`RunState`][run_state::RunState] that tracks the current runtime state,
/// used by [`Coordinator`][coordinator::Coordinator]
pub mod run_state;

/// ZMQ-based connections for client-coordinator communication
pub mod connections;

/// Re-export well-known service names from flowcore
pub use flowcore::services;

/// Re-export mDNS-SD service discovery helpers from flowcore
#[cfg(not(target_arch = "wasm32"))]
pub use flowcore::discovery;

#[cfg(feature = "debugger")]
/// [`DebugServerMessage`][debug_server_message::DebugServerMessage] — messages sent from the
/// debug server to debug clients
pub mod debug_server_message;

#[cfg(feature = "debugger")]
/// ZMQ-based [`DebuggerHandler`][debugger_handler::DebuggerHandler] implementation for
/// forwarding debug events to an external client like `flowrdb`
pub mod debug_zmq_handler;

#[cfg(feature = "debugger")]
/// CLI REPL debug client used by the `flowrdb` binary
pub mod debug_client;

#[cfg(feature = "debugger")]
mod debugger;

/// `wasmtime` module contains a number of implementations of the wasm execution
mod wasm;

#[cfg(debug_assertions)]
mod checks;
