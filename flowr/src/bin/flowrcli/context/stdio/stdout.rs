use std::sync::{Arc, Mutex};

use flowcore::{Implementation, RUN_AGAIN, RunAgain};
use flowcore::errors::Result;
use serde_json::Value;

use crate::cli::connections::CoordinatorConnection;
use crate::cli::coordinator_message::{ClientMessage, CoordinatorMessage};

/// `Implementation` struct for the `Stdout` function
pub struct Stdout {
    /// It holds a reference to the runtime client in order to write output
    pub server_connection: Arc<Mutex<CoordinatorConnection>>,
}

impl Implementation for Stdout {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let input = inputs.first().ok_or("Could not get input")?;

        // Gain sole access to send to the client to avoid mixing output from other functions
        let mut server = self.server_connection.lock()
            .map_err(|_| "Could not lock server")?;

        let _: Result<ClientMessage> = match input {
                Value::Null => server.send_and_receive_response(CoordinatorMessage::StdoutEof),
                Value::String(string) => server
                    .send_and_receive_response(CoordinatorMessage::Stdout(string.to_string())),
                Value::Bool(boolean) => server
                    .send_and_receive_response(CoordinatorMessage::Stdout(boolean.to_string())),
                Value::Number(number) => server
                    .send_and_receive_response(CoordinatorMessage::Stdout(number.to_string())),
                Value::Array(_array) => server
                    .send_and_receive_response(CoordinatorMessage::Stdout(input.to_string())),
                Value::Object(_obj) => server
                    .send_and_receive_response(CoordinatorMessage::Stdout(input.to_string())),
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

    use crate::cli::coordinator_message::{ClientMessage, CoordinatorMessage};
    use crate::cli::test_helper::test::wait_for_then_send;

    use super::Stdout;

    #[test]
    #[serial]
    fn send_null() {
        let server_connection = wait_for_then_send(CoordinatorMessage::StdoutEof, ClientMessage::Ack);
        let stderr = &Stdout { server_connection } as &dyn Implementation;
        let (value, run_again) = stderr.run(&[Value::Null]).expect("run() failed");

        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    #[serial]
    fn send_string() {
        let string = "string of text";
        let value = json!(string);
        let server_connection =
            wait_for_then_send(CoordinatorMessage::Stdout(string.into()), ClientMessage::Ack);
        let stderr = &Stdout { server_connection } as &dyn Implementation;
        let (value, run_again) = stderr.run(&[value]).expect("run() failed");

        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    #[serial]
    fn send_bool() {
        let bool = true;
        let value = json!(bool);
        let server_connection =
            wait_for_then_send(CoordinatorMessage::Stdout("true".into()), ClientMessage::Ack);
        let stderr = &Stdout { server_connection } as &dyn Implementation;
        let (value, run_again) = stderr.run(&[value]).expect("run() failed");

        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }
    #[test]
    #[serial]
    fn send_number() {
        let number = 42;
        let value = json!(number);
        let server_connection =
            wait_for_then_send(CoordinatorMessage::Stdout("42".into()), ClientMessage::Ack);
        let stderr = &Stdout { server_connection } as &dyn Implementation;
        let (value, run_again) = stderr.run(&[value]).expect("run() failed");

        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    #[serial]
    fn send_array() {
        let array = [1, 2, 3];
        let value = json!(array);
        let server_connection =
            wait_for_then_send(CoordinatorMessage::Stdout("[1,2,3]".into()), ClientMessage::Ack);
        let stderr = &Stdout { server_connection } as &dyn Implementation;
        let (value, run_again) = stderr.run(&[value]).expect("run() failed");

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
            CoordinatorMessage::Stdout("{\"number1\":42,\"number2\":99}".into()),
            ClientMessage::Ack,
        );
        let stderr = &Stdout { server_connection } as &dyn Implementation;
        let (value, run_again) = stderr.run(&[value]).expect("run() failed");

        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }
}
