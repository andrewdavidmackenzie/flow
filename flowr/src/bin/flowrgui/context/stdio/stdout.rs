use flowcore::errors::Result;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};
use serde_json::Value;

use crate::context::ContextIO;
use crate::gui::coordinator_message::CoordinatorMessage;

/// `Implementation` struct for the `Stdout` function
pub struct Stdout {
    /// It holds a reference to the runtime client in order to write output
    pub context_io: ContextIO,
}

impl Implementation for Stdout {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let input = inputs.first().ok_or("Could not get input")?;

        let msg = match input {
            Value::Null => CoordinatorMessage::StdoutEof,
            Value::String(string) => CoordinatorMessage::Stdout(string.clone()),
            Value::Bool(boolean) => CoordinatorMessage::Stdout(boolean.to_string()),
            Value::Number(number) => CoordinatorMessage::Stdout(number.to_string()),
            _ => CoordinatorMessage::Stdout(input.to_string()),
        };

        self.context_io.send_and_receive(msg)?;

        Ok((None, RUN_AGAIN))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use std::collections::HashMap;

    use flowcore::{Implementation, RUN_AGAIN};
    use serde_json::{json, Value};

    use crate::context::ContextIO;
    use crate::gui::client_message::ClientMessage;
    use crate::gui::coordinator_message::CoordinatorMessage;

    use super::Stdout;

    fn make_stdout() -> (
        Stdout,
        std::sync::mpsc::Receiver<crate::context::ContextRequest>,
    ) {
        let (tx, rx) = std::sync::mpsc::channel();
        let (blocking_tx, _blocking_rx) = std::sync::mpsc::channel();
        (
            Stdout {
                context_io: ContextIO::new(tx, blocking_tx),
            },
            rx,
        )
    }

    fn respond(
        rx: &std::sync::mpsc::Receiver<crate::context::ContextRequest>,
        expected: &CoordinatorMessage,
    ) {
        let req = rx.recv().expect("No request received");
        assert_eq!(
            std::mem::discriminant(&req.message),
            std::mem::discriminant(expected)
        );
        if let Some(response_tx) = req.response_tx {
            response_tx
                .send(ClientMessage::Ack)
                .expect("Could not send response");
        }
    }

    #[test]
    fn send_null() {
        let (stdout, rx) = make_stdout();
        let handle = std::thread::spawn(move || stdout.run(&[Value::Null]));
        respond(&rx, &CoordinatorMessage::StdoutEof);
        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    fn send_string() {
        let (stdout, rx) = make_stdout();
        let handle = std::thread::spawn(move || stdout.run(&[json!("hello")]));
        respond(&rx, &CoordinatorMessage::Stdout(String::new()));
        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    fn send_bool() {
        let (stdout, rx) = make_stdout();
        let handle = std::thread::spawn(move || stdout.run(&[json!(true)]));
        respond(&rx, &CoordinatorMessage::Stdout(String::new()));
        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    fn send_number() {
        let (stdout, rx) = make_stdout();
        let handle = std::thread::spawn(move || stdout.run(&[json!(42)]));
        respond(&rx, &CoordinatorMessage::Stdout(String::new()));
        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    fn send_array() {
        let (stdout, rx) = make_stdout();
        let handle = std::thread::spawn(move || stdout.run(&[json!([1, 2, 3])]));
        respond(&rx, &CoordinatorMessage::Stdout(String::new()));
        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    fn send_object() {
        let mut map = HashMap::new();
        map.insert("number1", 42);
        map.insert("number2", 99);
        let (stdout, rx) = make_stdout();
        let handle = std::thread::spawn(move || stdout.run(&[json!(map)]));
        respond(&rx, &CoordinatorMessage::Stdout(String::new()));
        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }
}
