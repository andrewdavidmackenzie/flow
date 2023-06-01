use std::sync::{Arc, Mutex};

use flowcore::{DONT_RUN_AGAIN, Implementation, RUN_AGAIN, RunAgain};
use flowcore::errors::*;
use serde_json::{json, Value};

use crate::cli::connections::CoordinatorConnection;
use crate::cli::coordinator_message::{ClientMessage, CoordinatorMessage};

/// `Implementation` struct for the `file_read` function
pub struct FileRead {
    /// It holds a reference to the runtime client in order to get file contents
    pub server_connection: Arc<Mutex<CoordinatorConnection>>,
}

impl Implementation for FileRead {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let path = &inputs[0];

        let mut server = self.server_connection.lock()
            .map_err(|_| "Could not lock server")?;

        let response = server.send_and_receive_response::<CoordinatorMessage, ClientMessage>(
            CoordinatorMessage::Read(path.as_str().unwrap_or("").to_string()));

        match response {
            Ok(ClientMessage::FileContents(_path, bytes)) => {
                let mut output_map = serde_json::Map::new();
                output_map.insert("bytes".into(), json!(bytes));
                let string = String::from_utf8(bytes)
                    .map_err(|_| "Could not create Utf8 String")?;
                output_map.insert("string".into(), json!(string));
                Ok((Some(Value::Object(output_map)), RUN_AGAIN))
            }
            _ => Ok((None, DONT_RUN_AGAIN)),
        }
    }
}

#[cfg(test)]
mod test {
    use flowcore::{Implementation, RUN_AGAIN};
    use serde_json::{json, Value};
    use serial_test::serial;

    use crate::cli::coordinator_message::ClientMessage::FileContents;
    use crate::cli::coordinator_message::CoordinatorMessage;
    use crate::cli::test_helper::test::wait_for_then_send;

    use super::FileRead;

    #[test]
    #[serial]
    fn read_file() {
        let file_path = "test_read";
        let file_contents : Vec<u8> = "test text".as_bytes().to_vec();
        let file_string = String::from_utf8(file_contents.clone())
            .expect("Could not create Utf8 String");

        let inputs = [json!(file_path)];
        let file_read_message = CoordinatorMessage::Read(file_path.to_string());

        let server_connection = wait_for_then_send(file_read_message,
        FileContents(file_path.to_string(), file_contents));

        let reader = &FileRead { server_connection } as &dyn Implementation;

        let (value, run_again) = reader.run(&inputs).expect("_file_write() failed");

        assert_eq!(run_again, RUN_AGAIN);
        match value {
            Some(Value::Object(map)) => {
                assert_eq!(map.get("string").expect("Could not get file contents as string"),
                           &json!(file_string))
            },
            _ => panic!("Did not get back FileContents")
        }
    }
}
