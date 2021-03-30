#![warn(clippy::unwrap_used)]

#[macro_use]
extern crate error_chain;

/// `function` defines functions that form part of a flow
pub mod function;
/// `input` defines the struct for inputs to functions in a flow
pub mod input;
/// `lib_manifest` defines the structs for specifying a Library's manifest and methods to load it
pub mod lib_manifest;
/// `manifest` is the struct that specifies the manifest of functions in a flow
pub mod manifest;
/// `output_connection` defines a struct for a function's output connection
pub mod output_connection;
/// Utility functions related to Urls
pub mod url_helper;

/// We'll put our errors in an `errors` module, and other modules in this crate will `use errors::*;`
/// to get access to everything `error_chain!` creates.
#[doc(hidden)]
pub mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain! {}
}

// Specify the errors we will produce and foreign links
#[doc(hidden)]
error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    foreign_links {
        Io(std::io::Error);
        Serde(serde_json::error::Error);
        Recv(std::sync::mpsc::RecvError);
        Provider(provider::errors::Error);
    }
}
