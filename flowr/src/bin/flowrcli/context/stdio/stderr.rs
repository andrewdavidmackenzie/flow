use flowcore::errors::Result;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};
use serde_json::Value;

use crate::cli::coordinator_message::CoordinatorMessage;
use crate::context::ContextIO;

/// `Implementation` struct for the `Stderr` function
pub struct Stderr {
    pub context_io: ContextIO,
}

impl Implementation for Stderr {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let input = inputs.first().ok_or("Could not get input")?;

        let msg = match input {
            Value::Null => CoordinatorMessage::StderrEof,
            Value::String(string) => CoordinatorMessage::Stderr(string.clone()),
            Value::Bool(boolean) => CoordinatorMessage::Stderr(boolean.to_string()),
            Value::Number(number) => CoordinatorMessage::Stderr(number.to_string()),
            _ => CoordinatorMessage::Stderr(input.to_string()),
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

    use crate::cli::coordinator_message::{ClientMessage, CoordinatorMessage};
    use crate::context::ContextIO;

    use super::Stderr;

    fn make_stderr() -> (
        Stderr,
        std::sync::mpsc::Receiver<crate::context::ContextRequest>,
    ) {
        let (tx, rx) = std::sync::mpsc::channel();
        let (blocking_tx, _blocking_rx) = std::sync::mpsc::channel();
        (
            Stderr {
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
        let (stderr, rx) = make_stderr();
        let handle = std::thread::spawn(move || stderr.run(&[Value::Null]));
        respond(&rx, &CoordinatorMessage::StderrEof);
        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    fn send_string() {
        let (stderr, rx) = make_stderr();
        let handle = std::thread::spawn(move || stderr.run(&[json!("hello")]));
        respond(&rx, &CoordinatorMessage::Stderr(String::new()));
        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    fn send_bool() {
        let (stderr, rx) = make_stderr();
        let handle = std::thread::spawn(move || stderr.run(&[json!(true)]));
        respond(&rx, &CoordinatorMessage::Stderr(String::new()));
        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    fn send_number() {
        let (stderr, rx) = make_stderr();
        let handle = std::thread::spawn(move || stderr.run(&[json!(42)]));
        respond(&rx, &CoordinatorMessage::Stderr(String::new()));
        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    fn send_array() {
        let (stderr, rx) = make_stderr();
        let handle = std::thread::spawn(move || stderr.run(&[json!([1, 2, 3])]));
        respond(&rx, &CoordinatorMessage::Stderr(String::new()));
        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    fn send_object() {
        let mut map = HashMap::new();
        map.insert("number1", 42);
        map.insert("number2", 99);
        let (stderr, rx) = make_stderr();
        let handle = std::thread::spawn(move || stderr.run(&[json!(map)]));
        respond(&rx, &CoordinatorMessage::Stderr(String::new()));
        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }
}
