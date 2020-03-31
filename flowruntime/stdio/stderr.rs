use std::sync::{Arc, Mutex};

use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use serde_json::Value;

use super::super::runtime_client::{Command, Response, RuntimeClient};

/// `Implementation` struct for the `Stderr` function
#[derive(Debug)]
pub struct Stderr {
    pub client: Arc<Mutex<dyn RuntimeClient>>
}

impl Implementation for Stderr {
    fn run(&self, inputs: &Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let input = &inputs[0][0];

        if let Ok(client) = self.client.lock() {
            match input {
                Value::Null => client.send_command(Command::Stdout("Null".into())),
                Value::String(string) => client.send_command(Command::Stderr(format!("{}", string))),
                Value::Bool(boolean) => client.send_command(Command::Stderr(boolean.to_string())),
                Value::Number(number) => client.send_command(Command::Stderr(number.to_string())),
                Value::Array(_array) => client.send_command(Command::Stdout(input.to_string())),
                _ => Response::Error("Cannot Print this type".into())
            };
        }

        (None, RUN_AGAIN)
    }
}