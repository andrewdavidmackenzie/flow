//! `context` module defines a set of functions for a flow program to interact with the host system,
//! such as file io, stdio etc.

use std::sync::{Arc, Mutex};

use url::Url;

use flowcore::model::metadata::MetaData;
use flowcore::model::lib_manifest::{ImplementationLocator::Native, LibraryManifest};

use crate::client_server::ServerConnection;
use crate::errors::*;

/// `args` is a module to interact with a programs arguments
pub mod args;
/// `file` is a module to interact with the file system
pub mod file;
/// `image` is a module to interact with images
pub mod image;
/// `stdio` is a module to interact with standard IO
pub mod stdio;
// Test helper functions
pub(crate) mod test_helper;

/// Return a `LibraryManifest` for the run-time functions
pub fn get_manifest(
    server_connection: Arc<Mutex<ServerConnection>>,
) -> Result<LibraryManifest> {
    let metadata = MetaData {
        name: "context".into(),
        version: "0.1.0".into(),
        description: "Flow Runtime functions".into(),
        authors: vec!["Andrew Mackenzie".to_string()],
    };
    let lib_url = Url::parse("lib://context")?;
    let mut manifest = LibraryManifest::new(lib_url, metadata);

    manifest.locators.insert(
        Url::parse("lib://context/args/get/get")
            .chain_err(|| "Could not parse url")
            .chain_err(|| "Could not parse url")?,
        Native(Arc::new(args::get::Get {
            server_connection: server_connection.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("lib://context/file/file_write/file_write")
            .chain_err(|| "Could not parse url")?,
        Native(Arc::new(file::file_write::FileWrite {
            server_connection: server_connection.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("lib://context/image/image_buffer/image_buffer")
            .chain_err(|| "Could not parse url")?,
        Native(Arc::new(image::image_buffer::ImageBuffer {
            server_connection: server_connection.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("lib://context/stdio/readline/readline")
            .chain_err(|| "Could not parse url")?,
        Native(Arc::new(stdio::readline::Readline {
            server_connection: server_connection.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("lib://context/stdio/stdin/stdin").chain_err(|| "Could not parse url")?,
        Native(Arc::new(stdio::stdin::Stdin {
            server_connection: server_connection.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("lib://context/stdio/stdout/stdout").chain_err(|| "Could not parse url")?,
        Native(Arc::new(stdio::stdout::Stdout {
            server_connection: server_connection.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("lib://context/stdio/stderr/stderr").chain_err(|| "Could not parse url")?,
        Native(Arc::new(stdio::stderr::Stderr { server_connection })),
    );

    Ok(manifest)
}
