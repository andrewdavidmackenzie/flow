// TODO #![deny(missing_docs)]
#![warn(clippy::unwrap_used)]
//! Runtime library for flow execution. This will be linked with other code to produce a
//! or runner, such as `flowr` command line runner.
//!
//! It is responsible for reading a flow definition in a `Manifest` file, loading the required
//! libraries from `LibraryManifest` files and then coordinating the execution by dispatching `Jobs`
//! to be executed by `Function` `Implementations`, providing them the `Inputs` required to run and
//! gathering the `Outputs` produced and passing those `Outputs` to other connected `Functions` in
//! the network of `Functions`.
//!
#[macro_use]
extern crate error_chain;

/// `coordinator` is the module that coordinates the execution of flows submitted to it
pub mod coordinator;
/// `flowruntime` module implements the executor/server side of the runtime functions and appears
/// to user code like a library
mod flowruntime;
/// `info` offers methods to get information about this library
pub mod info;
/// `loader` is responsible for loading a flow from it's manifest and loading libraries it uses
pub mod loader;

#[allow(unused_attributes)]
#[cfg_attr(feature = "distributed", path = "client_server/message_queue.rs")]
#[cfg_attr(not(feature = "distributed"), path = "client_server/channels.rs")]
pub mod client_server;

/// 'debug' defines structs passed between the Server and the Client regarding debug events
/// and client responses to them
#[cfg(feature = "debugger")]
pub mod debug_messages;

/// 'runtime' defines structs passed between the Server and the Client regarding runtime events
/// and client responses to them
pub mod runtime_messages;

/// Structure that defines/tracks the current runtime state
pub mod run_state;

#[cfg(feature = "debugger")]
mod debugger;

mod execution;
mod wasm;

#[cfg(feature = "metrics")]
pub mod metrics;

/// We'll put our errors in an `errors` module, and other modules in this crate will `use errors::*;`
/// to get access to everything `error_chain!` creates.
#[doc(hidden)]
pub mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain! {}
}

// Specify the errors we will produce and foreign links
#[doc(hidden)]
error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    foreign_links {
        Io(std::io::Error);
        Serde(serde_json::error::Error);
        Recv(std::sync::mpsc::RecvError);
        Url(url::ParseError);
        FlowStdLib(flowstdlib::errors::Error);
        FlowrCore(flowcore::errors::Error);
    }
}
