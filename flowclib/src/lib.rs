// TODO #![deny(missing_docs)]
extern crate erased_serde;
#[macro_use]
extern crate error_chain;
extern crate flowrlib;
#[macro_use]
extern crate log;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[cfg(test)]
#[macro_use]
extern crate serde_json;
#[cfg(not(test))]
extern crate serde_json;
extern crate serde_yaml;
#[macro_use]
extern crate shrinkwraprs;
extern crate strfmt;
extern crate toml;
#[cfg(test)]
extern crate url;

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