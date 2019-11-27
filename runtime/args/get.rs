use std::sync::{Arc, Mutex};

use flow_impl::{DONT_RUN_AGAIN, Implementation, RunAgain};
use serde_json::{json, Value};

use super::super::runtime_client::{Command, Response, RuntimeClient};

/// `Implementation` struct for the `get` function
pub struct Get<'a> {
    pub client: Arc<Mutex<&'a dyn RuntimeClient>>
}

impl<'a> Implementation for Get<'a> {
    fn run(&self, mut _inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        if let Ok(client) = self.client.lock() {
            match client.send_command(Command::Args) {
                Response::Args(arg_vec) => {
                    let j_args = Some(json!(arg_vec));
                    return (j_args, DONT_RUN_AGAIN)
                },
                _ => return (None, DONT_RUN_AGAIN)
            };
        }
        (None, DONT_RUN_AGAIN)
    }
}