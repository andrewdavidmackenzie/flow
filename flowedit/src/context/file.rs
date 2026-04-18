//! File I/O context functions.

use std::sync::{Arc, Mutex};

use flowcore::errors::Result;
use flowcore::{Implementation, RunAgain, DONT_RUN_AGAIN, RUN_AGAIN};
use serde_json::{json, Value};

use crate::coordinator::client_message::ClientMessage;
use crate::coordinator::coordinator_connection::CoordinatorConnection;
use crate::coordinator::coordinator_message::CoordinatorMessage;

/// File read context function
pub struct FileRead {
    pub server_connection: Arc<Mutex<CoordinatorConnection>>,
}

impl Implementation for FileRead {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let path = inputs.first().ok_or("Could not get filename")?;
        let mut server = self
            .server_connection
            .lock()
            .map_err(|_| "Could not lock server")?;

        let response = server.send_and_receive_response::<CoordinatorMessage, ClientMessage>(
            CoordinatorMessage::Read(path.as_str().unwrap_or("").to_string()),
        );

        match response {
            Ok(ClientMessage::FileContents(_path, bytes)) => {
                let mut output_map = serde_json::Map::new();
                output_map.insert("bytes".into(), json!(bytes));
                let string =
                    String::from_utf8(bytes).map_err(|_| "Could not create Utf8 String")?;
                output_map.insert("string".into(), json!(string));
                Ok((Some(Value::Object(output_map)), RUN_AGAIN))
            }
            _ => Ok((None, DONT_RUN_AGAIN)),
        }
    }
}

/// File write context function
pub struct FileWrite {
    pub server_connection: Arc<Mutex<CoordinatorConnection>>,
}

impl Implementation for FileWrite {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let filename = inputs.first().ok_or("Could not get filename")?;
        let bytes = inputs.get(1).ok_or("Could not get bytes")?;
        let mut server = self
            .server_connection
            .lock()
            .map_err(|_| "Could not lock server")?;

        let byte_array = bytes.as_array().ok_or("Could not get bytes")?;
        #[allow(clippy::cast_possible_truncation)]
        let bytes: Vec<u8> = byte_array
            .iter()
            .map(|byte_value| byte_value.as_u64().unwrap_or(0) as u8)
            .collect();
        let _ = server.send_and_receive_response::<CoordinatorMessage, ClientMessage>(
            CoordinatorMessage::Write(filename.as_str().unwrap_or("").to_string(), bytes),
        );

        Ok((None, RUN_AGAIN))
    }
}
