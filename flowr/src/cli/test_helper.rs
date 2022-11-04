#[cfg(test)]
pub mod test {
    use std::sync::{Arc, Mutex};

    use flowrlib::services::WAIT;

    use crate::cli::client_server::{ClientConnection, ServerConnection};
    use crate::cli::runtime_messages::{ClientMessage, ServerMessage};

    pub fn wait_for_then_send(
        wait_for_message: ServerMessage,
        then_send: ClientMessage,
    ) -> Arc<Mutex<ServerConnection>> {
        let server_connection = Arc::new(Mutex::new(
            ServerConnection::new("foo")
                .expect("Could not create server connection"),
        ));

        let connection = server_connection.lock()
            .expect("Could not get access to server connection");
        let client_connection = ClientConnection::new("foo")
            .expect("Could not create ClientConnection");

        client_connection
            .send(ClientMessage::Ack)
            .expect("Could not send initial 'Ack' message");

        // background thread that acts as a client that waits for the "wait_for_message" to be sent
        // to it from the server, and once received it replies with the "then_send" message to the server
        std::thread::spawn(move || loop {
            match client_connection.receive::<ServerMessage>() {
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
