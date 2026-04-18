//! Standard I/O context functions: stdout, stderr, stdin, readline.

use std::sync::{Arc, Mutex};

use flowcore::errors::Result;
use flowcore::{Implementation, RunAgain, DONT_RUN_AGAIN, RUN_AGAIN};
use serde_json::Value;

use crate::coordinator::client_message::ClientMessage;
use crate::coordinator::coordinator_connection::CoordinatorConnection;
use crate::coordinator::coordinator_message::CoordinatorMessage;

/// Stdout context function
pub struct Stdout {
    pub server_connection: Arc<Mutex<CoordinatorConnection>>,
}

impl Implementation for Stdout {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let input = inputs.first().ok_or("Could not get input")?;
        let mut server = self
            .server_connection
            .lock()
            .map_err(|_| "Could not lock server")?;

        let _: Result<ClientMessage> = match input {
            Value::Null => server.send_and_receive_response(CoordinatorMessage::StdoutEof),
            Value::String(string) => {
                server.send_and_receive_response(CoordinatorMessage::Stdout(string.clone()))
            }
            Value::Bool(boolean) => {
                server.send_and_receive_response(CoordinatorMessage::Stdout(boolean.to_string()))
            }
            Value::Number(number) => {
                server.send_and_receive_response(CoordinatorMessage::Stdout(number.to_string()))
            }
            Value::Array(_) | Value::Object(_) => {
                server.send_and_receive_response(CoordinatorMessage::Stdout(input.to_string()))
            }
        };

        Ok((None, RUN_AGAIN))
    }
}

/// Stderr context function
pub struct Stderr {
    pub server_connection: Arc<Mutex<CoordinatorConnection>>,
}

impl Implementation for Stderr {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let input = inputs.first().ok_or("Could not get input")?;
        let mut server = self
            .server_connection
            .lock()
            .map_err(|_| "Could not lock server")?;

        let _: Result<ClientMessage> = match input {
            Value::Null => server.send_and_receive_response(CoordinatorMessage::StderrEof),
            Value::String(string) => {
                server.send_and_receive_response(CoordinatorMessage::Stderr(string.clone()))
            }
            Value::Bool(boolean) => {
                server.send_and_receive_response(CoordinatorMessage::Stderr(boolean.to_string()))
            }
            Value::Number(number) => {
                server.send_and_receive_response(CoordinatorMessage::Stderr(number.to_string()))
            }
            Value::Array(_) | Value::Object(_) => {
                server.send_and_receive_response(CoordinatorMessage::Stderr(input.to_string()))
            }
        };

        Ok((None, RUN_AGAIN))
    }
}

/// Stdin context function
pub struct Stdin {
    pub server_connection: Arc<Mutex<CoordinatorConnection>>,
}

impl Implementation for Stdin {
    fn run(&self, _inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let mut server = self
            .server_connection
            .lock()
            .map_err(|_| "Could not lock server")?;

        let stdin_response = server.send_and_receive_response(CoordinatorMessage::GetStdin);

        match stdin_response {
            Ok(ClientMessage::Stdin(contents)) => {
                let mut output_map = serde_json::Map::new();
                if let Ok(value) = serde_json::from_str(&contents) {
                    let _ = output_map.insert("json".into(), value);
                }
                output_map.insert("string".into(), Value::String(contents));
                Ok((Some(Value::Object(output_map)), RUN_AGAIN))
            }
            Ok(ClientMessage::GetStdinEof) => {
                let mut output_map = serde_json::Map::new();
                output_map.insert("string".into(), Value::Null);
                output_map.insert("json".into(), Value::Null);
                Ok((Some(Value::Object(output_map)), DONT_RUN_AGAIN))
            }
            _ => Ok((None, DONT_RUN_AGAIN)),
        }
    }
}

/// Readline context function
pub struct Readline {
    pub server_connection: Arc<Mutex<CoordinatorConnection>>,
}

impl Implementation for Readline {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let mut server = self
            .server_connection
            .lock()
            .map_err(|_| "Could not lock server")?;

        let prompt = match inputs.first() {
            Some(Value::String(prompt)) => prompt.clone(),
            _ => String::new(),
        };

        let readline_response =
            server.send_and_receive_response(CoordinatorMessage::GetLine(prompt));

        match readline_response {
            Ok(ClientMessage::Line(contents)) => {
                let mut output_map = serde_json::Map::new();
                if let Ok(value) = serde_json::from_str(&contents) {
                    let _ = output_map.insert("json".into(), value);
                }
                output_map.insert("string".into(), Value::String(contents));
                Ok((Some(Value::Object(output_map)), RUN_AGAIN))
            }
            Ok(ClientMessage::GetLineEof) => {
                let mut output_map = serde_json::Map::new();
                output_map.insert("string".into(), Value::Null);
                output_map.insert("json".into(), Value::Null);
                Ok((Some(Value::Object(output_map)), DONT_RUN_AGAIN))
            }
            _ => Ok((None, DONT_RUN_AGAIN)),
        }
    }
}
