use std::sync::{Arc, Mutex};

use flow_impl::{DONT_RUN_AGAIN, Implementation, RunAgain};
use serde_json::{json, Value};

use flowrlib::runtime_client::{Command, Response, RuntimeClient};

/// `Implementation` struct for the `get` function
#[derive(Debug)]
pub struct Get {
    /// It holds a reference to the runtime client in order to Get the Args
    pub client: Arc<Mutex<dyn RuntimeClient>>
}

impl Implementation for Get {
    fn run(&self, mut _inputs: &[Value]) -> (Option<Value>, RunAgain) {
        if let Ok(mut client) = self.client.lock() {
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