// TODO #![deny(missing_docs)]
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

/// `info` offers methods to get information about this library
pub mod info;
/// `coordinator` is the module that coordinates the execution of flows submitted to it
pub mod coordinator;
/// `lib_manifest` defines the structs for specifying a Library's manifest and methods to load it
pub mod lib_manifest;
/// `function` defines functions that form part of a flow
pub mod function;
/// `output_connection` defines a struct for a function's output connection
pub mod output_connection;
/// `manifest` is the struct that specifies the manifest of functions in a flow
pub mod manifest;
/// `input` defines the struct for inputs to functions in a flow
pub mod input;
/// `loader` is responsible for loading a flow from it's manifest and loading libraries it uses
pub mod loader;
/// `provider` is a trait that is implemented to provide content to flowrlib in different environments
/// it runs in
pub mod provider;

#[cfg(feature = "debugger")]
/// 'debug_client' is used to connect a debugger to the run-time for debugging of flows
/// and is an optional feature called "debugger"
pub mod debug_client;

/// We'll put our errors in an `errors` module, and other modules in this crate will `use errors::*;`
/// to get access to everything `error_chain!` creates.
#[doc(hidden)]
pub mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain! {}
}

#[cfg(feature = "debugger")]
mod debugger;

mod execution;
mod wasm;
mod run_state;

#[cfg(feature = "metrics")]
mod metrics;

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
    }
}
