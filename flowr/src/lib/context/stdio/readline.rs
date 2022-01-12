use std::sync::{Arc, Mutex};

use serde_json::Value;

use flowcore::{DONT_RUN_AGAIN, Implementation, RUN_AGAIN, RunAgain};

use crate::client_server::ServerConnection;
use crate::runtime_messages::{ClientMessage, ServerMessage};

/// `Implementation` struct for the `readline` function
pub struct Readline {
    /// It holds a reference to the runtime client in order to read input
    pub server_connection: Arc<Mutex<ServerConnection<ServerMessage, ClientMessage>>>,
}

impl Implementation for Readline {
    fn run(&self, _inputs: &[Value]) -> (Option<Value>, RunAgain) {
        if let Ok(mut server) = self.server_connection.lock() {
            return match server.send_and_receive_response(ServerMessage::GetLine) {
                Ok(ClientMessage::Line(contents)) => {
                    let mut output_map = serde_json::Map::new();
                    if let Ok(value) = serde_json::from_str(&contents) {
                        let _ = output_map.insert("json".into(), value);
                    };
                    output_map.insert("string".into(), Value::String(contents));
                    (Some(Value::Object(output_map)), RUN_AGAIN)
                }
                Ok(ClientMessage::GetLineEof) => {
                    let mut output_map = serde_json::Map::new();
                    output_map.insert("string".into(), Value::Null);
                    output_map.insert("json".into(), Value::Null);
                    (Some(Value::Object(output_map)), DONT_RUN_AGAIN)
                }
                _ => (None, DONT_RUN_AGAIN),
            };
        }
        (None, DONT_RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use serde_json::Value;
    use serial_test::serial;

    use flowcore::{DONT_RUN_AGAIN, Implementation, RUN_AGAIN};

    use crate::runtime_messages::{ClientMessage, ServerMessage};

    use super::Readline;
    use super::super::super::test_helper::test::wait_for_then_send;

    #[test]
    #[serial(client_server)]
    fn gets_a_line_of_text() {
        let server_connection = wait_for_then_send(
            ServerMessage::GetLine,
            ClientMessage::Line("line of text".into()),
        );
        let reader = &Readline { server_connection } as &dyn Implementation;
        let (value, run_again) = reader.run(&[]);

        assert_eq!(run_again, RUN_AGAIN);

        let val = value.expect("Could not get value returned from implementation");
        let map = val.as_object().expect("Could not get map of output values");
        assert_eq!(
            map.get("string").expect("Could not get string args"),
            &json!("line of text")
        );
    }

    #[test]
    #[serial(client_server)]
    fn gets_json() {
        let server_connection = wait_for_then_send(
            ServerMessage::GetLine,
            ClientMessage::Line("\"json text\"".into()),
        );
        let reader = &Readline { server_connection } as &dyn Implementation;
        let (value, run_again) = reader.run(&[]);

        assert_eq!(run_again, RUN_AGAIN);

        let val = value.expect("Could not get value returned from implementation");
        let map = val.as_object().expect("Could not get map of output values");
        assert_eq!(
            map.get("json").expect("Could not get json args"),
            &json!("json text")
        );
    }

    #[test]
    #[serial(client_server)]
    fn get_eof() {
        let server_connection =
            wait_for_then_send(ServerMessage::GetLine, ClientMessage::GetLineEof);
        let reader = &Readline { server_connection } as &dyn Implementation;
        let (value, run_again) = reader.run(&[]);

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
