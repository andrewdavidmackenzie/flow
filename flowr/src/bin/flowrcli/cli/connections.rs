use std::fmt::Display;
use std::time::Duration;

/// This is the message-queue implementation of the Client<-->[Coordinator][flowrlib::coordinator::Coordinator]
/// communications
use log::{debug, info, trace};
use simpdiscoverylib::{BeaconListener, BeaconSender};
use zmq::Socket;

use flowcore::errors::{Result, ResultExt, bail};

/// WAIT for a message to arrive when performing a `receive()`
pub const WAIT: i32 = 0;

/// Do NOT WAIT for a message to arrive when performing a `receive()`
pub static DONT_WAIT: i32 = zmq::DONTWAIT;

/// Use this to discover the coordinator service by name
pub const COORDINATOR_SERVICE_NAME: &str = "runtime._flowr._tcp.local";

/// Use this to discover the debug service by name
#[cfg(feature = "debugger")]
pub const DEBUG_SERVICE_NAME: &str = "debug._flowr._tcp.local";

/// Try to discover a particular service by name
pub fn discover_service(discovery_port: u16, name: &str) -> Result<String> {
    let listener = BeaconListener::new(name.as_bytes(), discovery_port)?;
    let beacon = listener.wait(None)?;
    let address = format!("{}:{}", beacon.service_ip, beacon.service_port);
    Ok(address)
}

/// Start a background thread that sends out beacons for service discovery by a client every second
pub fn enable_service_discovery(discovery_port: u16, name: &str, service_port: u16) -> Result<()> {
    match BeaconSender::new(service_port, name.as_bytes(), discovery_port) {
        Ok(beacon) => {
            info!(
                    "Discovery beacon announcing service named '{name}', on port: {service_port}");
            std::thread::spawn(move || {
                let _ = beacon.send_loop(Duration::from_secs(1));
            });
        }
        Err(e) => bail!("Error starting discovery beacon: {}", e.to_string()),
    }

    Ok(())
}

/// `ClientConnection` stores information related to the connection from a client
/// to the [Coordinator][flowrlib::coordinator::Coordinator] and is used each time a message is to
/// be sent or received.
pub struct ClientConnection {
    requester: Socket,
}

impl ClientConnection {
    /// Create a new connection between client and [Coordinator][flowrlib::coordinator::Coordinator]
    pub fn new(coordinator_address: &str) -> Result<Self> {
        info!("Client will attempt to connect to coordinator at: '{coordinator_address}'");

        let context = zmq::Context::new();

        let requester = context
            .socket(zmq::REQ)
            .chain_err(|| "Client could not connect to coordinator service")?;

        requester
            .connect(&format!("tcp://{coordinator_address}"))
            .chain_err(|| format!("Client Connection - Could not connect to socket at: {coordinator_address}"))?;

        info!("Client connected to coordinator at '{coordinator_address}'");

        Ok(ClientConnection { requester })
    }

    /// Receive a [`CoordinatorMessage`][crate::cli::coordinator_message::CoordinatorMessage] from the
    /// [Coordinator][flowrlib::coordinator::Coordinator]
    pub fn receive<CM>(&self) -> Result<CM>
    where
        CM: From<String> + Display,
    {
        trace!("Client waiting for message from coordinator");

        let msg = self
            .requester
            .recv_msg(0)
            .map_err(|e| format!("Error receiving from coordinator: {e}"))?;

        let message_string = msg.as_str().ok_or("Could not get message as str")?
            .to_string();
        let message: CM = message_string.into();
        trace!("Client Received <--- {message}");
        Ok(message)
    }

    /// Send a [`CoordinatorMessage`][crate::cli::coordinator_message::CoordinatorMessage] to the
    /// [Coordinator][flowrlib::coordinator::Coordinator]
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
    /// Create a new [Coordinator][flowrlib::coordinator::Coordinator]
    /// side of the client/coordinator Connection
    pub fn new(service_name: &'static str, port: u16) -> Result<Self> {
        let context = zmq::Context::new();
        let responder = context
            .socket(zmq::REP)
            .chain_err(|| "Coordinator Connection - could not create Socket")?;

        debug!("Coordinator Connection attempting to bind to: tcp://*:{port}");
        responder.bind(&format!("tcp://*:{port}"))
            .chain_err(||
                format!("Coordinator Connection - could not bind on TCP Socket on: tcp://{port}"))?;

        info!("Service '{service_name}' listening on *:{port}");

        Ok(CoordinatorConnection {
            responder
        })
    }

    /// Receive a Message sent from the client to the [Coordinator][flowrlib::coordinator::Coordinator]
    pub fn receive<CM>(&self, flags: i32) -> Result<CM>
    where
        CM: From<String> + Display,
    {
        trace!("Coordinator waiting for message from client");

        let msg = self
            .responder
            .recv_msg(flags)
            .map_err(|e| format!("Coordinator error getting message: '{e}'"))?;

        let message_string = msg.as_str().ok_or("Could not get message as str")?
            .to_string();
        let message = message_string.into();
        trace!("                ---> Coordinator Received {message}");
        Ok(message)
    }

    /// Send a Message from the [Coordinator][flowrlib::coordinator::Coordinator]
    /// to the Client and wait for it's response
    pub fn send_and_receive_response<SM, CM>(&mut self, message: SM) -> Result<CM>
    where
        SM: Into<String> + Display,
        CM: From<String> + Display,
    {
        self.send(message)?;
        self.receive(WAIT)
    }

    /// Send a Message from the [Coordinator][flowrlib::coordinator::Coordinator]
    /// to the Client but don't wait for it's response
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
mod test {
    use std::fmt;
    use std::time::Duration;

    use portpicker::pick_unused_port;
    use serde_derive::{Deserialize, Serialize};
    use serial_test::serial;

    use crate::cli::connections::{ClientConnection, CoordinatorConnection, discover_service, DONT_WAIT, enable_service_discovery, WAIT};

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
            write!(f, "ClientMessage Hello", )
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

    // Requires network access
    #[test]
    #[serial]
    fn coordinator_receive_wait_get_reply() {
        let test_port = pick_unused_port().expect("No ports free");
        let mut coordinator_connection = CoordinatorConnection::new("test", test_port)
            .expect("Could not create CoordinatorConnection");

        let discovery_port = pick_unused_port().expect("No ports free");
        enable_service_discovery(discovery_port, "test", test_port)
            .expect("Could not enable service discovery");

        let coordinator_address = discover_service(discovery_port, "test")
            .expect("Could not discover service");
        let client = ClientConnection::new(&coordinator_address)
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

    // Requires network access
    #[test]
    #[serial]
    fn coordinator_receive_nowait_get_reply() {
        let test_port = pick_unused_port().expect("No ports free");
        let mut coordinator_connection = CoordinatorConnection::new("test", test_port)
            .expect("Could not create CoordinatorConnection");
        let discovery_port = pick_unused_port().expect("No ports free");
        enable_service_discovery(discovery_port, "test", test_port)
            .expect("Could not enable service discovery");

        let coordinator_address = discover_service(discovery_port, "test")
            .expect("Could discovery service");
        let client = ClientConnection::new(&coordinator_address)
            .expect("Could not create ClientConnection");

        // Open the connection by sending the first message from the client
        client
            .send(ClientMessage::Hello)
            .expect("Could not send initial 'Hello' message");

        std::thread::sleep(Duration::from_millis(100));

        assert_eq!(
            coordinator_connection
                .receive::<ClientMessage>(DONT_WAIT)
                .expect("Could not receive message at Coordinator"),
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
