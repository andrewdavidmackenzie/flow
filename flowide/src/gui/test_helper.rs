#[cfg(test)]
pub mod test {
    use std::sync::{Arc, Mutex};

    use portpicker::pick_unused_port;

    use crate::gui::connections::{ClientConnection, CoordinatorConnection, discover_service,
                                  enable_service_discovery, WAIT};
    use crate::gui::coordinator_message::{ClientMessage, CoordinatorMessage};

    pub fn wait_for_then_send(
        wait_for_message: CoordinatorMessage,
        then_send: ClientMessage,
    ) -> Arc<Mutex<CoordinatorConnection>> {
        let test_port = pick_unused_port().expect("No ports free");
        let server_connection = Arc::new(Mutex::new(
            CoordinatorConnection::new("foo", test_port)
                .expect("Could not create server connection"),
        ));
        let discovery_port = pick_unused_port().expect("No ports free");
        enable_service_discovery(discovery_port, "foo",
                                 test_port).expect("Could not enable service discovery");

        let connection = server_connection.lock()
            .expect("Could not get access to server connection");

        let server_address = discover_service(discovery_port, "foo")
            .expect("Could discovery service");
        let client_connection = ClientConnection::new(&server_address)
            .expect("Could not create ClientConnection");

        client_connection
            .send(ClientMessage::Ack)
            .expect("Could not send initial 'Ack' message");

        // background thread that acts as a client that waits for the "wait_for_message" to be sent
        // to it from the server, and once received it replies with the "then_send" message to the server
        std::thread::spawn(move || loop {
            match client_connection.receive::<CoordinatorMessage>() {
                Ok(received_message) => {
                    if std::mem::discriminant(&received_message) == std::mem::discriminant(&wait_for_message) {
                        client_connection
                            .send(then_send)
                            .expect("Could not send ClientMessage");

                        return;
                    }
                }
                _ => panic!("Error receiving ServerMessage"),
            }
        });

        // Get the initial Ack sent from client to open the connection
        connection
            .receive::<ClientMessage>(WAIT)
            .expect("Could not receive initial Ack message from client");

        server_connection.clone()
    }
}
