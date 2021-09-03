use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use flowcore::{Implementation, RunAgain, DONT_RUN_AGAIN};

use crate::client_server::ServerConnection;
use crate::runtime_messages::{ClientMessage, ServerMessage};

/// `Implementation` struct for the `get` function
pub struct Get {
    /// It holds a reference to the runtime client in order to Get the Args
    pub server_connection: Arc<Mutex<ServerConnection<ServerMessage, ClientMessage>>>,
}

impl Implementation for Get {
    fn run(&self, mut _inputs: &[Value]) -> (Option<Value>, RunAgain) {
        if let Ok(mut guard) = self.server_connection.lock() {
            return match guard.send_and_receive_response(ServerMessage::GetArgs) {
                Ok(ClientMessage::Args(arg_vec)) => {
                    let mut output_map = serde_json::Map::new();

                    // Construct an array of args parsed into Json Values
                    let mut json_arg_vec: Vec<Value> = Vec::new();
                    for arg in &arg_vec {
                        if let Ok(json) = serde_json::from_str(arg) {
                            json_arg_vec.push(json);
                        } else {
                            json_arg_vec.push(serde_json::Value::String(arg.into()))
                        }
                    }
                    // And add the array of Value at the "/json" route
                    let _ = output_map.insert("json".into(), Value::Array(json_arg_vec));

                    // Add the array of (unparsed) text values of the args at "/text" route
                    output_map.insert("string".into(), json!(arg_vec));

                    (Some(Value::Object(output_map)), DONT_RUN_AGAIN)
                }
                _ => (None, DONT_RUN_AGAIN),
            };
        }
        (None, DONT_RUN_AGAIN)
    }
}
