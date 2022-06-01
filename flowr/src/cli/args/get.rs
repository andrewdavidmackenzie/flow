use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use flowcore::{DONT_RUN_AGAIN, Implementation, RunAgain};
use flowcore::errors::Result;

use crate::cli::client_server::ServerConnection;
use crate::cli::runtime_messages::{ClientMessage, ServerMessage};

/// `Implementation` struct for the `get` function
pub struct Get {
    /// It holds a reference to the runtime client in order to Get the Args
    pub server_connection: Arc<Mutex<ServerConnection>>,
}

impl Implementation for Get {
    fn run(&self, mut _inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let mut guard = self.server_connection.lock()
            .map_err(|_| "Could not lock server")?;

        match guard.send_and_receive_response(ServerMessage::GetArgs) {
            Ok(ClientMessage::Args(arg_vec)) => {
                let mut output_map = serde_json::Map::new();

                // Construct an array of args parsed into Json Values
                let mut json_arg_vec: Vec<Value> = Vec::new();
                for arg in &arg_vec {
                    if let Ok(json) = serde_json::from_str(arg) {
                        json_arg_vec.push(json);
                    } else {
                        json_arg_vec.push(serde_json::Value::String(arg.into()))
                    }
                }
                // Add the json Array of args at the "/json" output route
                output_map.insert("json".into(), Value::Array(json_arg_vec));

                // Add the array of (unparsed) text values of the args at "/string" route
                output_map.insert("string".into(), json!(arg_vec));

                Ok((Some(Value::Object(output_map)), DONT_RUN_AGAIN))
            }
            _ => Ok((None, DONT_RUN_AGAIN)),
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, Mutex};

    use serde_json::json;
    use serial_test::serial;

    use flowcore::{DONT_RUN_AGAIN, Implementation};

    use crate::cli::client_server::{Method, RUNTIME_SERVICE_NAME, ServerConnection};
    use crate::cli::runtime_messages::ClientMessage::Args;
    use crate::cli::runtime_messages::ServerMessage::GetArgs;
    use crate::cli::test_helper::test::wait_for_then_send;

    use super::Get;

    #[test]
    #[serial]
    fn gets_args_no_client() {
        let getter = &Get {
            server_connection: Arc::new(Mutex::new(
                ServerConnection::new(RUNTIME_SERVICE_NAME, Method::Tcp(None))
                    .expect("Could not create server connection"),
            )),
        } as &dyn Implementation;
        let (value, run_again) = getter.run(&[]).expect("_get() failed");

        assert_eq!(run_again, DONT_RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    #[serial]
    fn gets_args() {
        let args: Vec<String> = vec!["flow_name", "arg1", "arg2"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let server_connection = wait_for_then_send(GetArgs, Args(args.clone()));

        let getter = &Get { server_connection } as &dyn Implementation;

        let (value, run_again) = getter.run(&[]).expect("_get() failed");

        assert_eq!(run_again, DONT_RUN_AGAIN);

        let val = value.expect("Could not get value returned from implementation");
        let map = val.as_object().expect("Could not get map of output values");
        assert!(map.contains_key("string"));
        assert_eq!(
            map.get("json").expect("Could not get json args"),
            &json!(args)
        );
    }
}
