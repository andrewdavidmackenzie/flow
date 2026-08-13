//! Channel-based IO handle for context functions and context proxying.
//!
//! `ContextIO` provides a way for context function implementations to
//! communicate with the client (for stdio, file, image operations) through
//! the coordinator's bridge threads. It is also used by the context proxy
//! to relay context requests from delegated sub-flows back to the origin.

use std::sync::mpsc;

use flowcore::errors::Result;

use crate::client_protocol::{ClientMessage, CoordinatorMessage};

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
    #[must_use]
    pub fn new(
        tx: mpsc::Sender<ContextRequest>,
        blocking_tx: mpsc::Sender<ContextRequest>,
    ) -> Self {
        ContextIO { tx, blocking_tx }
    }

    /// Send a message on the non-blocking channel and wait for the client's response.
    ///
    /// # Errors
    ///
    /// Returns an error if the message cannot be sent or the response cannot be received.
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
    ///
    /// # Errors
    ///
    /// Returns an error if the message cannot be sent or the response cannot be received.
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

    /// Send a message without waiting for a response (fire-and-forget).
    /// The bridge thread still completes the ZMQ round-trip.
    ///
    /// # Errors
    ///
    /// Returns an error if the message cannot be sent.
    pub fn send_no_reply(&self, message: CoordinatorMessage) -> Result<()> {
        self.tx
            .send(ContextRequest {
                message,
                response_tx: None,
            })
            .map_err(|e| format!("Could not send to bridge: {e}").into())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod test {
    use super::*;

    #[test]
    fn send_and_receive_roundtrip() {
        let (tx, rx) = mpsc::channel();
        let (blocking_tx, _blocking_rx) = mpsc::channel();
        let context_io = ContextIO::new(tx, blocking_tx);

        // Spawn a thread to simulate the bridge
        std::thread::spawn(move || {
            let request = rx.recv().unwrap();
            assert!(matches!(request.message, CoordinatorMessage::GetArgs));
            if let Some(response_tx) = request.response_tx {
                response_tx
                    .send(ClientMessage::Args(vec!["test".into()]))
                    .unwrap();
            }
        });

        let result = context_io.send_and_receive(CoordinatorMessage::GetArgs);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), ClientMessage::Args(_)));
    }

    #[test]
    fn send_no_reply_succeeds() {
        let (tx, _rx) = mpsc::channel();
        let (blocking_tx, _blocking_rx) = mpsc::channel();
        let context_io = ContextIO::new(tx, blocking_tx);

        let result = context_io.send_no_reply(CoordinatorMessage::StdoutEof);
        assert!(result.is_ok());
    }
}
