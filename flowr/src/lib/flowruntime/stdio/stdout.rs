use std::sync::{Arc, Mutex};

use serde_json::Value;

use flow_impl::{Implementation, RUN_AGAIN, RunAgain};

use crate::client_server::RuntimeServerConnection;
use crate::runtime::Event;

/// `Implementation` struct for the `Stdout` function
pub struct Stdout {
    /// It holds a reference to the runtime client in order to write output
    pub server_context: Arc<Mutex<RuntimeServerConnection>>
}

impl Implementation for Stdout {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let input = &inputs[0];

        // Gain sole access to send to the client to avoid mixing output from other functions
        if let Ok(mut server) = self.server_context.lock() {
            let _ = match input {
                Value::Null => server.send_event(Event::StdoutEOF),
                Value::String(string) => server.send_event(Event::Stdout(string.to_string())),
                Value::Bool(boolean) => server.send_event(Event::Stdout(boolean.to_string())),
                Value::Number(number) => server.send_event(Event::Stdout(number.to_string())),
                Value::Array(_array) => server.send_event(Event::Stdout(input.to_string())),
                _ => return (None, RUN_AGAIN)
            };
        }

        (None, RUN_AGAIN)
    }
}