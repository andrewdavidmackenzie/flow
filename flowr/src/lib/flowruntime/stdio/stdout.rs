use std::sync::{Arc, Mutex};

use serde_json::Value;

use flowcore::{Implementation, RunAgain, RUN_AGAIN};

use crate::client_server::RuntimeServerConnection;
use crate::runtime::Event;

/// `Implementation` struct for the `Stdout` function
pub struct Stdout {
    /// It holds a reference to the runtime client in order to write output
    pub server_context: Arc<Mutex<RuntimeServerConnection>>,
}

impl Implementation for Stdout {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let input = &inputs[0];

        // Gain sole access to send to the client to avoid mixing output from other functions
        if let Ok(mut server) = self.server_context.lock() {
            let _ = match input {
                Value::Null => server.send_event(Event::StdoutEof),
                Value::String(string) => server.send_event(Event::Stdout(string.to_string())),
                Value::Bool(boolean) => server.send_event(Event::Stdout(boolean.to_string())),
                Value::Number(number) => server.send_event(Event::Stdout(number.to_string())),
                Value::Array(_array) => server.send_event(Event::Stdout(input.to_string())),
                Value::Object(_obj) => server.send_event(Event::Stdout(input.to_string())),
            };
        }

        (None, RUN_AGAIN)
    }
}
