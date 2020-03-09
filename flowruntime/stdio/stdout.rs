use std::sync::{Arc, Mutex};

use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use serde_json::Value;

use super::super::runtime_client::{Command, Response, RuntimeClient};

/// `Implementation` struct for the `Stdout` function
#[derive(Debug)]
pub struct Stdout {
    pub client: Arc<Mutex<dyn RuntimeClient>>
}

impl Implementation for Stdout {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let input = inputs.remove(0).remove(0);

        if let Ok(client) = self.client.lock() {
            match input {
                Value::String(string) => client.send_command(Command::Stdout(format!("{}", string))),
                Value::Bool(boolean) => client.send_command(Command::Stdout(boolean.to_string())),
                Value::Number(number) => client.send_command(Command::Stdout(number.to_string())),
                Value::Array(array) => {
                    for entry in array {
                        client.send_command(Command::Stdout(format!("{}", entry)));
                    }
                    Response::Ack
                }
                _ => Response::Error("Cannot Print this type".into())
            };
        }

        (None, RUN_AGAIN)
    }
}