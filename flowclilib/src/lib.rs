//! Runtime library with methods to help flow execution. This will be linked with code generated from a flow definition
//! to enable it to be compiled and run as a native program.
#[macro_use]
extern crate log;
extern crate simplog;
extern crate clap;

pub mod startup;