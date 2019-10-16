#![deny(missing_docs)]
//! A module to help parse command line arguments for flow URLs and fetch the associated content
#[macro_use]
extern crate error_chain;
extern crate flowrlib;
#[macro_use]
extern crate log;

pub mod content;