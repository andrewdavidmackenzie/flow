use std::fmt::Display;

/// This is the message-queue implementation of the lib.client_server communications
use log::{info, trace};
use portpicker::pick_unused_port;
use zmq::Socket;

use flowcore::errors::*;
use flowrlib::services::{CLIENT_SERVER_DISCOVERY_PORT, discover_service, enable_service_discovery, WAIT};

/// `ClientConnection` stores information related to the connection from a runtime client
/// to the runtime server and is used each time a message is to be sent or received.
pub struct ClientConnection {
    requester: Socket,
}

impl ClientConnection {
    /// Create a new connection between client and server
    pub fn new(service_name: &str) -> Result<Self> {
        let server_address = discover_service(CLIENT_SERVER_DISCOVERY_PORT, service_name)?;

        info!("Client will attempt to connect to service '{service_name}' at: '{server_address}'");

        let context = zmq::Context::new();

        let requester = context
            .socket(zmq::REQ)
            .chain_err(|| "Runtime client could not connect to service")?;

        requester
            .connect(&format!("tcp://{server_address}"))
            .chain_err(|| "Could not connect to service")?;

        info!("Client connected to service '{service_name}' at '{server_address}'");

        Ok(ClientConnection { requester })
    }

    /// Receive a ServerMessage from the server
    pub fn receive<SM>(&self) -> Result<SM>
    where
        SM : From<String> + Display {
        trace!("Client waiting for message from server");

        let msg = self
            .requester
            .recv_msg(0)
            .map_err(|e| format!("Error receiving from service: {e}"))?;

        let message_string = msg.as_str().ok_or("Could not get message as str")?
            .to_string();
        let message: SM = message_string.into();
        trace!("Client Received <--- {}", message);
        Ok(message)
    }

    /// Send a ClientMessage to the Server
    pub fn send<CM>(&self, message: CM) -> Result<()>
    where
        CM: Into<String> + Display {
        trace!("Client Sent     ---> {}", message);
        self.requester
            .send(&message.into(), 0)
            .chain_err(|| "Error sending to service")
    }
}

/// `ServerConnection` store information about the server side of the client/server
/// communications between a runtime client and a runtime server and is used each time a message
/// needs to be sent or received.
pub struct ServerConnection {
    responder: Socket,
}

/// Implement a `ServerConnection` for sending and receiving messages between client and server
impl ServerConnection {
    /// Create a new Server side of the client/server Connection
    pub fn new(service_name: &'static str) -> Result<Self> {
        let context = zmq::Context::new();
        let responder = context
            .socket(zmq::REP)
            .chain_err(|| "Server Connection - could not create Socket")?;

        let port = pick_unused_port().chain_err(|| "No ports free")?;

        responder.bind(&format!("tcp://*:{port}"))
            .chain_err(|| "Server Connection - could not bind on TCP Socket")?;

        enable_service_discovery(CLIENT_SERVER_DISCOVERY_PORT, service_name, port)?;
        info!("Service '{}' listening on *:{}", service_name, port);

        Ok(ServerConnection {
            responder
        })
    }

    /// Receive a Message sent from the client to the server
    pub fn receive<CM>(&self, flags: i32) -> Result<CM>
    where
        CM: From<String> + Display {
        trace!("Server waiting for message from client");

        let msg = self
            .responder
            .recv_msg(flags)
            .map_err(|e| format!("Server error getting message: '{e}'"))?;

        let message_string = msg.as_str().ok_or("Could not get message as str")?
            .to_string();
        let message = message_string.into();
        trace!("                ---> Server Received {}", message);
        Ok(message)
    }

    /// Send a Message from the server to the Client and wait for it's response
    pub fn send_and_receive_response<SM, CM>(&mut self, message: SM) -> Result<CM>
    where
        SM: Into<String> + Display,
        CM: From<String> + Display {
        self.send(message)?;
        self.receive(WAIT)
    }

    /// Send a Message from the server to the Client but don't wait for it's response
    pub fn send<SM>(&mut self, message: SM) -> Result<()>
    where
        SM: Into<String> + Display {
        trace!("                <--- Server Sent {}", message);

        self.responder
            .send(&message.into(), 0)
            .map_err(|e| format!("Server error sending to client: '{e}'"))?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::fmt;
    use std::time::Duration;

    use serde_derive::{Deserialize, Serialize};
    use serial_test::serial;

    use flowrlib::services::{DONT_WAIT, WAIT};

    use crate::cli::client_server::{ClientConnection, ServerConnection};

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
            write!(f, "ClientMessage Hello",)
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
        let mut server = ServerConnection::new("test")
            .expect("Could not create ServerConnection");
        let client = ClientConnection::new("test")
            .expect("Could not create ClientConnection");

        // Open the connection by sending the first message from the client
        client
            .send(ClientMessage::Hello)
            .expect("Could not send initial 'Hello' message");

        // Receive and check it on the server
        let client_message = server
            .receive::<ClientMessage>(WAIT)
            .expect("Could not receive message at server");
        assert_eq!(client_message, ClientMessage::Hello);

        // Respond from the server
        server
            .send(ServerMessage::World)
            .expect("Could not send server message");

        // Receive it and check it on the client
        let server_message = client
            .receive::<ServerMessage>()
            .expect("Could not receive message at client");
        assert_eq!(server_message, ServerMessage::World);
    }

    #[test]
    #[serial]
    fn server_receive_nowait_get_reply() {
        let mut server = ServerConnection::new("test")
            .expect("Could not create ServerConnection");
        let client = ClientConnection::new("test")
            .expect("Could not create ClientConnection");

        // Open the connection by sending the first message from the client
        client
            .send(ClientMessage::Hello)
            .expect("Could not send initial 'Hello' message");

        std::thread::sleep(Duration::from_millis(10));

        assert_eq!(
            server
                .receive::<ClientMessage>(DONT_WAIT)
                .expect("Could not receive message at server"),
            ClientMessage::Hello
        );

        // Respond from the server
        server
            .send(ServerMessage::World)
            .expect("Could not send server message");

        // Receive it and check it on the client
        assert_eq!(
            client
                .receive::<ServerMessage>()
                .expect("Could not receive message at client"),
            ServerMessage::World
        );
    }
}
