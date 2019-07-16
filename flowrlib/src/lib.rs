//! Runtime library for flow execution. This will be linked with code generated from a flow definition
//! to enable it to be compiled and ran as a native program.
#[macro_use]
extern crate error_chain;
extern crate instant;
#[macro_use]
extern crate log;
extern crate multimap;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
#[cfg(not(target_arch = "wasm32"))]
extern crate wasmi;

pub mod info;
pub mod url;
pub mod coordinator;
pub mod implementation;
pub mod implementation_table;
pub mod function;
pub mod manifest;
pub mod input;
pub mod loader;
pub mod provider;

mod execution;
mod wasm;
mod run_state;

#[cfg(feature = "metrics")]
mod metrics;

#[cfg(feature = "debugger")]
mod debugger;
#[cfg(feature = "debugger")]
pub mod debug_client;

// We'll put our errors in an `errors` module, and other modules in
// this crate will `use errors::*;` to get access to everything
// `error_chain!` creates.
pub mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain! {
    }
}

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    foreign_links {
        Io(::std::io::Error);
        Serde(serde_json::error::Error);
        Recv(std::sync::mpsc::RecvError);
    }
}
