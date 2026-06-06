//! ZMQ-based connections for client-coordinator communication.
//!
//! Provides [`ClientConnection`] (REQ socket) and [`CoordinatorConnection`] (REP socket)
//! used by runners and debug clients to communicate with the coordinator.

use std::fmt::Display;

use log::{debug, info, trace};
use zmq::Socket;

use flowcore::errors::{Result, ResultExt};

/// WAIT for a message to arrive when performing a `receive()`
pub const WAIT: i32 = 0;

/// Do NOT WAIT for a message to arrive when performing a `receive()`
pub static DONT_WAIT: i32 = zmq::DONTWAIT;

/// `ClientConnection` stores information related to the connection from a client
/// to the [Coordinator][flowrlib::coordinator::Coordinator] and is used each time a message is to
/// be sent or received.
pub struct ClientConnection {
    requester: Socket,
}

impl ClientConnection {
    /// Create a new connection between client and [Coordinator][crate::coordinator::Coordinator]
    ///
    /// # Errors
    /// Returns an error if the ZMQ socket cannot be created or connected
    pub fn new(coordinator_address: &str) -> Result<Self> {
        info!("Client will attempt to connect to coordinator at: '{coordinator_address}'");

        let context = zmq::Context::new();

        let requester = context
            .socket(zmq::REQ)
            .chain_err(|| "Client could not connect to coordinator service")?;

        requester
            .connect(&format!("tcp://{coordinator_address}"))
            .chain_err(|| {
                format!("Client Connection - Could not connect to socket at: {coordinator_address}")
            })?;

        // Set a receive timeout so the client doesn't hang forever if the server exits
        requester
            .set_rcvtimeo(5_000)
            .chain_err(|| "Could not set receive timeout")?;

        info!("Client connected to coordinator at '{coordinator_address}'");

        Ok(ClientConnection { requester })
    }

    /// Receive a message from the coordinator
    ///
    /// # Errors
    /// Returns an error if the message cannot be received or deserialized
    pub fn receive<CM>(&self) -> Result<CM>
    where
        CM: From<String> + Display,
    {
        trace!("Client waiting for message from coordinator");

        let msg = self
            .requester
            .recv_msg(0)
            .map_err(|e| format!("Error receiving from coordinator: {e}"))?;

        let message_string = msg
            .as_str()
            .ok_or("Could not get message as str")?
            .to_string();
        let message: CM = message_string.into();
        trace!("Client Received <--- {message}");
        Ok(message)
    }

    /// Send a message to the coordinator
    ///
    /// # Errors
    /// Returns an error if the message cannot be sent
    pub fn send<CM>(&self, message: CM) -> Result<()>
    where
        CM: Into<String> + Display,
    {
        trace!("Client Sent     ---> {message}");
        self.requester
            .send(&message.into(), 0)
            .chain_err(|| "Error sending to coordinator")
    }
}

/// [`CoordinatorConnection`] store information about the [Coordinator][flowrlib::coordinator::Coordinator]
/// side of the client/coordinator communications between a client and a [Coordinator][flowrlib::coordinator::Coordinator]
/// and is used each time a message needs to be sent or received.
pub struct CoordinatorConnection {
    responder: Socket,
}

/// Implement a [`CoordinatorConnection`] for sending and receiving messages between client and
/// a [Coordinator][flowrlib::coordinator::Coordinator]
impl CoordinatorConnection {
    /// Create a new coordinator-side connection that listens on the given port
    ///
    /// # Errors
    /// Returns an error if the ZMQ socket cannot be created or bound
    pub fn new(service_name: &str, port: u16) -> Result<Self> {
        let context = zmq::Context::new();
        let responder = context
            .socket(zmq::REP)
            .chain_err(|| "Coordinator Connection - could not create Socket")?;

        debug!("Coordinator Connection attempting to bind to: tcp://*:{port}");
        responder.bind(&format!("tcp://*:{port}")).chain_err(|| {
            format!("Coordinator Connection - could not bind on TCP Socket on: tcp://{port}")
        })?;

        info!("Service '{service_name}' listening on *:{port}");

        Ok(CoordinatorConnection { responder })
    }

