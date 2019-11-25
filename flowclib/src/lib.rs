// TODO #![deny(missing_docs)]
//! This is the rust `flow` Compiler Library. It can be linked with other code to produce
//! a flow compiler, such as the `flowc` command line flow compiler.
#[macro_use]
extern crate error_chain;

pub mod deserializers;
pub mod dumper;
pub mod info;
pub mod compiler;
pub mod generator;
pub mod model;

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
        Io(std::io::Error);
    }
}