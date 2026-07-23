use flowcore::errors::Result;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};
use serde_json::Value;

use crate::cli::coordinator_message::CoordinatorMessage;
use crate::context::ContextIO;

/// `Implementation` struct for the `file_write` function
pub struct FileWrite {
    pub context_io: ContextIO,
}

impl Implementation for FileWrite {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let filename = inputs.first().ok_or("Could not get filename")?;
        let bytes = inputs.get(1).ok_or("Could not get bytes")?;

        let byte_array = bytes.as_array().ok_or("Could not get bytes")?;

        #[allow(clippy::cast_possible_truncation)]
        let bytes = byte_array
            .iter()
            .map(|byte_value| byte_value.as_u64().unwrap_or(0) as u8)
            .collect();

        self.context_io.send_nonblocking(CoordinatorMessage::Write(
            filename.as_str().unwrap_or("").to_string(),
            bytes,
        ))?;

        Ok((None, RUN_AGAIN))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use flowcore::{Implementation, RUN_AGAIN};
    use serde_json::json;

    use crate::cli::coordinator_message::{ClientMessage, CoordinatorMessage};
    use crate::context::ContextIO;

    use super::FileWrite;

    fn make_file_write() -> (
        FileWrite,
        std::sync::mpsc::Receiver<crate::context::ContextRequest>,
    ) {
        let (tx, rx) = std::sync::mpsc::channel();
        let (dummy_tx, _dummy_rx) = std::sync::mpsc::channel();
        (
            FileWrite {
                context_io: ContextIO::new(dummy_tx, tx),
            },
            rx,
        )
    }

    #[test]
    fn write_file() {
        let file_path = std::path::Path::new("fake").join("write_test");
        let file_path = file_path.to_str().expect("Could not convert path");
        let file_contents = "test text".as_bytes().to_vec();
        let inputs = [json!(file_path), json!(file_contents)];

        let (writer, rx) = make_file_write();
        let handle = std::thread::spawn(move || writer.run(&inputs));

        let req = rx.recv().expect("No request received");
        assert!(matches!(req.message, CoordinatorMessage::Write(..)));
        if let Some(response_tx) = req.response_tx {
            response_tx
                .send(ClientMessage::Ack)
                .expect("Could not send response");
        }

        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }
}
