use std::sync::{Arc, Mutex};

use serde_json::Value;

use flowcore::{Implementation, RunAgain, RUN_AGAIN};

use crate::client_server::RuntimeServerConnection;
use crate::runtime_messages::ServerMessage;

/// `Implementation` struct for the `Stderr` function
pub struct Stderr {
    /// It holds a reference to the runtime client in order to write output
    pub server_context: Arc<Mutex<RuntimeServerConnection>>,
}

impl Implementation for Stderr {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let input = &inputs[0];

        if let Ok(mut server) = self.server_context.lock() {
            let _ = match input {
                Value::Null => server.send_message(ServerMessage::StderrEof),
                Value::String(string) => {
                    server.send_message(ServerMessage::Stderr(string.to_string()))
                }
                Value::Bool(boolean) => {
                    server.send_message(ServerMessage::Stderr(boolean.to_string()))
                }
                Value::Number(number) => {
                    server.send_message(ServerMessage::Stderr(number.to_string()))
                }
                Value::Array(_array) => {
                    server.send_message(ServerMessage::Stdout(input.to_string()))
                }
                Value::Object(_obj) => {
                    server.send_message(ServerMessage::Stdout(input.to_string()))
                }
            };
        }

        (None, RUN_AGAIN)
    }
}
