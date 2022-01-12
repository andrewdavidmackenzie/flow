use std::sync::{Arc, Mutex};

use serde_json::Value;

use flowcore::{Implementation, RUN_AGAIN, RunAgain};

use crate::client_server::ServerConnection;
use crate::errors::*;
use crate::runtime_messages::{ClientMessage, ServerMessage};

/// `Implementation` struct for the `Stderr` function
pub struct Stderr {
    /// It holds a reference to the runtime client in order to write output
    pub server_connection: Arc<Mutex<ServerConnection<ServerMessage, ClientMessage>>>,
}

impl Implementation for Stderr {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        if inputs.len() == 1 {
            let input = &inputs[0];

            if let Ok(mut server) = self.server_connection.lock() {
                let _: Result<ClientMessage> =
                    match input {
                        Value::Null => server.send_and_receive_response(ServerMessage::StderrEof),
                        Value::String(string) => server
                            .send_and_receive_response(ServerMessage::Stderr(string.to_string())),
                        Value::Bool(boolean) => server
                            .send_and_receive_response(ServerMessage::Stderr(boolean.to_string())),
                        Value::Number(number) => server
                            .send_and_receive_response(ServerMessage::Stderr(number.to_string())),
                        Value::Array(_array) => server
                            .send_and_receive_response(ServerMessage::Stderr(input.to_string())),
                        Value::Object(_obj) => server
                            .send_and_receive_response(ServerMessage::Stderr(input.to_string())),
                    };
            }
        }

        (None, RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use serde_json::{json, Value};
    use serial_test::serial;

    use flowcore::{Implementation, RUN_AGAIN};

    use crate::context::stdio::stderr::Stderr;
    use crate::runtime_messages::{ClientMessage, ServerMessage};

    use super::super::super::test_helper::test::wait_for_then_send;

    #[test]
    #[serial(client_server)]
    fn invalid_input() {
        let server_connection = wait_for_then_send(ServerMessage::StderrEof, ClientMessage::Ack);
        let stderr = &Stderr { server_connection } as &dyn Implementation;
        let (value, run_again) = stderr.run(&[]);

        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    #[serial(client_server)]
    fn send_null() {
        let server_connection = wait_for_then_send(ServerMessage::StderrEof, ClientMessage::Ack);
        let stderr = &Stderr { server_connection } as &dyn Implementation;
        let (value, run_again) = stderr.run(&[Value::Null]);

        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    #[serial(client_server)]
    fn send_string() {
        let string = "string of text";
        let value = json!(string);
        let server_connection =
            wait_for_then_send(ServerMessage::Stderr(string.into()), ClientMessage::Ack);
        let stderr = &Stderr { server_connection } as &dyn Implementation;
        let (value, run_again) = stderr.run(&[value]);

        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    #[serial(client_server)]
    fn send_bool() {
        let bool = true;
        let value = json!(bool);
        let server_connection =
            wait_for_then_send(ServerMessage::Stderr("true".into()), ClientMessage::Ack);
        let stderr = &Stderr { server_connection } as &dyn Implementation;
        let (value, run_again) = stderr.run(&[value]);

        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }
    #[test]
    #[serial(client_server)]
    fn send_number() {
        let number = 42;
        let value = json!(number);
        let server_connection =
            wait_for_then_send(ServerMessage::Stderr("42".into()), ClientMessage::Ack);
        let stderr = &Stderr { server_connection } as &dyn Implementation;
        let (value, run_again) = stderr.run(&[value]);

        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    #[serial(client_server)]
    fn send_array() {
        let array = [1, 2, 3];
        let value = json!(array);
        let server_connection =
            wait_for_then_send(ServerMessage::Stderr("[1,2,3]".into()), ClientMessage::Ack);
        let stderr = &Stderr { server_connection } as &dyn Implementation;
        let (value, run_again) = stderr.run(&[value]);

        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    #[serial(client_server)]
    fn send_object() {
        let mut map = HashMap::new();
        map.insert("number1", 42);
        map.insert("number2", 99);
        let value = json!(map);
        let server_connection = wait_for_then_send(
            ServerMessage::Stderr("{\"number1\":42,\"number2\":99}".into()),
            ClientMessage::Ack,
        );
        let stderr = &Stderr { server_connection } as &dyn Implementation;
        let (value, run_again) = stderr.run(&[value]);

        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }
}
