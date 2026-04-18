//! Context functions for flowedit's flow runner.
//!
//! Adapted from flowrgui — these implementations communicate with the GUI
//! via ZMQ messages through the coordinator connection.

use std::sync::{Arc, Mutex};

use flowcore::errors::{Result, ResultExt};
use flowcore::model::lib_manifest::ImplementationLocator::Native;
use flowcore::model::lib_manifest::LibraryManifest;
use flowcore::model::metadata::MetaData;
use url::Url;

use crate::coordinator::CoordinatorConnection;

pub(crate) mod args;
pub(crate) mod file;
pub(crate) mod image;
pub(crate) mod stdio;

/// Return a `LibraryManifest` for the context functions
pub(crate) fn get_manifest(
    server_connection: Arc<Mutex<CoordinatorConnection>>,
) -> Result<LibraryManifest> {
    let metadata = MetaData {
        name: "context".into(),
        version: "0.1.0".into(),
        description: "context functions for flowedit".into(),
        authors: vec!["Andrew Mackenzie".to_string()],
    };
    let lib_url = Url::parse("context://")?;
    let mut manifest = LibraryManifest::new(lib_url, metadata);

    manifest.locators.insert(
        Url::parse("context://args/get")?,
        Native(Arc::new(args::Get {
            server_connection: server_connection.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("context://file/file_write").chain_err(|| "Could not parse url")?,
        Native(Arc::new(file::FileWrite {
            server_connection: server_connection.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("context://file/file_read").chain_err(|| "Could not parse url")?,
        Native(Arc::new(file::FileRead {
            server_connection: server_connection.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("context://image/image_buffer").chain_err(|| "Could not parse url")?,
        Native(Arc::new(image::ImageBuffer {
            server_connection: server_connection.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("context://stdio/readline").chain_err(|| "Could not parse url")?,
        Native(Arc::new(stdio::Readline {
            server_connection: server_connection.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("context://stdio/stdin").chain_err(|| "Could not parse url")?,
        Native(Arc::new(stdio::Stdin {
            server_connection: server_connection.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("context://stdio/stdout").chain_err(|| "Could not parse url")?,
        Native(Arc::new(stdio::Stdout {
            server_connection: server_connection.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("context://stdio/stderr").chain_err(|| "Could not parse url")?,
        Native(Arc::new(stdio::Stderr { server_connection })),
    );

    Ok(manifest)
}
