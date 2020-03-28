use std::sync::{Arc, Mutex};

use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use serde_json::Value;

use super::super::runtime_client::{Command, Response, RuntimeClient};

/// `Implementation` struct for the `file_write` function
#[derive(Debug)]
pub struct FileWrite {
    pub client: Arc<Mutex<dyn RuntimeClient>>
}

impl Implementation for FileWrite {
    fn run(&self, inputs: &Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let filename = &inputs[0][0];
        let bytes = &inputs[1][0];

        if let Ok(client) = self.client.lock() {
            match client.send_command(Command::Write(filename.to_string(),
                                                     bytes.as_str().unwrap().as_bytes().to_vec())) {
                Response::Ack => return (None, RUN_AGAIN),
                _ => return (None, RUN_AGAIN)
            }
        }

        (None, RUN_AGAIN)
    }
}