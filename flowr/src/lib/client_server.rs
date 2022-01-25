use std::fmt::Display;
use std::marker::PhantomData;
use std::time::Duration;

/// This is the message-queue implementation of the lib.client_server communications
use log::{info, trace};
use portpicker::pick_unused_port;
use simpdiscoverylib::{BeaconListener, BeaconSender};
use zmq::{DONTWAIT, Message};
use zmq::Socket;

use crate::errors::*;

const FLOW_SERVICE_NAME: &str = "_flowr._tcp.local";

/// Structure that holds information about the Server to help clients connect to it
#[derive(Clone)]
pub struct ServerInfo {
    /// Optional tuple of Server hostname and port to connect to
    pub hostname_and_port: Option<(String, u16)>,
    /// Name of the server service name to connect to
    pub name: String,
}

impl ServerInfo {
    /// Create a new ServerInfo struct
    pub fn new(hostname_and_port: Option<(String, u16)>, name: &str) -> Self {
        ServerInfo {
            hostname_and_port,
            name: name.into(),
        }
    }
}

/// `ClientConnection` stores information related to the connection from a runtime client
/// to the runtime server and is used each time a message is to be sent or received.
pub struct ClientConnection<SM, CM> {
    port: u16,
    requester: Socket,
    phantom: PhantomData<SM>,
    phantom2: PhantomData<CM>,
}

impl<SM, CM> ClientConnection<SM, CM>
where
    SM: From<Message> + Display,
    CM: Into<Message> + Display,
{
    /// Create a new connection between client and server
    pub fn new(server_info: ServerInfo) -> Result<Self> {
        let full_service_name = format!("{}.{}", server_info.name, FLOW_SERVICE_NAME);

        let (hostname, port) = server_info.hostname_and_port.unwrap_or(
            Self::discover_service(&full_service_name)
                .ok_or("Could not discover service hostname & port and none were specified")?,
        );

        info!(
            "Client will attempt to connect to service '{}' at: '{}'",
            full_service_name, hostname
        );

        let context = zmq::Context::new();

        let requester = context
            .socket(zmq::REQ)
            .chain_err(|| "Runtime client could not connect to service")?;

        requester
            .connect(&format!("tcp://{}:{}", hostname, port))
            .chain_err(|| "Could not connect to service")?;

        info!(
            "Client connected to service '{}' on {}:{}",
            full_service_name, hostname, port
        );

        Ok(ClientConnection {
            port,
            requester,
            phantom: PhantomData,
            phantom2: PhantomData,
        })
    }

    // Try to discover a server that a client can send a submission to
    fn discover_service(name: &str) -> Option<(String, u16)> {
        let listener = BeaconListener::new(name.as_bytes()).ok()?;
        info!(
            "Client is waiting for a Service Discovery beacon for '{}'",
            name
        );
        let beacon = listener.wait(Some(Duration::from_secs(10))).ok()?;
        info!(
            "Service '{}' discovered at IP: {}, Port: {}",
            name, beacon.service_ip, beacon.service_port
        );
        Some((beacon.service_ip, beacon.service_port))
    }

    /// Receive a ServerMessage from the server
    pub fn receive(&self) -> Result<SM> {
        trace!("Client waiting for message from server");

        let msg = self
            .requester
            .recv_msg(0)
            .map_err(|e| format!("Error receiving from service: {}", e))?;

        let message = SM::from(msg);
        trace!("Client Received <--- {}", message);
        Ok(message)
    }

    /// Send a ClientMessage to the Server
    pub fn send(&self, message: CM) -> Result<()> {
        trace!("Client Sent     ---> to {} {}", self.port, message);
        self.requester
            .send(message, 0)
            .chain_err(|| "Error sending to service")
    }
}

/// `ServerConnection` store information about the server side of the client/server
/// communications between a runtime client and a runtime server and is used each time a message
/// needs to be sent or received.
pub struct ServerConnection<SM, CM> {
    name: &'static str,
    port: u16,
    responder: zmq::Socket,
    phantom: PhantomData<SM>,
    phantom2: PhantomData<CM>,
}

