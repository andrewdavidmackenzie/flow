//! Module of context functions for Cli Flowr Runner

use std::sync::mpsc;
use std::sync::Arc;

use flowcore::errors::{Result, ResultExt};
use flowcore::model::lib_manifest::ImplementationLocator::Native;
use flowcore::model::lib_manifest::LibraryManifest;
use flowcore::model::metadata::MetaData;
use url::Url;

use crate::cli::coordinator_message::{ClientMessage, CoordinatorMessage};

mod args;
mod file;
mod image;
mod stdio;

/// A request sent from a context function to the ZMQ bridge thread.
pub struct ContextRequest {
    /// The message to send to the client
    pub message: CoordinatorMessage,
    /// If `Some`, the bridge sends the client's response back on this channel.
    /// If `None`, the message is fire-and-forget (no response expected).
    pub response_tx: Option<mpsc::Sender<ClientMessage>>,
}

/// Channel-based IO handle for context functions.
///
/// Uses two separate channels: one for blocking IO (readline, stdin) and one
/// for non-blocking IO (stdout, stderr, file, image, args). Each channel has
/// its own bridge thread and ZMQ socket, allowing non-blocking IO to proceed
/// while blocking IO is waiting for user input.
#[derive(Clone)]
pub struct ContextIO {
    blocking_tx: mpsc::Sender<ContextRequest>,
    nonblocking_tx: mpsc::Sender<ContextRequest>,
}

impl ContextIO {
    /// Create a new `ContextIO` with separate channels for blocking and non-blocking IO.
    pub fn new(
        blocking_tx: mpsc::Sender<ContextRequest>,
        nonblocking_tx: mpsc::Sender<ContextRequest>,
    ) -> Self {
        ContextIO {
            blocking_tx,
            nonblocking_tx,
        }
    }

    /// Send a message on the blocking channel and wait for the response.
    /// Used by readline and stdin which may block indefinitely.
    pub fn send_blocking(&self, message: CoordinatorMessage) -> Result<ClientMessage> {
        send_and_receive(&self.blocking_tx, message)
    }

    /// Send a message on the non-blocking channel and wait for the response.
    /// Used by stdout, stderr, file, image, args — these return quickly.
    pub fn send_nonblocking(&self, message: CoordinatorMessage) -> Result<ClientMessage> {
        send_and_receive(&self.nonblocking_tx, message)
    }
}

fn send_and_receive(
    tx: &mpsc::Sender<ContextRequest>,
    message: CoordinatorMessage,
) -> Result<ClientMessage> {
    let (response_tx, response_rx) = mpsc::channel();
    tx.send(ContextRequest {
        message,
        response_tx: Some(response_tx),
    })
    .map_err(|e| format!("Could not send to bridge: {e}"))?;
    response_rx
        .recv()
        .map_err(|e| format!("Could not receive from bridge: {e}").into())
}

/// Return a `LibraryManifest` for the context functions
pub fn get_manifest(context_io: ContextIO) -> Result<LibraryManifest> {
    let metadata = MetaData {
        name: "context".into(),
        version: "0.1.0".into(),
        description: "context functions for Flowr Cli Runner".into(),
        authors: vec!["Andrew Mackenzie".to_string()],
    };
    let lib_url = Url::parse("context://")?;
    let mut manifest = LibraryManifest::new(lib_url, metadata);

    manifest.locators.insert(
        Url::parse("context://args/get")?,
        Native(Arc::new(args::get::Get {
            context_io: context_io.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("context://file/file_write").chain_err(|| "Could not parse url")?,
        Native(Arc::new(file::file_write::FileWrite {
            context_io: context_io.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("context://file/file_read").chain_err(|| "Could not parse url")?,
        Native(Arc::new(file::file_read::FileRead {
            context_io: context_io.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("context://image/image_buffer").chain_err(|| "Could not parse url")?,
        Native(Arc::new(image::image_buffer::ImageBuffer {
            context_io: context_io.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("context://image/image_read").chain_err(|| "Could not parse url")?,
        Native(Arc::new(image::image_read::ImageRead {
            context_io: context_io.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("context://image/image_write").chain_err(|| "Could not parse url")?,
        Native(Arc::new(image::image_write::ImageWrite {
            context_io: context_io.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("context://stdio/readline").chain_err(|| "Could not parse url")?,
        Native(Arc::new(stdio::readline::Readline {
            context_io: context_io.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("context://stdio/stdin").chain_err(|| "Could not parse url")?,
        Native(Arc::new(stdio::stdin::Stdin {
            context_io: context_io.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("context://stdio/stdout").chain_err(|| "Could not parse url")?,
        Native(Arc::new(stdio::stdout::Stdout {
            context_io: context_io.clone(),
        })),
    );
    manifest.locators.insert(
        Url::parse("context://stdio/stderr").chain_err(|| "Could not parse url")?,
        Native(Arc::new(stdio::stderr::Stderr { context_io })),
    );

    Ok(manifest)
}
