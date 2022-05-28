use std::sync::{Arc, Mutex};

use url::Url;

use flowcore::errors::*;
use flowcore::model::lib_manifest::ImplementationLocator::Native;
use flowcore::model::lib_manifest::LibraryManifest;
use flowcore::model::metadata::MetaData;

use crate::ServerConnection;

/// Module of context functions for Cli Flowr Runner

mod args;
mod file;
mod image;
mod stdio;
// Test helper functions
pub(crate) mod test_helper;
/// runtime_messages is the enum for the different messages sent back and fore between the client
/// and server implementation of the CLI context functions
pub mod runtime_messages;
/// 'debug' defines structs passed between the Server and the Client regarding debug events
/// and client responses to them
#[cfg(feature = "debugger")]
pub mod debug_server_message;
/// message_queue implementation of the communications between the runtime client, debug client and
/// the runtime server and debug server.
pub mod client_server;
#[cfg(feature = "debugger")]
pub mod cli_debug_client;
pub mod cli_runtime_client;
#[cfg(feature = "debugger")]
pub mod cli_debug_server;
pub mod cli_runtime_server;

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
