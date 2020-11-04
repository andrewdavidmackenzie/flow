use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use flow_impl::{DONT_RUN_AGAIN, Implementation, RunAgain};

use crate::client_server::RuntimeServerContext;
use crate::runtime::{Event, Response};

/// `Implementation` struct for the `get` function
pub struct Get {
    /// It holds a reference to the runtime client in order to Get the Args
    pub server_context: Arc<Mutex<RuntimeServerContext>>
}

impl Implementation for Get {
    fn run(&self, mut _inputs: &[Value]) -> (Option<Value>, RunAgain) {
        if let Ok(mut guard) = self.server_context.lock() {
            return match guard.send_event(Event::GetArgs) {
                Ok(Response::Args(arg_vec)) => {
                    let j_args = Some(json!(arg_vec));
                    (j_args, DONT_RUN_AGAIN)
                },
                _ => (None, DONT_RUN_AGAIN)
            };
        }
        (None, DONT_RUN_AGAIN)
    }
}