//! Module of context functions for Gui Flowr Runner

use std::sync::mpsc;
use std::sync::Arc;

use flowcore::errors::{Result, ResultExt};
use flowcore::model::lib_manifest::ImplementationLocator::Native;
use flowcore::model::lib_manifest::LibraryManifest;
use flowcore::model::metadata::MetaData;
use url::Url;

use crate::gui::client_message::ClientMessage;
use crate::gui::coordinator_message::CoordinatorMessage;

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
/// Uses two channels: one for non-blocking IO (stdout, stderr, file, image, args)
/// and one for blocking IO (readline, stdin). This allows blocking IO to be
/// handled on a separate ZMQ socket so it doesn't block non-blocking IO.
#[derive(Clone)]
pub struct ContextIO {
    /// Channel for non-blocking context function requests (stdout, stderr, etc.)
    tx: mpsc::Sender<ContextRequest>,
    /// Channel for blocking context function requests (readline, stdin)
    blocking_tx: mpsc::Sender<ContextRequest>,
}

impl ContextIO {
    /// Create a new `ContextIO` backed by the given channel senders.
    pub fn new(
        tx: mpsc::Sender<ContextRequest>,
        blocking_tx: mpsc::Sender<ContextRequest>,
    ) -> Self {
        ContextIO { tx, blocking_tx }
    }

    /// Send a message on the non-blocking channel and wait for the client's response.
    pub fn send_and_receive(&self, message: CoordinatorMessage) -> Result<ClientMessage> {
        let (response_tx, response_rx) = mpsc::channel();
        self.tx
            .send(ContextRequest {
                message,
                response_tx: Some(response_tx),
            })
            .map_err(|e| format!("Could not send to bridge: {e}"))?;
        response_rx
            .recv()
            .map_err(|e| format!("Could not receive from bridge: {e}").into())
    }

    /// Send a message on the blocking IO channel and wait for the client's response.
    /// Used by context functions that may block for user input (readline, stdin).
    pub fn send_and_receive_blocking(&self, message: CoordinatorMessage) -> Result<ClientMessage> {
        let (response_tx, response_rx) = mpsc::channel();
        self.blocking_tx
            .send(ContextRequest {
                message,
                response_tx: Some(response_tx),
            })
            .map_err(|e| format!("Could not send to blocking bridge: {e}"))?;
        response_rx
            .recv()
            .map_err(|e| format!("Could not receive from blocking bridge: {e}").into())
    }
}

/// Return a `LibraryManifest` for the context functions
pub fn get_manifest(context_io: ContextIO) -> Result<LibraryManifest> {
    let metadata = MetaData {
        name: "context".into(),
        version: "0.1.0".into(),
        description: "context functions for Gui Flowr Runner".into(),
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
