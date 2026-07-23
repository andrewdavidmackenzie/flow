use flowcore::errors::Result;
use flowcore::{Implementation, RunAgain, DONT_RUN_AGAIN, RUN_AGAIN};
use serde_json::{json, Value};

use crate::cli::coordinator_message::{ClientMessage, CoordinatorMessage};
use crate::context::ContextIO;

/// `Implementation` struct for the `file_read` function
pub struct FileRead {
    pub context_io: ContextIO,
}

impl Implementation for FileRead {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let path = inputs.first().ok_or("Could not get path")?;

        let response = self.context_io.send_nonblocking(CoordinatorMessage::Read(
            path.as_str().unwrap_or("").to_string(),
        ));

        match response {
            Ok(ClientMessage::FileContents(_path, bytes)) => {
                let mut output_map = serde_json::Map::new();
                output_map.insert("bytes".into(), json!(bytes));
                let string =
                    String::from_utf8(bytes).map_err(|_| "Could not create Utf8 String")?;
                output_map.insert("string".into(), json!(string));
                Ok((Some(Value::Object(output_map)), RUN_AGAIN))
            }
            _ => Ok((None, DONT_RUN_AGAIN)),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use flowcore::{Implementation, RUN_AGAIN};
    use serde_json::{json, Value};

    use crate::cli::coordinator_message::{ClientMessage, CoordinatorMessage};
    use crate::context::ContextIO;

    use super::FileRead;

    fn make_file_read() -> (
        FileRead,
        std::sync::mpsc::Receiver<crate::context::ContextRequest>,
    ) {
        let (tx, rx) = std::sync::mpsc::channel();
        let (dummy_tx, _dummy_rx) = std::sync::mpsc::channel();
        (
            FileRead {
                context_io: ContextIO::new(dummy_tx, tx),
            },
            rx,
        )
    }

    #[test]
    fn read_file() {
        let file_path = "test_read";
        let file_contents: Vec<u8> = "test text".as_bytes().to_vec();
        let file_string =
            String::from_utf8(file_contents.clone()).expect("Could not create Utf8 String");

        let inputs = [json!(file_path)];

        let (reader, rx) = make_file_read();
        let handle = std::thread::spawn(move || reader.run(&inputs));

        let req = rx.recv().expect("No request");
        assert!(matches!(req.message, CoordinatorMessage::Read(_)));
        req.response_tx
            .unwrap()
            .send(ClientMessage::FileContents(
                file_path.to_string(),
                file_contents,
            ))
            .unwrap();

        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        match value {
            Some(Value::Object(map)) => {
                assert_eq!(
                    map.get("string")
                        .expect("Could not get file contents as string"),
                    &json!(file_string)
                );
            }
            _ => panic!("Did not get back FileContents"),
        }
    }
}
