//! Args context function.

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use flowcore::errors::Result;
use flowcore::{Implementation, RunAgain, DONT_RUN_AGAIN};

use crate::coordinator::client_message::ClientMessage;
use crate::coordinator::coordinator_connection::CoordinatorConnection;
use crate::coordinator::coordinator_message::CoordinatorMessage;

/// Get flow arguments
pub struct Get {
    pub server_connection: Arc<Mutex<CoordinatorConnection>>,
}

impl Implementation for Get {
    fn run(&self, _inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let mut guard = self
            .server_connection
            .lock()
            .map_err(|_| "Could not lock server")?;

        let sent = guard.send_and_receive_response(CoordinatorMessage::GetArgs);

        match sent {
            Ok(ClientMessage::Args(arg_vec)) => {
                let mut output_map = serde_json::Map::new();
                let mut json_arg_vec: Vec<Value> = Vec::new();
                for arg in &arg_vec {
                    if let Ok(json_val) = serde_json::from_str(arg) {
                        json_arg_vec.push(json_val);
                    } else {
                        json_arg_vec.push(Value::String(arg.into()));
                    }
                }
                output_map.insert("json".into(), Value::Array(json_arg_vec));
                output_map.insert("string".into(), json!(arg_vec));
                Ok((Some(Value::Object(output_map)), DONT_RUN_AGAIN))
            }
            _ => Ok((None, DONT_RUN_AGAIN)),
        }
    }
}
