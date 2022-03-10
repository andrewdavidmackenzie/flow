use std::sync::{Arc, Mutex};

use serde_json::Value;

use flowcore::{Implementation, RUN_AGAIN, RunAgain};
use flowcore::errors::*;

use crate::client_server::ServerConnection;
use crate::runtime_messages::{ClientMessage, ServerMessage};

/// `Implementation` struct for the `file_write` function
pub struct FileWrite {
    /// It holds a reference to the runtime client in order to get file contents
    pub server_connection: Arc<Mutex<ServerConnection>>,
}

impl Implementation for FileWrite {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let filename = &inputs[0];
        let bytes = &inputs[1];

        let mut server = self.server_connection.lock()
            .map_err(|_| "Could not lock server")?;

        let byte_array = bytes.as_array().ok_or("Could not get bytes")?;

        let bytes = byte_array
            .iter()
            .map(|byte_value| byte_value.as_u64().unwrap_or(0) as u8)
            .collect();
        let _ = server.send_and_receive_response::<ServerMessage, ClientMessage>(ServerMessage::Write(
            filename.as_str().unwrap_or("").to_string(),
            bytes,
        ));

        Ok((None, RUN_AGAIN))
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use serial_test::serial;

    use flowcore::{Implementation, RUN_AGAIN};

    use crate::runtime_messages::{ClientMessage, ServerMessage};
    use crate::test_helper::test::wait_for_then_send;

    use super::FileWrite;

    #[test]
    #[serial]
    fn write_file() {
        let file_path = "/fake/write_test";
        let file_contents = "test text".as_bytes().to_vec();
        let inputs = [json!(file_path), json!(file_contents)];
        let file_write_message = ServerMessage::Write(file_path.to_string(), file_contents);

        let server_connection = wait_for_then_send(file_write_message, ClientMessage::Ack);

        let writer = &FileWrite { server_connection } as &dyn Implementation;

        let (value, run_again) = writer.run(&inputs).expect("_file_write() failed");

        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }
}
