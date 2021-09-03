use std::sync::{Arc, Mutex};

use serde_json::Value;

use flowcore::{Implementation, RunAgain, RUN_AGAIN};

use crate::client_server::ServerConnection;
use crate::errors::*;
use crate::runtime_messages::{ClientMessage, ServerMessage};

/// `Implementation` struct for the `Stderr` function
pub struct Stderr {
    /// It holds a reference to the runtime client in order to write output
    pub server_connection: Arc<Mutex<ServerConnection<ServerMessage, ClientMessage>>>,
}

impl Implementation for Stderr {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let input = &inputs[0];

        if let Ok(mut server) = self.server_connection.lock() {
            let _: Result<ClientMessage> = match input {
                Value::Null => server.send_and_receive_response(ServerMessage::StderrEof),
                Value::String(string) => {
                    server.send_and_receive_response(ServerMessage::Stderr(string.to_string()))
                }
                Value::Bool(boolean) => {
                    server.send_and_receive_response(ServerMessage::Stderr(boolean.to_string()))
                }
                Value::Number(number) => {
                    server.send_and_receive_response(ServerMessage::Stderr(number.to_string()))
                }
                Value::Array(_array) => {
                    server.send_and_receive_response(ServerMessage::Stdout(input.to_string()))
                }
                Value::Object(_obj) => {
                    server.send_and_receive_response(ServerMessage::Stdout(input.to_string()))
                }
            };
        }

        (None, RUN_AGAIN)
    }
}
