//! Runtime library for flow execution. This will be linked with code generated from a flow definition
//! to enable it to be compiled and ran as a native program.
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_json;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[cfg(not(target_arch = "wasm32"))]
extern crate wasmi;
extern crate multimap;
extern crate instant;

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
