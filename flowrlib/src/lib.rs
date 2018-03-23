//! Runtime library for flow execution. This will be linked with code generated from a flow definition
//! to enable it to be compiled and ran as a native program.
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_json;

pub mod info;
pub mod execution;
mod runlist;
pub mod value;
pub mod implementation;
pub mod function;
pub mod runnable;
pub mod zero_fifo;