use serde_json::{json, Value};

use flowcore::errors::Result;
use flowcore::{Implementation, RunAgain, DONT_RUN_AGAIN};

use crate::cli::coordinator_message::{ClientMessage, CoordinatorMessage};
use crate::context::ContextIO;

/// `Implementation` struct for the `get` function
pub struct Get {
    pub context_io: ContextIO,
}

impl Implementation for Get {
    fn run(&self, mut _inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let response = self
            .context_io
            .send_and_receive(CoordinatorMessage::GetArgs);

        match response {
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
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use flowcore::{Implementation, DONT_RUN_AGAIN};

    use crate::cli::coordinator_message::{ClientMessage, CoordinatorMessage};
    use crate::context::ContextIO;

    use super::Get;

    fn make_get() -> (
        Get,
        std::sync::mpsc::Receiver<crate::context::ContextRequest>,
    ) {
        let (tx, rx) = std::sync::mpsc::channel();
        let (blocking_tx, _blocking_rx) = std::sync::mpsc::channel();
        (
            Get {
                context_io: ContextIO::new(tx, blocking_tx),
            },
            rx,
        )
    }

    #[test]
    fn gets_args_no_client() {
        let (tx, rx) = std::sync::mpsc::channel();
        let (blocking_tx, _blocking_rx) = std::sync::mpsc::channel();
        let getter = Get {
            context_io: ContextIO::new(tx, blocking_tx),
        };
        drop(rx);
        let (value, run_again) = getter.run(&[]).expect("_get() failed");

        assert_eq!(run_again, DONT_RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    fn gets_args() {
        let args: Vec<String> = ["flow_name", "arg1", "arg2"]
            .iter()
            .map(ToString::to_string)
            .collect();

        let (getter, rx) = make_get();
        let args_clone = args.clone();
        let handle = std::thread::spawn(move || getter.run(&[]));

        let req = rx.recv().expect("No request");
        assert!(matches!(req.message, CoordinatorMessage::GetArgs));
        req.response_tx
            .unwrap()
            .send(ClientMessage::Args(args_clone))
            .unwrap();

        let (value, run_again) = handle.join().unwrap().expect("_get() failed");

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
    fn gets_args_num() {
        let args: Vec<String> = ["flow_name", "10"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();

        let (getter, rx) = make_get();
        let args_clone = args.clone();
        let handle = std::thread::spawn(move || getter.run(&[]));

        let req = rx.recv().expect("No request");
        req.response_tx
            .unwrap()
            .send(ClientMessage::Args(args_clone))
            .unwrap();

        let (value, run_again) = handle.join().unwrap().expect("_get() failed");

        assert_eq!(run_again, DONT_RUN_AGAIN);

        let val = value.expect("Could not get value returned from implementation");
        let map = val.as_object().expect("Could not get map of output values");
        let json = map
            .get("json")
            .expect("Could not get json args")
            .as_array()
            .expect("Could not get json map as an array of values");
        assert_eq!(
            json.get(1).expect("Could not get get element 1"),
            &json!(10)
        );
    }

    #[test]
    fn gets_args_array_num() {
        let args: Vec<String> = ["flow_name", "[10,20]"]
            .iter()
            .map(ToString::to_string)
            .collect();

        let (getter, rx) = make_get();
        let args_clone = args.clone();
        let handle = std::thread::spawn(move || getter.run(&[]));

        let req = rx.recv().expect("No request");
        req.response_tx
            .unwrap()
            .send(ClientMessage::Args(args_clone))
            .unwrap();

        let (value, run_again) = handle.join().unwrap().expect("_get() failed");

        assert_eq!(run_again, DONT_RUN_AGAIN);

        let val = value.expect("Could not get value returned from implementation");
        let map = val.as_object().expect("Could not get map of output values");
        let json = map
            .get("json")
            .expect("Could not get json args")
            .as_array()
            .expect("Could not get json map as an array of values");
        assert_eq!(
            json.get(1).expect("Could not get get element 1"),
            &json!([10, 20])
        );
    }

    #[test]
    fn gets_args_array_array_num() {
        let args: Vec<String> = ["flow_name", "[[10,20],[30,40]]"]
            .iter()
            .map(ToString::to_string)
            .collect();

        let (getter, rx) = make_get();
        let args_clone = args.clone();
        let handle = std::thread::spawn(move || getter.run(&[]));

        let req = rx.recv().expect("No request");
        req.response_tx
            .unwrap()
            .send(ClientMessage::Args(args_clone))
            .unwrap();

        let (value, run_again) = handle.join().unwrap().expect("_get() failed");

        assert_eq!(run_again, DONT_RUN_AGAIN);

        let val = value.expect("Could not get value returned from implementation");
        let map = val.as_object().expect("Could not get map of output values");
        let json = map
            .get("json")
            .expect("Could not get json args")
            .as_array()
            .expect("Could not get json map as an array of values");
        assert_eq!(
            json.get(1).expect("Could not get get element 1"),
            &json!([[10, 20], [30, 40]])
        );
    }
}
