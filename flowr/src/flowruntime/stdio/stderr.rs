use std::sync::{Arc, Mutex};

use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use serde_json::Value;

use crate::client_server::RuntimeClient;
use crate::runtime::{Event, Response};

/// `Implementation` struct for the `Stderr` function
#[derive(Debug)]
pub struct Stderr {
    /// It holds a reference to the runtime client in order to write output
    pub client: Arc<Mutex<dyn RuntimeClient>>
}

impl Implementation for Stderr {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let input = &inputs[0];

        if let Ok(mut client) = self.client.lock() {
            match input {
                Value::Null => client.send_event(Event::StderrEOF),
                Value::String(string) => client.send_event(Event::Stderr(string.to_string())),
                Value::Bool(boolean) => client.send_event(Event::Stderr(boolean.to_string())),
                Value::Number(number) => client.send_event(Event::Stderr(number.to_string())),
                Value::Array(_array) => client.send_event(Event::Stdout(input.to_string())),
                _ => Response::Error("Cannot Print this type".into())
            };
        }

        (None, RUN_AGAIN)
    }
}