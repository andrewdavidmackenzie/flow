#![deny(missing_docs)]
//! `flowruntime` is a crate that defines a set of functions for a flow program to interact with the
//! host system, such as files, stdio etc.

use std::sync::{Arc, Mutex};

use flowrlib::lib_manifest::{ImplementationLocator::Native, LibraryManifest};
use flowrlib::manifest::MetaData;

use crate::runtime_client::RuntimeClient;

/// `runtime_client` is a trait for clients connectibe to the run-time must implement
pub mod runtime_client;
/// `args` is a module to interact with a programs arguments
pub mod args;
/// `file` is a module to interact with the file system
pub mod file;
/// `stdio` is a module to interact with standard IO
pub mod stdio;

/// Return a `LibraryManifest` for the run-time functions
pub fn get_manifest(client: Arc<Mutex<dyn RuntimeClient>>) -> LibraryManifest {
    let metadata = MetaData {
        name: "flowruntime".into(),
        version: "0.1.0".into(),
        description: "Runtime library provided by flowr binary".into(),
        authors: vec!("Andrew Mackenzie".to_string())
    };
    let mut manifest = LibraryManifest::new(metadata);

    manifest.locators.insert("lib://flowruntime/args/get/Get".to_string(),
                             Native(Arc::new(args::get::Get { client: client.clone() })));
    manifest.locators.insert("lib://flowruntime/file/file_write/FileWrite".to_string(),
                             Native(Arc::new(file::file_write::FileWrite { client: client.clone() })));
    manifest.locators.insert("lib://flowruntime/stdio/readline/Readline".to_string(),
                             Native(Arc::new(stdio::readline::Readline { client: client.clone() })));
    manifest.locators.insert("lib://flowruntime/stdio/stdin/Stdin".to_string(),
                             Native(Arc::new(stdio::stdin::Stdin { client: client.clone() })));
    manifest.locators.insert("lib://flowruntime/stdio/stdout/Stdout".to_string(),
                             Native(Arc::new(stdio::stdout::Stdout { client: client.clone() })));
    manifest.locators.insert("lib://flowruntime/stdio/stderr/Stderr".to_string(),
                             Native(Arc::new(stdio::stderr::Stderr { client: client.clone() })));

    manifest
}