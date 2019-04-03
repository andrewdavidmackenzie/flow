//! A module to help parse command line arguments for flow URLs and fetch the associated content
extern crate curl;
extern crate flowrlib;
extern crate glob;
#[macro_use]
extern crate log;
extern crate simpath;
extern crate tempdir;
extern crate url;

pub mod content;
pub mod args;