use std::sync::{Arc, Mutex};

use serde_json::Value;

use flowcore::{Implementation, RunAgain, RUN_AGAIN};

use crate::client_server::ServerConnection;
use crate::runtime_messages::{ClientMessage, ServerMessage};

/// `Implementation` struct for the `file_write` function
pub struct FileWrite {
    /// It holds a reference to the runtime client in order to get file contents
    pub server_connection: Arc<Mutex<ServerConnection<ServerMessage, ClientMessage>>>,
}

impl Implementation for FileWrite {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        if inputs.len() == 2 {
            let filename = &inputs[0];
            let bytes = &inputs[1];

            if let Ok(mut server) = self.server_connection.lock() {
                return match bytes.as_str() {
                    Some(string) => {
                        match server.send_and_receive_response(ServerMessage::Write(
                            filename.to_string(),
                            string.as_bytes().to_vec(),
                        )) {
                            Ok(ClientMessage::Ack) => (None, RUN_AGAIN),
                            _ => (None, RUN_AGAIN),
                        }
                    }
                    None => (None, RUN_AGAIN),
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

    use super::super::super::test_helper::test::wait_for_then_send;
    use super::FileWrite;

    #[test]
    #[serial(client_server)]
    fn write_file() {
        let file_path = "/fake/write_test".to_string();
        let file_contents = "test text".as_bytes().to_vec();
        let inputs = [json!(file_path), json!(file_contents)];
        let file_write_message = ServerMessage::Write(file_path, file_contents);

        let server_connection = wait_for_then_send(file_write_message, ClientMessage::Ack);

        let writer = &FileWrite { server_connection } as &dyn Implementation;

        let (value, run_again) = writer.run(&inputs);

        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }
}
