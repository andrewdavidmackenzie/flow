use std::fmt::Display;
use std::time::Duration;

/// This is the message-queue implementation of the lib.client_server communications
use log::{info, trace};
use portpicker::pick_unused_port;
use simpdiscoverylib::{BeaconListener, BeaconSender};
use zmq::Socket;

use flowcore::errors::*;

/// WAIT for a message to arrive when performing a receive()
pub const WAIT:i32 = 0;
/// Do NOT WAIT for a message to arrive when performing a receive()
pub static DONT_WAIT:i32 = zmq::DONTWAIT;

const DISCOVERY_PORT:u16 = 9002;

/// Structure that holds information about the Server to help clients connect to it
#[derive(Clone, Debug)]
pub struct ServerInfo {
    /// Name of the service name to connect to on the server
    service_name: String,
    /// What communication method is used to communicate between client and server
    server_address: Option<(String, u16)>
}

impl ServerInfo {
    /// Create a new ServerInfo struct
    pub fn new(service_name: &str, address: Option<&str>, port: u16) -> Self {
        ServerInfo {
            service_name: service_name.into(),
            server_address: address
                    .map(|s| s.to_string())
                    .map(|name| (name, port),
            ) }
    }
}

/// `ClientConnection` stores information related to the connection from a runtime client
/// to the runtime server and is used each time a message is to be sent or received.
pub struct ClientConnection {
    requester: Socket,
}

impl ClientConnection {
    /// Create a new connection between client and server
    pub fn new(server_info: &mut ServerInfo) -> Result<Self> {
        let requester;

        let host_port = server_info.server_address.clone().unwrap_or(
            Self::discover_service(&server_info.service_name)?
        );

        info!(
            "Client will attempt to connect to service '{}' at: '{}:{}'",
            server_info.service_name, host_port.0, host_port.1
        );

        let context = zmq::Context::new();

        requester = context
            .socket(zmq::REQ)
            .chain_err(|| "Runtime client could not connect to service")?;

        requester
            .connect(&format!("tcp://{}:{}", host_port.0, host_port.1))
            .chain_err(|| "Could not connect to service")?;

        info!("Client connected to service '{}' on {}:{}",
                    server_info.service_name, host_port.0, host_port.1
                );
        server_info.server_address = Some(host_port);

        Ok(ClientConnection { requester })
    }

    // Try to discover a server offering a particular service by name
     fn discover_service(name: &str) -> Result<(String, u16)> {
        trace!("Creating beacon");
        let listener = BeaconListener::new(name.as_bytes(), DISCOVERY_PORT)?;
        info!("Client is waiting for a Service Discovery beacon for service with name '{}'", name);
        let beacon = listener.wait(Some(Duration::from_secs(10)))?;
        info!(
            "Service '{}' discovered at IP: {}, Port: {}",
            name, beacon.service_ip, beacon.service_port
        );
        Ok((beacon.service_ip, beacon.service_port))
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
    pub fn new(service_name: &'static str, host: Option<(String, u16)>) -> Result<Self> {
        let context = zmq::Context::new();
        let responder = context
            .socket(zmq::REP)
            .chain_err(|| "Server Connection - could not create Socket")?;

        let host_port = host.unwrap_or(("*".into(), pick_unused_port()
            .chain_err(|| "No ports free")?));

        responder.bind(&format!("tcp://{}:{}", host_port.0, host_port.1))
            .chain_err(|| "Server Connection - could not bind on TCO Socket")?;

        Self::enable_service_discovery(service_name, host_port.1)?;
        info!("Service '{}' listening on {}:{}", service_name, host_port.0, host_port.1);

        Ok(ServerConnection {
            responder
        })
    }

    /*
       Start a background thread that sends out beacons for service discovery by a client every second
    */
    fn enable_service_discovery(name: &str, port: u16) -> Result<()> {
        match BeaconSender::new(port, name.as_bytes(), DISCOVERY_PORT) {
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

    use crate::cli::client_server::{ClientConnection, DONT_WAIT, ServerConnection, ServerInfo, WAIT};

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
        let mut server = ServerConnection::new("test", None)
            .expect("Could not create ServerConnection");
        let mut server_info = ServerInfo::new("test", None, 0);
        let client = ClientConnection::new(&mut server_info)
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
        let mut server = ServerConnection::new("test", None)
            .expect("Could not create ServerConnection");
        let mut server_info = ServerInfo::new("test", None, 0);
        let client = ClientConnection::new(&mut server_info)
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
