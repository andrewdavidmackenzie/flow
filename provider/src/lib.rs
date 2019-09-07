//! A module to help parse command line arguments for flow URLs and fetch the associated content
extern crate curl;
#[macro_use]
extern crate error_chain;
extern crate flowrlib;
extern crate glob;
#[macro_use]
extern crate log;
extern crate simpath;
extern crate tempdir;
extern crate url;

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