use std::sync::{Arc, Mutex};

use flow_impl::{DONT_RUN_AGAIN, Implementation, RUN_AGAIN, RunAgain};
use serde_json::Value;

use crate::runtime_client::{Event, Response, RuntimeClient};

/// `Implementation` struct for the `readline` function
#[derive(Debug)]
pub struct Readline {
    /// It holds a reference to the runtime client in order to read input
    pub client: Arc<Mutex<dyn RuntimeClient>>
}

impl Implementation for Readline {
    fn run(&self, _inputs: &[Value]) -> (Option<Value>, RunAgain) {
        if let Ok(mut client) = self.client.lock() {
            return match client.send_event(Event::GetLine) {
                Response::Line(contents) => (Some(Value::String(contents)), RUN_AGAIN),
               Response::GetLineEOF => (Some(Value::Null), DONT_RUN_AGAIN),
                _ => (None, DONT_RUN_AGAIN)
            }
        }
        (None, DONT_RUN_AGAIN)
    }
}