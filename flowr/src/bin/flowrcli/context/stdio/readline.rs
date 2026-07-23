use flowcore::errors::Result;
use flowcore::{Implementation, RunAgain, DONT_RUN_AGAIN, RUN_AGAIN};
use serde_json::Value;

use crate::cli::coordinator_message::{ClientMessage, CoordinatorMessage};
use crate::context::ContextIO;

pub struct Readline {
    pub context_io: ContextIO,
}

impl Implementation for Readline {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let prompt = match inputs.first() {
            Some(Value::String(prompt)) => prompt.clone(),
            _ => String::new(),
        };

        let readline_response = self
            .context_io
            .send_and_receive_blocking(CoordinatorMessage::GetLine(prompt));

        match readline_response {
            Ok(ClientMessage::Line(contents)) => {
                let mut output_map = serde_json::Map::new();
                if let Ok(value) = serde_json::from_str(&contents) {
                    let _ = output_map.insert("json".into(), value);
                }
                output_map.insert("string".into(), Value::String(contents));
                Ok((Some(Value::Object(output_map)), RUN_AGAIN))
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

    use crate::cli::coordinator_message::{ClientMessage, CoordinatorMessage};
    use crate::context::ContextIO;

    use super::Readline;

    fn make_readline() -> (
        Readline,
        std::sync::mpsc::Receiver<crate::context::ContextRequest>,
    ) {
        let (nonblocking_tx, _nonblocking_rx) = std::sync::mpsc::channel();
        let (blocking_tx, blocking_rx) = std::sync::mpsc::channel();
        (
            Readline {
                context_io: ContextIO::new(nonblocking_tx, blocking_tx),
            },
            blocking_rx,
        )
    }

    #[test]
    fn gets_a_line_of_text() {
        let (reader, rx) = make_readline();
        let handle = std::thread::spawn(move || reader.run(&[]));

        let req = rx.recv().expect("No request");
        assert!(matches!(req.message, CoordinatorMessage::GetLine(_)));
        req.response_tx
            .unwrap()
            .send(ClientMessage::Line("line of text".into()))
            .unwrap();

        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        let val = value.expect("No value");
        let map = val.as_object().expect("Not an object");
        assert_eq!(map.get("string").unwrap(), &json!("line of text"));
    }

    #[test]
    fn gets_json() {
        let (reader, rx) = make_readline();
        let handle = std::thread::spawn(move || reader.run(&[]));

        let req = rx.recv().expect("No request");
        req.response_tx
            .unwrap()
            .send(ClientMessage::Line("\"json text\"".into()))
            .unwrap();

        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        let val = value.expect("No value");
        let map = val.as_object().expect("Not an object");
        assert_eq!(map.get("json").unwrap(), &json!("json text"));
    }

    #[test]
    fn get_eof() {
        let (reader, rx) = make_readline();
        let handle = std::thread::spawn(move || reader.run(&[]));

        let req = rx.recv().expect("No request");
        req.response_tx
            .unwrap()
            .send(ClientMessage::GetLineEof)
            .unwrap();

        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, DONT_RUN_AGAIN);
        assert!(value.is_none());
    }
}
