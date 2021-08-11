//! `flowruntime` is a crate that defines a set of functions for a flow program to interact with the
//! host system, such as files, stdio etc.

use std::sync::{Arc, Mutex};

use url::Url;

use flowcore::flow_manifest::MetaData;
use flowcore::lib_manifest::{ImplementationLocator::Native, LibraryManifest};

use crate::client_server::ServerConnection;
use crate::errors::*;
use crate::runtime_messages::{ClientMessage, ServerMessage};

/// `env` is a module to interact with a programs arguments
pub mod env;
/// `file` is a module to interact with the file system
pub mod file;
/// `image` is a module to interact with images
pub mod image;
/// `stdio` is a module to interact with standard IO
pub mod stdio;

/// Return a `LibraryManifest` for the run-time functions
pub fn get_manifest(
    server_context: Arc<Mutex<ServerConnection<ServerMessage, ClientMessage>>>,
) -> Result<LibraryManifest> {
    let metadata = MetaData {
        name: "flowruntime".into(),
        version: "0.1.0".into(),
        description: "Flow Runtime functions".into(),
        authors: vec!["Andrew Mackenzie".to_string()],
    };
    let mut manifest = LibraryManifest::new(metadata);

    manifest.locators.insert(
        Url::parse("lib://flowruntime/env/args/args")
            .chain_err(|| "Could not parse url")
            .chain_err(|| "Could not parse url")?,
        Native(Arc::new(env::args::Args {
            server_context: server_context.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("lib://flowruntime/file/file_write/file_write")
            .chain_err(|| "Could not parse url")?,
        Native(Arc::new(file::file_write::FileWrite {
            server_context: server_context.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("lib://flowruntime/image/image_buffer/image_buffer")
            .chain_err(|| "Could not parse url")?,
        Native(Arc::new(image::image_buffer::ImageBuffer {
            server_context: server_context.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("lib://flowruntime/stdio/readline/readline")
            .chain_err(|| "Could not parse url")?,
        Native(Arc::new(stdio::readline::Readline {
            server_context: server_context.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("lib://flowruntime/stdio/stdin/stdin").chain_err(|| "Could not parse url")?,
        Native(Arc::new(stdio::stdin::Stdin {
            server_context: server_context.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("lib://flowruntime/stdio/stdout/stdout").chain_err(|| "Could not parse url")?,
        Native(Arc::new(stdio::stdout::Stdout {
            server_context: server_context.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("lib://flowruntime/stdio/stderr/stderr").chain_err(|| "Could not parse url")?,
        Native(Arc::new(stdio::stderr::Stderr { server_context })),
    );

    Ok(manifest)
}
