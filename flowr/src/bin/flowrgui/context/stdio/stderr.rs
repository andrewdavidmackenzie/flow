use flowcore::errors::Result;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};
use serde_json::Value;

use crate::context::ContextIO;
use crate::gui::coordinator_message::CoordinatorMessage;

/// `Implementation` struct for the `Stderr` function
pub struct Stderr {
    /// It holds a reference to the runtime client in order to write output
    pub context_io: ContextIO,
}

impl Implementation for Stderr {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let value = inputs.first().ok_or("Could not get 'value' input")?;
        let format = inputs.get(1).and_then(Value::as_str).unwrap_or("{}");

        let msg = match value {
            Value::Null => CoordinatorMessage::StderrEof,
            _ => CoordinatorMessage::Stderr(super::format_output(value, format)),
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
        // Compare the payload string when the expected value is non-empty
        if let (CoordinatorMessage::Stderr(actual), CoordinatorMessage::Stderr(exp)) =
            (&req.message, expected)
        {
            if !exp.is_empty() {
                assert_eq!(actual, exp, "Stderr payload mismatch");
            }
        }
        if let Some(response_tx) = req.response_tx {
            response_tx
                .send(ClientMessage::Ack)
                .expect("Could not send response");
        }
    }

    #[test]
    fn send_null() {
        let (stderr, rx) = make_stderr();
        let handle = std::thread::spawn(move || stderr.run(&[Value::Null, json!("{}")]));
        respond(&rx, &CoordinatorMessage::StderrEof);
        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    fn send_string() {
        let (stderr, rx) = make_stderr();
        let handle = std::thread::spawn(move || stderr.run(&[json!("hello"), json!("{}")]));
        respond(&rx, &CoordinatorMessage::Stderr("hello".into()));
        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    fn send_bool() {
        let (stderr, rx) = make_stderr();
        let handle = std::thread::spawn(move || stderr.run(&[json!(true), json!("{}")]));
        respond(&rx, &CoordinatorMessage::Stderr("true".into()));
        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    fn send_number() {
        let (stderr, rx) = make_stderr();
        let handle = std::thread::spawn(move || stderr.run(&[json!(42), json!("{}")]));
        respond(&rx, &CoordinatorMessage::Stderr("42".into()));
        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    fn send_array() {
        let (stderr, rx) = make_stderr();
        let handle = std::thread::spawn(move || stderr.run(&[json!([1, 2, 3]), json!("{}")]));
        respond(&rx, &CoordinatorMessage::Stderr("[1,2,3]".into()));
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
        let handle = std::thread::spawn(move || stderr.run(&[json!(map), json!("{}")]));
        respond(&rx, &CoordinatorMessage::Stderr(String::new()));
        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    fn send_with_format_prefix() {
        let (stderr, rx) = make_stderr();
        let handle = std::thread::spawn(move || stderr.run(&[json!(42), json!("Error: {}")]));
        respond(&rx, &CoordinatorMessage::Stderr("Error: 42".into()));
        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }
}
