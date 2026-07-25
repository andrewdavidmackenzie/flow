//! Shared test helpers for `flowrlib` tests.

/// Shared fixtures for manifest, submission and dispatcher construction used by
/// multiple test modules.
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
pub(crate) mod fixtures {
    use portpicker::pick_unused_port;

    use flowcore::model::flow_manifest::FlowManifest;
    use flowcore::model::metadata::MetaData;
    use flowcore::model::runtime_function::RuntimeFunction;

    pub fn get_four_ports() -> (u16, u16, u16, u16) {
        (
            pick_unused_port().expect("No ports free"),
            pick_unused_port().expect("No ports free"),
            pick_unused_port().expect("No ports free"),
            pick_unused_port().expect("No ports free"),
        )
    }

    pub fn get_bind_addresses(ports: (u16, u16, u16, u16)) -> (String, String, String, String) {
        (
            format!("tcp://*:{}", ports.0),
            format!("tcp://*:{}", ports.1),
            format!("tcp://*:{}", ports.2),
            format!("tcp://*:{}", ports.3),
        )
    }

    pub fn test_meta_data() -> MetaData {
        MetaData {
            name: "test".into(),
            version: "0.0.0".into(),
            description: "a test".into(),
            authors: vec!["me".into()],
        }
    }

    pub fn test_manifest(functions: Vec<RuntimeFunction>) -> FlowManifest {
        let mut manifest = FlowManifest::new(test_meta_data());
        for function in functions {
            manifest.add_function(function);
        }
        manifest
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used)]
#[doc(hidden)]
pub mod test {
    use std::fmt::Display;
    use std::sync::{Arc, Mutex};

    use portpicker::pick_unused_port;

    use crate::connections::{ClientConnection, CoordinatorConnection, WAIT};
    use crate::discovery::{discover_service, enable_service_discovery};

    /// Set up a test coordinator connection that waits for a specific message
    /// and replies with another. Returns the server-side connection.
    ///
    /// The `ack` parameter is the initial handshake message sent by the client.
    pub fn wait_for_then_send<CM, SM>(
        wait_for_message: CM,
        then_send: SM,
        ack: SM,
    ) -> Arc<Mutex<CoordinatorConnection>>
    where
        CM: From<String> + Display + Send + 'static,
        SM: Into<String> + From<String> + Display + Send + 'static,
    {
        let test_port = pick_unused_port().expect("No ports free");
        let service_name = format!("test-{test_port}");
        let server_connection = Arc::new(Mutex::new(
            CoordinatorConnection::new(&service_name, test_port)
                .expect("Could not create server connection"),
        ));
        let _mdns = enable_service_discovery(&service_name, test_port)
            .expect("Could not enable service discovery");

        let connection = server_connection
            .lock()
            .expect("Could not get access to server connection");

        let server_address = discover_service(&service_name).expect("Could not discover service");
        let client_connection =
            ClientConnection::new(&server_address).expect("Could not create ClientConnection");

        client_connection
            .send(ack)
            .expect("Could not send initial handshake message");

        std::thread::spawn(move || loop {
            match client_connection.receive::<CM>() {
                Ok(received_message) => {
                    if std::mem::discriminant(&received_message)
                        == std::mem::discriminant(&wait_for_message)
                    {
                        client_connection
                            .send(then_send)
                            .expect("Could not send reply message");
                        return;
                    }
                }
                _ => panic!("Error receiving message"),
            }
        });

        connection
            .receive::<SM>(WAIT)
            .expect("Could not receive initial handshake from client");

        server_connection.clone()
    }
}
