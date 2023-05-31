use std::sync::{Arc, Mutex};

use serde_json::Value;

use flowcore::{DONT_RUN_AGAIN, Implementation, RUN_AGAIN, RunAgain};
use flowcore::errors::*;

use crate::cli::connections::CoordinatorConnection;
use crate::cli::coordinator_message::{ClientMessage, CoordinatorMessage};

/// `Implementation` struct for the `readline` function
pub struct Readline {
    /// It holds a reference to the runtime client in order to read input
    pub server_connection: Arc<Mutex<CoordinatorConnection>>,
}

impl Implementation for Readline {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let mut server = self.server_connection.lock()
            .map_err(|_| "Could not lock server")?;

        let prompt = match inputs.get(0) {
            Some(Value::String(prompt)) => prompt.clone(),
            _ => "".into()
        };

        let readline_response = server.send_and_receive_response(
            CoordinatorMessage::GetLine(prompt));

        match readline_response {
            Ok(ClientMessage::Line(contents)) => {
                let mut output_map = serde_json::Map::new();
                if let Ok(value) = serde_json::from_str(&contents) {
                    let _ = output_map.insert("json".into(), value);
                };
                output_map.insert("string".into(), Value::String(contents));
                Ok((Some(Value::Object(output_map)), RUN_AGAIN))
            }
            Ok(ClientMessage::GetLineEof) => {
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
mod test {
    use serde_json::json;
    use serde_json::Value;
    use serial_test::serial;

    use flowcore::{DONT_RUN_AGAIN, Implementation, RUN_AGAIN};

    use crate::cli::coordinator_message::{ClientMessage, CoordinatorMessage};
    use crate::cli::test_helper::test::wait_for_then_send;

    use super::Readline;

    #[test]
    #[serial]
    fn gets_a_line_of_text() {
        let server_connection = wait_for_then_send(
            CoordinatorMessage::GetLine("".into()),
            ClientMessage::Line("line of text".into()),
        );
        let reader = &Readline { server_connection } as &dyn Implementation;
        let (value, run_again) = reader.run(&[]).expect("_readline() failed");

        assert_eq!(run_again, RUN_AGAIN);

        let val = value.expect("Could not get value returned from implementation");
        let map = val.as_object().expect("Could not get map of output values");
        assert_eq!(
            map.get("string").expect("Could not get string args"),
            &json!("line of text")
        );
    }

    #[test]
    #[serial]
    fn gets_json() {
        let server_connection = wait_for_then_send(
            CoordinatorMessage::GetLine("".into()),
            ClientMessage::Line("\"json text\"".into()),
        );
        let reader = &Readline { server_connection } as &dyn Implementation;
        let (value, run_again) = reader.run(&[]).expect("_readline() failed");

        assert_eq!(run_again, RUN_AGAIN);

        let val = value.expect("Could not get value returned from implementation");
        let map = val.as_object().expect("Could not get map of output values");
        assert_eq!(
            map.get("json").expect("Could not get json args"),
            &json!("json text")
        );
    }

    #[test]
    #[serial]
    fn get_eof() {
        let server_connection =
            wait_for_then_send(CoordinatorMessage::GetLine("".into()),
                               ClientMessage::GetLineEof);
        let reader = &Readline { server_connection } as &dyn Implementation;
        let (value, run_again) = reader.run(&[]).expect("_readline() failed");

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
