// TODO #![deny(missing_docs)]
//! A module to help parse command line arguments for flow URLs and fetch the associated content
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;

pub mod content;
pub mod args;

pub mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain! {}
}

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    foreign_links {
        Runtime(flowrlib::errors::Error);
        Io(std::io::Error);
    }
}