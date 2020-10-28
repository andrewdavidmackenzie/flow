use std::sync::{Arc, Mutex};

use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use serde_json::Value;

use crate::client_server::RuntimeServerContext;
use crate::runtime::{Event, Response};

/// `Implementation` struct for the `Stderr` function
#[derive(Debug)]
pub struct Stderr {
    /// It holds a reference to the runtime client in order to write output
    pub server_context: Arc<Mutex<RuntimeServerContext>>
}

impl Implementation for Stderr {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let input = &inputs[0];

        if let Ok(mut server) = self.server_context.lock() {
            match input {
                Value::Null => server.send_event(Event::StderrEOF),
                Value::String(string) => server.send_event(Event::Stderr(string.to_string())),
                Value::Bool(boolean) => server.send_event(Event::Stderr(boolean.to_string())),
                Value::Number(number) => server.send_event(Event::Stderr(number.to_string())),
                Value::Array(_array) => server.send_event(Event::Stdout(input.to_string())),
                _ => Response::Error("Cannot Print this type".into())
            };
        }

        (None, RUN_AGAIN)
    }
}