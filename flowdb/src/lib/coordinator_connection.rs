//! [`CoordinatorConnection`] — a ZMQ REP socket connection used by the debug server
//! (in the coordinator) to communicate with debug clients.

use std::fmt::Display;

use log::{debug, info, trace};
use zmq::Socket;

use flowcore::errors::Result;

/// WAIT for a message to arrive when performing a `receive()`
pub const WAIT: i32 = 0;

/// Do NOT WAIT for a message to arrive when performing a `receive()`
pub const DONT_WAIT: i32 = zmq::DONTWAIT;

/// [`CoordinatorConnection`] stores information about the debug server side of the
/// connection between a debug client and the [Coordinator][flowrlib::coordinator::Coordinator].
pub struct CoordinatorConnection {
    responder: Socket,
}

impl CoordinatorConnection {
    /// Create a new debug server connection that listens on the given port
    ///
    /// # Errors
    /// Returns an error if the ZMQ socket cannot be created or bound
    pub fn new(service_name: &str, port: u16) -> Result<Self> {
        let context = zmq::Context::new();
        let responder = context
            .socket(zmq::REP)
            .map_err(|e| format!("Debug server could not create REP socket: {e}"))?;

        debug!("Debug server attempting to bind to: tcp://*:{port}");
        responder
            .bind(&format!("tcp://*:{port}"))
            .map_err(|e| format!("Debug server could not bind on tcp://*:{port}: {e}"))?;

        info!("Debug service '{service_name}' listening on *:{port}");

        Ok(CoordinatorConnection { responder })
    }

    /// Receive a message from the debug client
    ///
    /// # Errors
    /// Returns an error if the message cannot be received or deserialized
    pub fn receive<CM>(&self, flags: i32) -> Result<CM>
    where
        CM: From<String> + Display,
    {
        trace!("Debug server waiting for message from client");

        let msg = self
            .responder
            .recv_msg(flags)
            .map_err(|e| format!("Debug server error getting message: '{e}'"))?;

        let message_string = msg
            .as_str()
            .ok_or("Could not get message as str")?
            .to_string();
        let message = message_string.into();
        trace!("                ---> Debug server received {message}");
        Ok(message)
    }

    /// Send a message to the debug client and wait for its response
    ///
    /// # Errors
    /// Returns an error if the message cannot be sent or the response cannot be received
    pub fn send_and_receive_response<SM, CM>(&mut self, message: SM) -> Result<CM>
    where
        SM: Into<String> + Display,
        CM: From<String> + Display,
    {
        self.send(message)?;
        self.receive(WAIT)
    }

    /// Send a message to the debug client without waiting for a response
    ///
    /// # Errors
    /// Returns an error if the message cannot be sent
    pub fn send<SM>(&mut self, message: SM) -> Result<()>
    where
        SM: Into<String> + Display,
    {
        trace!("                <--- Debug server sent {message}");

        self.responder
            .send(&message.into(), 0)
            .map_err(|e| format!("Debug server error sending to client: '{e}'"))?;

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use std::fmt;
    use std::time::Duration;

    use portpicker::pick_unused_port;
    use serde_derive::{Deserialize, Serialize};
    use serial_test::serial;

    use super::{CoordinatorConnection, DONT_WAIT, WAIT};
    use crate::client_connection::ClientConnection;

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    enum ServerMessage {
        World,
    }

    impl fmt::Display for ServerMessage {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(
                f,
                "ServerMessage {}",
                match self {
                    ServerMessage::World => "World",
                }
            )
        }
    }

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    enum ClientMessage {
        Hello,
    }

    impl fmt::Display for ClientMessage {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "ClientMessage Hello")
        }
    }

    impl From<ServerMessage> for String {
        fn from(event: ServerMessage) -> Self {
            serde_json::to_string(&event).expect("Could not serialize message")
        }
    }

    impl From<String> for ServerMessage {
        fn from(msg: String) -> Self {
            serde_json::from_str(&msg).expect("Could not deserialize message")
        }
    }

    impl From<ClientMessage> for String {
        fn from(msg: ClientMessage) -> Self {
            serde_json::to_string(&msg).expect("Could not convert message to string")
        }
    }

    impl From<String> for ClientMessage {
        fn from(msg: String) -> Self {
            serde_json::from_str(&msg).expect("Could not deserialize message")
        }
    }

    #[test]
    #[serial]
    fn server_receive_wait_get_reply() {
        let test_port = pick_unused_port().expect("No ports free");
        let service_name = format!("test-{test_port}");
        let mut server_connection = CoordinatorConnection::new(&service_name, test_port)
            .expect("Could not create CoordinatorConnection");

        let client = ClientConnection::new(&format!("localhost:{test_port}"))
            .expect("Could not create ClientConnection");

        client
            .send(ClientMessage::Hello)
            .expect("Could not send initial 'Hello' message");

        let client_message = server_connection
            .receive::<ClientMessage>(WAIT)
            .expect("Could not receive message at server");
        assert_eq!(client_message, ClientMessage::Hello);

        server_connection
            .send(ServerMessage::World)
            .expect("Could not send server message");

        let server_message = client
            .receive::<ServerMessage>()
            .expect("Could not receive message at client");
        assert_eq!(server_message, ServerMessage::World);
    }

    #[test]
    #[serial]
    fn server_receive_nowait_get_reply() {
        let test_port = pick_unused_port().expect("No ports free");
        let service_name = format!("test-{test_port}");
        let mut server_connection = CoordinatorConnection::new(&service_name, test_port)
            .expect("Could not create CoordinatorConnection");

        let client = ClientConnection::new(&format!("localhost:{test_port}"))
            .expect("Could not create ClientConnection");

        client
            .send(ClientMessage::Hello)
            .expect("Could not send initial 'Hello' message");

        let mut received = None;
        for _ in 0..5 {
            std::thread::sleep(Duration::from_millis(100));
            if let Ok(msg) = server_connection.receive::<ClientMessage>(DONT_WAIT) {
                received = Some(msg);
                break;
            }
        }
        assert_eq!(
            received.expect("Could not receive message at server after retries"),
            ClientMessage::Hello
        );

        server_connection
            .send(ServerMessage::World)
            .expect("Could not send server message");

        assert_eq!(
            client
                .receive::<ServerMessage>()
                .expect("Could not receive message at client"),
            ServerMessage::World
        );
    }
}
