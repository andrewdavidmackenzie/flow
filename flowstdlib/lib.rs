#![deny(missing_docs)]
#![warn(clippy::unwrap_used)]

//! This is the `flowstdlib` standard library of functions for `flow`

use std::sync::Arc;

use url::Url;

use errors::*;
use flowcore::flow_manifest::MetaData;
use flowcore::lib_manifest::{ImplementationLocator::Native, LibraryManifest};

/// We'll put our errors in an `errors` module, and other modules in this crate will `use errors::*;`
/// to get access to everything `error_chain` creates.
pub mod errors;

//include!(concat!(env!("OUT_DIR"), "/manifest.rs"));
include!("manifest.rs");
