//! Module of context functions for Cli Flowr Runner

use std::sync::Arc;

use flowcore::errors::{Result, ResultExt};
use flowcore::model::lib_manifest::ImplementationLocator::Native;
use flowcore::model::lib_manifest::LibraryManifest;
use flowcore::model::metadata::MetaData;
use url::Url;

// Re-export from library
pub use flowrlib::context_io::{ContextIO, ContextRequest};

mod args;
mod file;
mod image;
mod stdio;
mod time;

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
    manifest.locators.insert(
        Url::parse("context://time/get").chain_err(|| "Could not parse url")?,
        Native(Arc::new(time::get::Get::new())),
    );

    Ok(manifest)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use super::{ContextIO, ContextRequest};
    use flowrlib::client_protocol::{ClientMessage, CoordinatorMessage};

    /// `send_and_receive` should deliver the request through the non-blocking channel.
    #[test]
    fn send_and_receive_uses_nonblocking_channel() {
        let (tx, rx) = std::sync::mpsc::channel::<ContextRequest>();
        let (blocking_tx, blocking_rx) = std::sync::mpsc::channel::<ContextRequest>();
        let context_io = ContextIO::new(tx, blocking_tx);

        // Spawn because send_and_receive blocks waiting for the response
        let handle =
            std::thread::spawn(move || context_io.send_and_receive(CoordinatorMessage::GetArgs));

        // The request must arrive on the non-blocking receiver
        let req = rx.recv().expect("Expected request on non-blocking channel");
        assert!(
            matches!(req.message, CoordinatorMessage::GetArgs),
            "Expected GetArgs on non-blocking channel"
        );
        // Nothing should arrive on the blocking receiver
        assert!(
            blocking_rx.try_recv().is_err(),
            "Blocking channel should be empty"
        );
        // Respond so the thread can finish
        req.response_tx.unwrap().send(ClientMessage::Ack).unwrap();
        let result = handle.join().unwrap();
        assert!(result.is_ok());
    }

    /// `send_and_receive_blocking` should deliver the request through the blocking channel.
    #[test]
    fn send_and_receive_blocking_uses_blocking_channel() {
        let (tx, rx) = std::sync::mpsc::channel::<ContextRequest>();
        let (blocking_tx, blocking_rx) = std::sync::mpsc::channel::<ContextRequest>();
        let context_io = ContextIO::new(tx, blocking_tx);

        let handle = std::thread::spawn(move || {
            context_io.send_and_receive_blocking(CoordinatorMessage::GetLine("prompt".into()))
        });

        // The request must arrive on the blocking receiver
        let req = blocking_rx
            .recv()
            .expect("Expected request on blocking channel");
        assert!(
            matches!(req.message, CoordinatorMessage::GetLine(_)),
            "Expected GetLine on blocking channel"
        );
        // Nothing should arrive on the non-blocking receiver
        assert!(
            rx.try_recv().is_err(),
            "Non-blocking channel should be empty"
        );
        req.response_tx
            .unwrap()
            .send(ClientMessage::Line("hello".into()))
            .unwrap();
        let result = handle
            .join()
            .unwrap()
            .expect("send_and_receive_blocking failed");
        assert!(matches!(result, ClientMessage::Line(_)));
    }

    /// When the non-blocking channel is disconnected, `send_and_receive` returns an error.
    #[test]
    fn send_and_receive_error_on_disconnected_channel() {
        let (tx, rx) = std::sync::mpsc::channel::<ContextRequest>();
        let (blocking_tx, _blocking_rx) = std::sync::mpsc::channel::<ContextRequest>();
        let context_io = ContextIO::new(tx, blocking_tx);
        drop(rx); // disconnect the receiver
        let result = context_io.send_and_receive(CoordinatorMessage::GetArgs);
        assert!(result.is_err());
    }

    /// When the blocking channel is disconnected, `send_and_receive_blocking` returns an error.
    #[test]
    fn send_and_receive_blocking_error_on_disconnected_channel() {
        let (tx, _rx) = std::sync::mpsc::channel::<ContextRequest>();
        let (blocking_tx, blocking_rx) = std::sync::mpsc::channel::<ContextRequest>();
        let context_io = ContextIO::new(tx, blocking_tx);
        drop(blocking_rx); // disconnect the receiver
        let result =
            context_io.send_and_receive_blocking(CoordinatorMessage::GetLine(String::new()));
        assert!(result.is_err());
    }

    /// `send_no_reply` delivers through the non-blocking channel without expecting a response.
    #[test]
    fn send_no_reply_uses_nonblocking_channel() {
        let (tx, rx) = std::sync::mpsc::channel::<ContextRequest>();
        let (blocking_tx, blocking_rx) = std::sync::mpsc::channel::<ContextRequest>();
        let context_io = ContextIO::new(tx, blocking_tx);

        context_io
            .send_no_reply(CoordinatorMessage::Stdout("hello".into()))
            .expect("send_no_reply should succeed");

        let req = rx.recv().expect("Expected request on non-blocking channel");
        assert!(
            matches!(req.message, CoordinatorMessage::Stdout(_)),
            "Expected Stdout on non-blocking channel"
        );
        assert!(
            req.response_tx.is_none(),
            "send_no_reply should not set response_tx"
        );
        assert!(
            blocking_rx.try_recv().is_err(),
            "Blocking channel should be empty"
        );
    }
}
