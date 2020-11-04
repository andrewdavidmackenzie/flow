
//! `flowruntime` is a crate that defines a set of functions for a flow program to interact with the
//! host system, such as files, stdio etc.

use std::sync::{Arc, Mutex};

use flowrstructs::lib_manifest::{ImplementationLocator::Native, LibraryManifest};
use flowrstructs::manifest::MetaData;

use crate::client_server::RuntimeServerConnection;

/// `args` is a module to interact with a programs arguments
pub mod args;
/// `file` is a module to interact with the file system
pub mod file;
/// `stdio` is a module to interact with standard IO
pub mod stdio;
/// `image` is a module to interact with images
pub mod image;

/// Return a `LibraryManifest` for the run-time functions
pub fn get_manifest(server_context: Arc<Mutex<RuntimeServerConnection>>) -> LibraryManifest {
    let metadata = MetaData {
        name: "flowruntime".into(),
        version: "0.1.0".into(),
        description: "Flow Runtime functions".into(),
        authors: vec!("Andrew Mackenzie".to_string())
    };
    let mut manifest = LibraryManifest::new(metadata);

    manifest.locators.insert("lib://flowruntime/args/get/get".to_string(),
                             Native(Arc::new(args::get::Get { server_context: server_context.clone() })));
    manifest.locators.insert("lib://flowruntime/file/file_write/file_write".to_string(),
                             Native(Arc::new(file::file_write::FileWrite { server_context: server_context.clone() })));
    manifest.locators.insert("lib://flowruntime/image/image_buffer/image_buffer".to_string(),
                             Native(Arc::new(image::image_buffer::ImageBuffer { server_context: server_context.clone() })));
    manifest.locators.insert("lib://flowruntime/stdio/readline/readline".to_string(),
                             Native(Arc::new(stdio::readline::Readline { server_context: server_context.clone() })));
    manifest.locators.insert("lib://flowruntime/stdio/stdin/stdin".to_string(),
                             Native(Arc::new(stdio::stdin::Stdin { server_context: server_context.clone() })));
    manifest.locators.insert("lib://flowruntime/stdio/stdout/stdout".to_string(),
                             Native(Arc::new(stdio::stdout::Stdout { server_context: server_context.clone() })));
    manifest.locators.insert("lib://flowruntime/stdio/stderr/stderr".to_string(),
                             Native(Arc::new(stdio::stderr::Stderr { server_context })));

    manifest
}