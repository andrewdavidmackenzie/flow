use std::sync::{Arc, Mutex};

use serde_json::Value;

use flow_impl::{DONT_RUN_AGAIN, Implementation, RUN_AGAIN, RunAgain};

use crate::client_server::RuntimeServerConnection;
use crate::runtime::{Event, Response};

/// `Implementation` struct for the `readline` function
pub struct Readline {
    /// It holds a reference to the runtime client in order to read input
    pub server_context: Arc<Mutex<RuntimeServerConnection>>
}

impl Implementation for Readline {
    fn run(&self, _inputs: &[Value]) -> (Option<Value>, RunAgain) {
        if let Ok(mut server) = self.server_context.lock() {
            return match server.send_event(Event::GetLine) {
                Ok(Response::Line(contents)) => {
                    let mut output_map = serde_json::Map::new();
                    output_map.insert("text".into(), Value::String(contents));
                    (Some(Value::Object(output_map)), RUN_AGAIN)
                },
                Ok(Response::GetLineEOF) => {
                    (Some(Value::Null), DONT_RUN_AGAIN)
                },
                _ => (None, DONT_RUN_AGAIN)
            };
        }
        (None, DONT_RUN_AGAIN)
    }
}