use std::sync::{Arc, Mutex};

use flow_impl::{DONT_RUN_AGAIN, Implementation, RUN_AGAIN, RunAgain};
use serde_json::Value;

use crate::client_server::RuntimeServerContext;
use crate::runtime::{Event, Response};

/// `Implementation` struct for the `Stdin` function
#[derive(Debug)]
pub struct Stdin {
    /// It holds a reference to the runtime client in order to read input
    pub server_context: Arc<Mutex<RuntimeServerContext>>
}

impl Implementation for Stdin {
    fn run(&self, _inputs: &[Value]) -> (Option<Value>, RunAgain) {
        if let Ok(mut server) = self.server_context.lock() {
            return match server.send_event(Event::GetStdin) {
                Response::Stdin(contents) => (Some(Value::String(contents)), RUN_AGAIN),
                Response::GetStdinEOF => (Some(Value::Null), DONT_RUN_AGAIN),
                _ => (None, DONT_RUN_AGAIN)
            }
        }
        (None, DONT_RUN_AGAIN)
    }
}