/// Implement a server connection for sending server messages of type <SM> and receiving
/// back client messages of type <CM>
impl<'a, SM, CM> ServerConnection<SM, CM>
where
    SM: Into<Message> + Display,
    CM: From<Message> + Display,
    zmq::Message: std::convert::From<CM>,
{
    /// Create a new Server side of the client/server Connection
    pub fn new(name: &'static str, port: Option<u16>) -> Result<Self> {
        let context = zmq::Context::new();
        let responder = context
            .socket(zmq::REP)
            .chain_err(|| "Server Connection - could not create Socket")?;

        let chosen_port = port.unwrap_or(pick_unused_port().chain_err(|| "No ports free")?);

        responder
            .bind(&format!("tcp://*:{}", chosen_port))
            .chain_err(|| "Server Connection - could not bind on Socket")?;

        let full_service_name = format!("{}.{}", name, FLOW_SERVICE_NAME);

        Self::enable_service_discovery(&full_service_name, chosen_port)?;

        info!("Service '{}' listening on port {}", name, chosen_port);

        Ok(ServerConnection {
            name,
            port: chosen_port,
            responder,
            phantom: PhantomData,
            phantom2: PhantomData,
        })
    }

    /// Get the `ServerInfo` struct that clients use to connect to the server
    pub fn get_server_info(&self) -> ServerInfo {
        ServerInfo {
            hostname_and_port: Some(("localhost".into(), self.port)),
            name: self.name.into(),
        }
    }

    /*
       Start a background thread that sends out beacons for service discovery by a client every second
    */
    fn enable_service_discovery(name: &str, port: u16) -> Result<()> {
        match BeaconSender::new(port, name.as_bytes()) {
            Ok(beacon) => {
                info!(
                    "Discovery beacon announcing service named '{}', on port: {}",
                    name, port
                );
                std::thread::spawn(move || {
                    let _ = beacon.send_loop(Duration::from_secs(1));
                });
            }
            Err(e) => bail!("Error starting discovery beacon: {}", e.to_string()),
        }

        Ok(())
    }

    /// Receive a Message sent from the client to the server
    pub fn receive(&self) -> Result<CM> {
        trace!("Server waiting for message from client");

        let msg = self
            .responder
            .recv_msg(0)
            .map_err(|e| format!("Server error getting message: '{}'", e))?;

        let message = CM::from(msg);
        trace!(
            "                ---> Server Received on {} {}",
            self.port,
            message
        );
        Ok(message)
    }

    /// Try to Receive a Message sent from the client to the server but without blocking
    pub fn receive_no_wait(&self) -> Result<CM> {
        let msg = self
            .responder
            .recv_msg(DONTWAIT)
            .chain_err(|| "Server could not receive message")?;

        let message = CM::from(msg);
        trace!(
            "                ---> Server Received on {} {}",
            self.port,
            message
        );
        Ok(message)
    }

    /// Send a Message from the server to the Client and wait for it's response
    pub fn send_and_receive_response(&mut self, message: SM) -> Result<CM> {
        self.send(message)?;
        self.receive()
    }

    /// Send a Message from the server to the Client but don't wait for it's response
    pub fn send(&mut self, message: SM) -> Result<()> {
        trace!(
            "                <--- Server Sent on {}: {}",
            self.port,
            message
        );

        self.responder
            .send(message, 0)
            .map_err(|e| format!("Server error sending to client: '{}'", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::fmt;
    use std::time::Duration;

    use serde_derive::{Deserialize, Serialize};
    use serial_test::serial;
    use zmq::Message;

    use crate::client_server::{ClientConnection, ServerConnection, ServerInfo};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
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

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    enum ClientMessage {
        Hello,
    }

    impl fmt::Display for ClientMessage {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "ClientMessage Hello",)
        }
    }

    impl From<ServerMessage> for Message {
        fn from(event: ServerMessage) -> Self {
            Message::from(&serde_json::to_string(&event).expect("Could not serialize message"))
        }
    }

    impl From<Message> for ServerMessage {
        fn from(msg: Message) -> Self {
            serde_json::from_str(msg.as_str().expect("Could not convert message to &str"))
                .expect("Could not deserialize message")
        }
    }

    impl From<ClientMessage> for Message {
        fn from(msg: ClientMessage) -> Self {
            Message::from(
                &serde_json::to_string(&msg).expect("Could not convert message to string"),
            )
        }
    }

    impl From<Message> for ClientMessage {
        fn from(msg: Message) -> Self {
            serde_json::from_str(msg.as_str().expect("Could not convert message to str"))
                .expect("Could not deserialize message")
        }
    }

    #[test]
    #[serial(client_server)]
    fn hello_world() {
        let mut server = ServerConnection::<ServerMessage, ClientMessage>::new("test", None)
            .expect("Could not create ServerConnection");
        let server_info = ServerInfo::new(None, "test");
        let client = ClientConnection::<ServerMessage, ClientMessage>::new(server_info)
            .expect("Could not create ClientConnection");

        // Open the connection by sending the first message from the client
        client
            .send(ClientMessage::Hello)
            .expect("Could not send initial 'Hello' message");

        // Receive and check it on the server
        let client_message = server
            .receive()
            .expect("Could not receive message at server");
        println!("Client Message = {}", client_message);
        assert_eq!(client_message, ClientMessage::Hello);

        // Respond from the server
        server
            .send(ServerMessage::World)
            .expect("Could not send server message");

        // Receive it and check it on the client
        let server_message = client
            .receive()
            .expect("Could not receive message at client");
        println!("Server Message = {}", server_message);
        assert_eq!(server_message, ServerMessage::World);
    }

    #[test]
    #[serial(client_server)]
    fn receive_no_wait() {
        let mut server = ServerConnection::<ServerMessage, ClientMessage>::new("test", None)
            .expect("Could not create ServerConnection");
        let server_info = ServerInfo::new(None, "test");
        let client = ClientConnection::<ServerMessage, ClientMessage>::new(server_info)
            .expect("Could not create ClientConnection");

        // Open the connection by sending the first message from the client
        client
            .send(ClientMessage::Hello)
            .expect("Could not send initial 'Hello' message");

        std::thread::sleep(Duration::from_millis(10));

        // Receive and check it on the server
        assert_eq!(
            server
                .receive_no_wait()
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
                .receive()
                .expect("Could not receive message at client"),
            ServerMessage::World
        );
    }
}
