use std::sync::{Arc, Mutex};

use url::Url;

use flowcore::model::lib_manifest::ImplementationLocator::Native;
use flowcore::model::lib_manifest::LibraryManifest;
use flowcore::model::metadata::MetaData;

use crate::errors::*;
use crate::ServerConnection;

/// Module of context functions for Cli Flowr Runner

mod args;
mod file;
mod image;
mod stdio;

/// Return a `LibraryManifest` for the context functions
pub fn get_manifest(
    server_connection: Arc<Mutex<ServerConnection>>,
) -> Result<LibraryManifest> {
    let metadata = MetaData {
        name: "context".into(),
        version: "0.1.0".into(),
        description: "context functions for Flowr Cli Runner".into(),
        authors: vec!["Andrew Mackenzie".to_string()],
    };
    let lib_url = Url::parse("lib://context")?;
    let mut manifest = LibraryManifest::new(lib_url, metadata);

    manifest.locators.insert(
        Url::parse("context://args/get")?,
        Native(Arc::new(args::get::Get {
            server_connection: server_connection.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("context://file/file_write")
            .chain_err(|| "Could not parse url")?,
        Native(Arc::new(file::file_write::FileWrite {
            server_connection: server_connection.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("context://image/image_buffer")
            .chain_err(|| "Could not parse url")?,
        Native(Arc::new(image::image_buffer::ImageBuffer {
            server_connection: server_connection.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("context://stdio/readline")
            .chain_err(|| "Could not parse url")?,
        Native(Arc::new(stdio::readline::Readline {
            server_connection: server_connection.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("context://stdio/stdin").chain_err(|| "Could not parse url")?,
        Native(Arc::new(stdio::stdin::Stdin {
            server_connection: server_connection.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("context://stdio/stdout").chain_err(|| "Could not parse url")?,
        Native(Arc::new(stdio::stdout::Stdout {
            server_connection: server_connection.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("context://stdio/stderr").chain_err(|| "Could not parse url")?,
        Native(Arc::new(stdio::stderr::Stderr { server_connection })),
    );

    Ok(manifest)
}