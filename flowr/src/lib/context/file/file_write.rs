use std::sync::{Arc, Mutex};

use serde_json::Value;

use flowcore::{Implementation, RUN_AGAIN, RunAgain};

use crate::client_server::ServerConnection;
use crate::runtime_messages::{ClientMessage, ServerMessage};

/// `Implementation` struct for the `file_write` function
pub struct FileWrite {
    /// It holds a reference to the runtime client in order to get file contents
    pub server_connection: Arc<Mutex<ServerConnection>>,
}

impl Implementation for FileWrite {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        if inputs.len() == 2 {
            let filename = &inputs[0];
            let bytes = &inputs[1];

            if let Ok(mut server) = self.server_connection.lock() {
                if let Some(byte_array) = bytes.as_array() {
                    let bytes = byte_array
                        .iter()
                        .map(|byte_value| byte_value.as_u64().unwrap_or(0) as u8)
                        .collect();
                    let _ = server.send_and_receive_response::<ServerMessage, ClientMessage>(ServerMessage::Write(
                        filename.as_str().unwrap_or("").to_string(),
                        bytes,
                    ));
                };
            }
        }

        (None, RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use serial_test::serial;

    use flowcore::{Implementation, RUN_AGAIN};

    use crate::runtime_messages::{ClientMessage, ServerMessage};

    use super::FileWrite;
    use super::super::super::test_helper::test::wait_for_then_send;

    #[test]
    #[serial(client_server)]
    fn write_file_invalid() {
        let file_path = "/fake/write_test";
        let file_contents = "test text".as_bytes().to_vec();
        let inputs = [json!(file_path)]; // No contents parameter
        let file_write_message = ServerMessage::Write(file_path.to_string(), file_contents);
        let server_connection = wait_for_then_send(file_write_message, ClientMessage::Ack);
        let writer = &FileWrite { server_connection } as &dyn Implementation;
        let (value, run_again) = writer.run(&inputs);
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    #[serial(client_server)]
    fn write_file() {
        let file_path = "/fake/write_test";
        let file_contents = "test text".as_bytes().to_vec();
        let inputs = [json!(file_path), json!(file_contents)];
        let file_write_message = ServerMessage::Write(file_path.to_string(), file_contents);

        let server_connection = wait_for_then_send(file_write_message, ClientMessage::Ack);

        let writer = &FileWrite { server_connection } as &dyn Implementation;

        let (value, run_again) = writer.run(&inputs);

        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }
}
