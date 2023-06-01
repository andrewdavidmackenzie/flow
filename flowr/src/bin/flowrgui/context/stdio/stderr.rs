use std::sync::{Arc, Mutex};

use flowcore::{Implementation, RUN_AGAIN, RunAgain};
use flowcore::errors::*;
use serde_json::Value;

use crate::gui::client_message::ClientMessage;
use crate::gui::coordinator_connection::CoordinatorConnection;
use crate::gui::coordinator_message::CoordinatorMessage;

/// `Implementation` struct for the `Stderr` function
pub struct Stderr {
    /// It holds a reference to the runtime client in order to write output
    pub server_connection: Arc<Mutex<CoordinatorConnection>>,
}

impl Implementation for Stderr {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let input = &inputs[0];

        let mut server = self.server_connection.lock()
            .map_err(|_| "Could not lock server")?;

        let _: Result<ClientMessage> = match input {
                Value::Null => server.send_and_receive_response(CoordinatorMessage::StderrEof),
                Value::String(string) => server
                    .send_and_receive_response(CoordinatorMessage::Stderr(string.to_string())),
                Value::Bool(boolean) => server
                    .send_and_receive_response(CoordinatorMessage::Stderr(boolean.to_string())),
                Value::Number(number) => server
                    .send_and_receive_response(CoordinatorMessage::Stderr(number.to_string())),
                Value::Array(_array) => server
                    .send_and_receive_response(CoordinatorMessage::Stderr(input.to_string())),
                Value::Object(_obj) => server
                    .send_and_receive_response(CoordinatorMessage::Stderr(input.to_string())),
            };

        Ok((None, RUN_AGAIN))
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use flowcore::{Implementation, RUN_AGAIN};
    use serde_json::{json, Value};
    use serial_test::serial;

    use crate::gui::client_message::ClientMessage;
    use crate::gui::coordinator_message::CoordinatorMessage;
    use crate::gui::test_helper::test::wait_for_then_send;

    use super::Stderr;

    #[test]
    #[serial]
    fn send_null() {
        let server_connection = wait_for_then_send(CoordinatorMessage::StderrEof, ClientMessage::Ack);
        let stderr = &Stderr { server_connection } as &dyn Implementation;
        let (value, run_again) = stderr.run(&[Value::Null]).expect("_stderr() failed");

        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    #[serial]
    fn send_string() {
        let string = "string of text";
        let value = json!(string);
        let server_connection =
            wait_for_then_send(CoordinatorMessage::Stderr(string.into()), ClientMessage::Ack);
        let stderr = &Stderr { server_connection } as &dyn Implementation;
        let (value, run_again) = stderr.run(&[value]).expect("_stderr() failed");

        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    #[serial]
    fn send_bool() {
        let bool = true;
        let value = json!(bool);
        let server_connection =
            wait_for_then_send(CoordinatorMessage::Stderr("true".into()), ClientMessage::Ack);
        let stderr = &Stderr { server_connection } as &dyn Implementation;
        let (value, run_again) = stderr.run(&[value]).expect("_stderr() failed");

        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }
    #[test]
    #[serial]
    fn send_number() {
        let number = 42;
        let value = json!(number);
        let server_connection =
            wait_for_then_send(CoordinatorMessage::Stderr("42".into()), ClientMessage::Ack);
        let stderr = &Stderr { server_connection } as &dyn Implementation;
        let (value, run_again) = stderr.run(&[value]).expect("_stderr() failed");

        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    #[serial]
    fn send_array() {
        let array = [1, 2, 3];
        let value = json!(array);
        let server_connection =
            wait_for_then_send(CoordinatorMessage::Stderr("[1,2,3]".into()), ClientMessage::Ack);
        let stderr = &Stderr { server_connection } as &dyn Implementation;
        let (value, run_again) = stderr.run(&[value]).expect("_stderr() failed");

        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    #[serial]
    fn send_object() {
        let mut map = HashMap::new();
        map.insert("number1", 42);
        map.insert("number2", 99);
        let value = json!(map);
        let server_connection = wait_for_then_send(
            CoordinatorMessage::Stderr("{\"number1\":42,\"number2\":99}".into()),
            ClientMessage::Ack,
        );
        let stderr = &Stderr { server_connection } as &dyn Implementation;
        let (value, run_again) = stderr.run(&[value]).expect("_stderr() failed");

        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }
}
