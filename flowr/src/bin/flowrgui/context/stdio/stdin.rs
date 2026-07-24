use flowcore::errors::Result;
use flowcore::{Implementation, RunAgain, DONT_RUN_AGAIN, RUN_AGAIN};
use serde_json::Value;

use crate::context::ContextIO;
use crate::gui::client_message::ClientMessage;
use crate::gui::coordinator_message::CoordinatorMessage;

/// `Implementation` struct for the `Stdin` function
pub struct Stdin {
    /// It holds a reference to the runtime client in order to read input
    pub context_io: ContextIO,
}

impl Implementation for Stdin {
    fn run(&self, _inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let stdin_response = self
            .context_io
            .send_and_receive_blocking(CoordinatorMessage::GetStdin);

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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use flowcore::{Implementation, DONT_RUN_AGAIN, RUN_AGAIN};
    use serde_json::json;
    use serde_json::Value;

    use crate::context::ContextIO;
    use crate::gui::client_message::ClientMessage;
    use crate::gui::coordinator_message::CoordinatorMessage;

    use super::Stdin;

    fn make_stdin() -> (
        Stdin,
        std::sync::mpsc::Receiver<crate::context::ContextRequest>,
    ) {
        let (nonblocking_tx, _nonblocking_rx) = std::sync::mpsc::channel();
        let (blocking_tx, blocking_rx) = std::sync::mpsc::channel();
        (
            Stdin {
                context_io: ContextIO::new(nonblocking_tx, blocking_tx),
            },
            blocking_rx,
        )
    }

    #[test]
    fn gets_a_line_of_text() {
        let (stdin, rx) = make_stdin();
        let handle = std::thread::spawn(move || stdin.run(&[]));

        let req = rx.recv().expect("No request");
        assert!(matches!(req.message, CoordinatorMessage::GetStdin));
        req.response_tx
            .unwrap()
            .send(ClientMessage::Stdin("line of text".into()))
            .unwrap();

        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        let val = value.expect("Could not get value returned from implementation");
        let map = val.as_object().expect("Could not get map of output values");
        assert_eq!(
            map.get("string").expect("Could not get string args"),
            &json!("line of text")
        );
    }

    #[test]
    fn bad_reply_message() {
        let (stdin, rx) = make_stdin();
        let handle = std::thread::spawn(move || stdin.run(&[]));

        let req = rx.recv().expect("No request");
        req.response_tx.unwrap().send(ClientMessage::Ack).unwrap();

        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, DONT_RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    fn gets_json() {
        let (stdin, rx) = make_stdin();
        let handle = std::thread::spawn(move || stdin.run(&[]));

        let req = rx.recv().expect("No request");
        req.response_tx
            .unwrap()
            .send(ClientMessage::Stdin("\"json text\"".into()))
            .unwrap();

        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        let val = value.expect("Could not get value returned from implementation");
        let map = val.as_object().expect("Could not get map of output values");
        assert_eq!(
            map.get("json").expect("Could not get json args"),
            &json!("json text")
        );
    }

    #[test]
    fn get_eof() {
        let (stdin, rx) = make_stdin();
        let handle = std::thread::spawn(move || stdin.run(&[]));

        let req = rx.recv().expect("No request");
        req.response_tx
            .unwrap()
            .send(ClientMessage::GetStdinEof)
            .unwrap();

        let (value, run_again) = handle.join().unwrap().expect("run() failed");
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
