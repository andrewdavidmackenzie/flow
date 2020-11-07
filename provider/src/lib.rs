// TODO #![deny(missing_docs)]
#![warn(clippy::unwrap_used)]
//! A module to help parse command line arguments for flow URLs and fetch the associated content
#[macro_use]
extern crate error_chain;

pub mod content;
pub mod args;

#[doc(hidden)]
pub mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain! {}
}

#[doc(hidden)]
error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    foreign_links {
        Io(std::io::Error);
        Easy(curl::Error);
    }
}