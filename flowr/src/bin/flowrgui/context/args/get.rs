use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use flowcore::{DONT_RUN_AGAIN, Implementation, RunAgain};
use flowcore::errors::Result;

use crate::gui::client_message::ClientMessage;
use crate::gui::coordinator_connection::CoordinatorConnection;
use crate::gui::coordinator_message::CoordinatorMessage;

/// `Implementation` struct for the `get` function
pub struct Get {
    /// It holds a reference to the runtime client in order to Get the Args
    pub server_connection: Arc<Mutex<CoordinatorConnection>>,
}

impl Implementation for Get {
    fn run(&self, mut _inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let mut guard = self.server_connection.lock()
            .map_err(|_| "Could not lock server")?;

        let sent = guard.send_and_receive_response(CoordinatorMessage::GetArgs);

        match sent {
            Ok(ClientMessage::Args(arg_vec)) => {
                let mut output_map = serde_json::Map::new();

                // Construct an array of args parsed into Json Values
                let mut json_arg_vec: Vec<Value> = Vec::new();
                for arg in &arg_vec {
                    if let Ok(json) = serde_json::from_str(arg) {
                        json_arg_vec.push(json);
                    } else {
                        json_arg_vec.push(Value::String(arg.into()));
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

    use portpicker::pick_unused_port;
    use serde_json::json;
    use serial_test::serial;

    use flowcore::{DONT_RUN_AGAIN, Implementation};

    use crate::gui::client_message::ClientMessage::Args;
    use crate::gui::coordinator_connection::CoordinatorConnection;
    use crate::gui::coordinator_message::CoordinatorMessage::GetArgs;
    use crate::gui::test_helper::test::wait_for_then_send;

    use super::Get;

    #[test]
    #[serial]
    fn gets_args_no_client() {
        let test_port = pick_unused_port().expect("No ports free");
        let getter = &Get {
            server_connection: Arc::new(Mutex::new(
                CoordinatorConnection::new("foo", test_port)
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
        let args: Vec<String> = ["flow_name", "arg1", "arg2"]
            .iter()
            .map(ToString::to_string)
            .collect();

        let server_connection = wait_for_then_send(GetArgs, Args(args.clone()));

        let getter = &Get { server_connection } as &dyn Implementation;

        let (value, run_again) = getter.run(&[]).expect("_get() failed");

        assert_eq!(run_again, DONT_RUN_AGAIN);

        let val = value.expect("Could not get value returned from implementation");
        let map = val.as_object().expect("Could not get map of output values");
        assert!(map.contains_key("json"));
        assert_eq!(
            map.get("json").expect("Could not get json args"),
            &json!(args)
        );
    }

    #[test]
    #[serial]
    fn gets_args_num() {
        let args: Vec<String> = ["flow_name", "10"]
            .iter()
            .map(ToString::to_string)
            .collect();

        let server_connection = wait_for_then_send(GetArgs, Args(args));

        let getter = &Get { server_connection } as &dyn Implementation;

        let (value, run_again) = getter.run(&[]).expect("_get() failed");

        assert_eq!(run_again, DONT_RUN_AGAIN);

        let val = value.expect("Could not get value returned from implementation");
        let map = val.as_object().expect("Could not get map of output values");
        let json = map.get("json").expect("Could not get json args")
            .as_array().expect("Could not get json map as an array of values");
        assert_eq!(json.get(1).expect("Could not get get element 1"), &json!(10));
    }

    #[test]
    #[serial]
    fn gets_args_array_num() {
        let args: Vec<String> = ["flow_name", "[10,20]"]
            .iter()
            .map(ToString::to_string)
            .collect();

        let server_connection = wait_for_then_send(GetArgs, Args(args));

        let getter = &Get { server_connection } as &dyn Implementation;

        let (value, run_again) = getter.run(&[]).expect("_get() failed");

        assert_eq!(run_again, DONT_RUN_AGAIN);

        let val = value.expect("Could not get value returned from implementation");
        let map = val.as_object().expect("Could not get map of output values");
        let json = map.get("json").expect("Could not get json args")
            .as_array().expect("Could not get json map as an array of values");
        assert_eq!(json.get(1).expect("Could not get get element 1"), &json!([10,20]));
    }

    #[test]
    #[serial]
    fn gets_args_array_array_num() {
        let args: Vec<String> = ["flow_name", "[[10,20],[30,40]]"]
            .iter()
            .map(ToString::to_string)
            .collect();

        let server_connection = wait_for_then_send(GetArgs, Args(args));

        let getter = &Get { server_connection } as &dyn Implementation;

        let (value, run_again) = getter.run(&[]).expect("_get() failed");

        assert_eq!(run_again, DONT_RUN_AGAIN);

        let val = value.expect("Could not get value returned from implementation");
        let map = val.as_object().expect("Could not get map of output values");
        let json = map.get("json").expect("Could not get json args")
            .as_array().expect("Could not get json map as an array of values");
        assert_eq!(json.get(1).expect("Could not get get element 1"), &json!([[10,20], [30,40]]));
    }
}
