use std::sync::{Arc, Mutex};

use serde_json::Value;

use flowcore::{DONT_RUN_AGAIN, Implementation, RUN_AGAIN, RunAgain};
use flowcore::errors::*;

use crate::cli::client_server::ServerConnection;
use crate::cli::runtime_messages::{ClientMessage, ServerMessage};

/// `Implementation` struct for the `Stdin` function
pub struct Stdin {
    /// It holds a reference to the runtime client in order to read input
    pub server_connection: Arc<Mutex<ServerConnection>>,
}

impl Implementation for Stdin {
    fn run(&self, _inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let mut server = self.server_connection.lock()
            .map_err(|_| "Could not lock server")?;

        match server.send_and_receive_response(ServerMessage::GetStdin) {
            Ok(ClientMessage::Stdin(contents)) => {
                let mut output_map = serde_json::Map::new();
                if let Ok(value) = serde_json::from_str(&contents) {
                    let _ = output_map.insert("json".into(), value);
                };
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

#[cfg(test)]
mod test {
    use serde_json::json;
    use serde_json::Value;
    use serial_test::serial;

    use flowcore::{DONT_RUN_AGAIN, Implementation, RUN_AGAIN};

    use crate::cli::runtime_messages::{ClientMessage, ServerMessage};
    use crate::cli::test_helper::test::wait_for_then_send;

    use super::Stdin;

    #[test]
    #[serial]
    fn gets_a_line_of_text() {
        let server_connection = wait_for_then_send(
            ServerMessage::GetStdin,
            ClientMessage::Stdin("line of text".into()),
        );
        let stdin = &Stdin { server_connection } as &dyn Implementation;
        let (value, run_again) = stdin.run(&[]).expect("_stdin() failed");

        assert_eq!(run_again, RUN_AGAIN);

        let val = value.expect("Could not get value returned from implementation");
        let map = val.as_object().expect("Could not get map of output values");
        assert_eq!(
            map.get("string").expect("Could not get string args"),
            &json!("line of text")
        );
    }

    #[test]
    #[serial]
    fn bad_reply_message() {
        let server_connection = wait_for_then_send(ServerMessage::GetStdin, ClientMessage::Ack);
        let stdin = &Stdin { server_connection } as &dyn Implementation;
        let (value, run_again) = stdin.run(&[]).expect("_stdin() failed");

        assert_eq!(run_again, DONT_RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    #[serial]
    fn gets_json() {
        let server_connection = wait_for_then_send(
            ServerMessage::GetStdin,
            ClientMessage::Stdin("\"json text\"".into()),
        );
        let stdin = &Stdin { server_connection } as &dyn Implementation;
        let (value, run_again) = stdin.run(&[]).expect("_stdin() failed");

        assert_eq!(run_again, RUN_AGAIN);

        let val = value.expect("Could not get value returned from implementation");
        let map = val.as_object().expect("Could not get map of output values");
        assert_eq!(
            map.get("json").expect("Could not get json args"),
            &json!("json text")
        );
    }

    #[test]
    #[serial]
    fn get_eof() {
        let server_connection =
            wait_for_then_send(ServerMessage::GetStdin, ClientMessage::GetStdinEof);
        let stdin = &Stdin { server_connection } as &dyn Implementation;
        let (value, run_again) = stdin.run(&[]).expect("_stdin() failed");

        assert_eq!(run_again, DONT_RUN_AGAIN);
        let val = value.expect("Could not get value returned from implementation");
        let map = val.as_object().expect("Could not get map of output values");
        assert_eq!(
            map.get("json").expect("Could not get json args"),
            &Value::Null
        );
        assert_eq!(
            map.get("string").expect("Could not get string args"),
            &Value::Null
        );
    }
}
