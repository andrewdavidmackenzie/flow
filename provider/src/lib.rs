// TODO #![deny(missing_docs)]
#![warn(clippy::unwrap_used)]
//! The `Provider` that helps resolve Urls and calls the underlying content provider

#[macro_use]
extern crate error_chain;

mod content;
pub mod lib_provider;

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
        Url(url::ParseError);
    }
}
