//! Runtime library for flow execution. This will be linked with code generated from a flow definition
//! to enable it to be compiled and ran as a native program.
#[macro_use]
extern crate log;
#[cfg(test)]
#[macro_use]
extern crate serde_json;
#[cfg(not(test))]
extern crate serde_json;
extern crate simplog;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[cfg(not(target_arg = "wasm32"))]
extern crate wasmi;

pub mod info;
pub mod url;
pub mod execution;
pub mod runlist;
pub mod implementation;
pub mod implementation_table;
pub mod process;
pub mod manifest;
pub mod input;
pub mod loader;
pub mod provider;
mod wasm;
#[cfg(feature = "debugger")]
pub mod debugger;