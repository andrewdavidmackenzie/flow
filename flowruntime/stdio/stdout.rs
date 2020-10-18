use std::sync::{Arc, Mutex};

use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use serde_json::Value;

use flowrlib::runtime_client::{Command, Response, RuntimeClient};

/// `Implementation` struct for the `Stdout` function
#[derive(Debug)]
pub struct Stdout {
    /// It holds a reference to the runtime client in order to write output
    pub client: Arc<Mutex<dyn RuntimeClient>>
}

impl Implementation for Stdout {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let input = &inputs[0];

        // Gain sole access to send to the client to avoid mixing output from other functions
        if let Ok(mut client) = self.client.lock() {
            match input {
                Value::Null => client.send_command(Command::EOF),
                Value::String(string) => client.send_command(Command::Stdout(string.to_string())),
                Value::Bool(boolean) => client.send_command(Command::Stdout(boolean.to_string())),
                Value::Number(number) => client.send_command(Command::Stdout(number.to_string())),
                Value::Array(_array) => client.send_command(Command::Stdout(input.to_string())),
                _ => Response::Error("Cannot Print this type".into())
            };
        }

        (None, RUN_AGAIN)
    }
}