    /// Receive a message from the client
    ///
    /// # Errors
    /// Returns an error if the message cannot be received or deserialized
    pub fn receive<CM>(&self, flags: i32) -> Result<CM>
    where
        CM: From<String> + Display,
    {
        trace!("Coordinator waiting for message from client");

        let msg = self
            .responder
            .recv_msg(flags)
            .map_err(|e| format!("Coordinator error getting message: '{e}'"))?;

        let message_string = msg
            .as_str()
            .ok_or("Could not get message as str")?
            .to_string();
        let message = message_string.into();
        trace!("                ---> Coordinator Received {message}");
        Ok(message)
    }

    /// Send a message to the client and wait for its response
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

    /// Send a message to the client without waiting for a response
    ///
    /// # Errors
    /// Returns an error if the message cannot be sent
    pub fn send<SM>(&mut self, message: SM) -> Result<()>
    where
        SM: Into<String> + Display,
    {
        trace!("                <--- Coordinator Sent {message}");

        self.responder
            .send(&message.into(), 0)
            .map_err(|e| format!("Coordinator error sending to client: '{e}'"))?;

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

    use super::{ClientConnection, CoordinatorConnection, DONT_WAIT, WAIT};

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    enum CoordinatorMessage {
        World,
    }

    impl fmt::Display for CoordinatorMessage {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(
                f,
                "CoordinatorMessage {}",
                match self {
                    CoordinatorMessage::World => "World",
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

    impl From<CoordinatorMessage> for String {
        fn from(event: CoordinatorMessage) -> Self {
            serde_json::to_string(&event).expect("Could not serialize message")
        }
    }

    impl From<String> for CoordinatorMessage {
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
    fn coordinator_receive_wait_get_reply() {
        let test_port = pick_unused_port().expect("No ports free");
        let service_name = format!("test-{test_port}");
        let mut coordinator_connection = CoordinatorConnection::new(&service_name, test_port)
            .expect("Could not create CoordinatorConnection");

        let client = ClientConnection::new(&format!("localhost:{test_port}"))
            .expect("Could not create ClientConnection");

        // Open the connection by sending the first message from the client
        client
            .send(ClientMessage::Hello)
            .expect("Could not send initial 'Hello' message");

        // Receive and check it on the coordinator
        let client_message = coordinator_connection
            .receive::<ClientMessage>(WAIT)
            .expect("Could not receive message at Coordinator");
        assert_eq!(client_message, ClientMessage::Hello);

        // Respond from the coordinator
        coordinator_connection
            .send(CoordinatorMessage::World)
            .expect("Could not send Coordinator message");

        // Receive it and check it on the client
        let coordinator_message = client
            .receive::<CoordinatorMessage>()
            .expect("Could not receive message at client");
        assert_eq!(coordinator_message, CoordinatorMessage::World);
    }

    #[test]
    #[serial]
    fn coordinator_receive_nowait_get_reply() {
        let test_port = pick_unused_port().expect("No ports free");
        let service_name = format!("test-{test_port}");
        let mut coordinator_connection = CoordinatorConnection::new(&service_name, test_port)
            .expect("Could not create CoordinatorConnection");

        let client = ClientConnection::new(&format!("localhost:{test_port}"))
            .expect("Could not create ClientConnection");

        // Open the connection by sending the first message from the client
        client
            .send(ClientMessage::Hello)
            .expect("Could not send initial 'Hello' message");

        let mut received = None;
        for _ in 0..5 {
            std::thread::sleep(Duration::from_millis(100));
            if let Ok(msg) = coordinator_connection.receive::<ClientMessage>(DONT_WAIT) {
                received = Some(msg);
                break;
            }
        }
        assert_eq!(
            received.expect("Could not receive message at Coordinator after retries"),
            ClientMessage::Hello
        );

        // Respond from the coordinator
        coordinator_connection
            .send(CoordinatorMessage::World)
            .expect("Could not send Coordinator message");

        // Receive it and check it on the client
        assert_eq!(
            client
                .receive::<CoordinatorMessage>()
                .expect("Could not receive message at client"),
            CoordinatorMessage::World
        );
    }
}
