use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use flowcore::{Implementation, RunAgain, DONT_RUN_AGAIN};

use crate::client_server::ServerConnection;
use crate::runtime_messages::{ClientMessage, ServerMessage};

/// `Implementation` struct for the `get` function
pub struct Get {
    /// It holds a reference to the runtime client in order to Get the Args
    pub server_connection: Arc<Mutex<ServerConnection<ServerMessage, ClientMessage>>>,
}

impl Implementation for Get {
    fn run(&self, mut _inputs: &[Value]) -> (Option<Value>, RunAgain) {
        if let Ok(mut guard) = self.server_connection.lock() {
            return match guard.send_and_receive_response(ServerMessage::GetArgs) {
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
                    // And add the array of Value at the "/json" route
                    let _ = output_map.insert("json".into(), Value::Array(json_arg_vec));

                    // Add the array of (unparsed) text values of the args at "/string" route
                    output_map.insert("string".into(), json!(arg_vec));

                    (Some(Value::Object(output_map)), DONT_RUN_AGAIN)
                }
                _ => (None, DONT_RUN_AGAIN),
            };
        }
        (None, DONT_RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, Mutex};

    use serial_test::serial;

    use flowcore::{Implementation, DONT_RUN_AGAIN};

    use crate::client_server::{ClientConnection, ServerConnection};
    use crate::coordinator::RUNTIME_SERVICE_NAME;
    //    use crate::runtime_messages::ClientMessage::Args;
    //    use crate::runtime_messages::ServerMessage::GetArgs;
    use crate::runtime_messages::{ClientMessage, ServerMessage};

    use super::Get;

    #[test]
    #[serial(client_server)]
    fn gets_args_no_client() {
        let getter = &Get {
            server_connection: Arc::new(Mutex::new(
                ServerConnection::new(RUNTIME_SERVICE_NAME, None)
                    .expect("Could not create server connection"),
            )),
        } as &dyn Implementation;
        let (value, run_again) = getter.run(&[]);

        assert_eq!(run_again, DONT_RUN_AGAIN);
        assert_eq!(value, None);
    }

    fn wait_for_then_send(wait_for_message: ServerMessage, then_send: ClientMessage) {
        let client_connection =
            ClientConnection::<ServerMessage, ClientMessage>::new(RUNTIME_SERVICE_NAME, None)
                .expect("Could not create ClientConnection");

        client_connection
            .send(ClientMessage::Ack)
            .expect("Could not send initial 'Ack' message");

        std::thread::spawn(move || loop {
            match client_connection.receive() {
                Ok(received_message) => {
                    if received_message == wait_for_message {
                        client_connection
                            .send(then_send)
                            .expect("Could not send ClientMessage");

                        return;
                    }
                }
                _ => panic!("Error receiving ServerMessage"),
            }
        });
    }

    /*
    fn server_connection() -> Arc<Mutex<ServerConnection<ServerMessage, ClientMessage>>> {
        Arc::new(Mutex::new(
            ServerConnection::new(RUNTIME_SERVICE_NAME, None)
                .expect("Could not create server connection"),
        ))
    }

    #[test]
    #[serial(client_server)]
    fn gets_args() {
        let server_connection = server_connection();

        let getter = &Get {
            server_connection: server_connection.clone(),
        } as &dyn Implementation;

        let args = vec!["flow_name", "arg1", "arg2"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        wait_for_then_send(GetArgs, Args(args));

        // Get the initial Ack sent from client to open the connection
        let guard = server_connection
            .lock()
            .expect("Could not get a lock on the server connection");
        guard
            .receive()
            .expect("Could not receive initial Ack message from client");

        let (value, run_again) = getter.run(&[]);

        assert_eq!(run_again, DONT_RUN_AGAIN);

        let val = value.expect("Could not get value returned from implementation");
        let map = val.as_object().expect("Could not get map of output values");
        assert!(map.contains_key("json"));
        assert!(map.contains_key("string"));
    }
     */
}
