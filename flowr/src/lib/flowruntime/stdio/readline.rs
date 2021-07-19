use std::sync::{Arc, Mutex};

use serde_json::Value;

use flowcore::{Implementation, RunAgain, DONT_RUN_AGAIN, RUN_AGAIN};

use crate::client_server::ServerConnection;
use crate::runtime_messages::{ClientMessage, ServerMessage};

/// `Implementation` struct for the `readline` function
pub struct Readline {
    /// It holds a reference to the runtime client in order to read input
    pub server_context: Arc<Mutex<ServerConnection<ServerMessage, ClientMessage>>>,
}

impl Implementation for Readline {
    fn run(&self, _inputs: &[Value]) -> (Option<Value>, RunAgain) {
        if let Ok(mut server) = self.server_context.lock() {
            return match server.send_and_receive_response(ServerMessage::GetLine) {
                Ok(ClientMessage::Line(contents)) => {
                    let mut output_map = serde_json::Map::new();
                    if let Ok(value) = serde_json::from_str(&contents) {
                        let _ = output_map.insert("json".into(), value);
                    };
                    output_map.insert("string".into(), Value::String(contents));
                    (Some(Value::Object(output_map)), RUN_AGAIN)
                }
                Ok(ClientMessage::GetLineEof) => {
                    let mut output_map = serde_json::Map::new();
                    output_map.insert("string".into(), Value::Null);
                    output_map.insert("json".into(), Value::Null);
                    (Some(Value::Object(output_map)), DONT_RUN_AGAIN)
                }
                _ => (None, DONT_RUN_AGAIN),
            };
        }
        (None, DONT_RUN_AGAIN)
    }
}
