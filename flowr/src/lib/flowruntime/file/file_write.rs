use std::sync::{Arc, Mutex};

use serde_json::Value;

use flowcore::{Implementation, RunAgain, RUN_AGAIN};

use crate::client_server::ServerConnection;
use crate::runtime_messages::{ClientMessage, ServerMessage};

/// `Implementation` struct for the `file_write` function
pub struct FileWrite {
    /// It holds a reference to the runtime client in order to get file contents
    pub server_context: Arc<Mutex<ServerConnection>>,
}

impl Implementation for FileWrite {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let filename = &inputs[0];
        let bytes = &inputs[1];

        if let Ok(mut server) = self.server_context.lock() {
            match bytes.as_str() {
                Some(string) => {
                    return match server.send_message(ServerMessage::Write(
                        filename.to_string(),
                        string.as_bytes().to_vec(),
                    )) {
                        Ok(ClientMessage::Ack) => (None, RUN_AGAIN),
                        _ => (None, RUN_AGAIN),
                    }
                }
                None => return (None, RUN_AGAIN),
            }
        }

        (None, RUN_AGAIN)
    }
}
