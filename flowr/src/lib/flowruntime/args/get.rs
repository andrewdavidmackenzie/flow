use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use flow_impl::{DONT_RUN_AGAIN, Implementation, RunAgain};

use crate::client_server::RuntimeServerConnection;
use crate::runtime::{Event, Response};

/// `Implementation` struct for the `get` function
pub struct Get {
    /// It holds a reference to the runtime client in order to Get the Args
    pub server_context: Arc<Mutex<RuntimeServerConnection>>
}

impl Implementation for Get {
    fn run(&self, mut _inputs: &[Value]) -> (Option<Value>, RunAgain) {
        if let Ok(mut guard) = self.server_context.lock() {
            return match guard.send_event(Event::GetArgs) {
                Ok(Response::Args(arg_vec)) => {
                    let mut output_map = serde_json::Map::new();

                    // Construct an array of args parsed into Json Values
                    let mut json_arg_vec: Vec<Value> = Vec::new();
                    for arg in &arg_vec {
                        if let Ok(json) = serde_json::from_str(arg) {
                            json_arg_vec.push(json);
                        }
                    }
                    // And add the array of Value at the "/json" route
                    let _ = output_map.insert("json".into(), json!(json_arg_vec));

                    // Add the array of (unparsed) text values of the args at "/text" route
                    output_map.insert("text".into(), json!(arg_vec));

                    (Some(Value::Object(output_map)), DONT_RUN_AGAIN)
                }
                _ => (None, DONT_RUN_AGAIN)
            };
        }
        (None, DONT_RUN_AGAIN)
    }
}