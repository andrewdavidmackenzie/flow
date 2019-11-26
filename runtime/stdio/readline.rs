use std::sync::{Arc, Mutex};

use flow_impl::{DONT_RUN_AGAIN, Implementation, RUN_AGAIN, RunAgain};
use serde_json::Value;

use super::super::runtime_client::{Command, Response, RuntimeClient};

/// `Implementation` struct for the `readline` function
pub struct Readline<'a> {
    pub client: Arc<Mutex<&'a dyn RuntimeClient>>
}

impl<'a> Implementation for Readline<'a> {
    fn run(&self, _inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        if let Ok(client) = self.client.lock() {
            match client.send_command(Command::Readline) {
                Response::Readline(contents) => return (Some(Value::String(contents)), RUN_AGAIN),
                _ => return (None, DONT_RUN_AGAIN)
            }
        }
        (None, DONT_RUN_AGAIN)
    }
}