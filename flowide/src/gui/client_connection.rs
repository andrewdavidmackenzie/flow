use std::fmt::Display;

/// This is the message-queue implementation of the Client<-->[Coordinator][flowrlib::coordinator::Coordinator]
/// communications
use log::{info, trace};
use simpdiscoverylib::BeaconListener;
use zmq::Socket;

use flowcore::errors::*;

use crate::gui::coordinator_connection::WAIT;

/// Try to discover a particular service by name
pub fn discover_service(discovery_port: u16, name: &str) -> Result<String> {
    let listener = BeaconListener::new(name.as_bytes(), discovery_port)?;
    let beacon = listener.wait(None)?;
    let address = format!("{}:{}", beacon.service_ip, beacon.service_port);
    Ok(address)
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

    /// Receive a [CoordinatorMessage][crate::gui::coordinator_message::CoordinatorMessage] from the
    /// [Coordinator][flowrlib::coordinator::Coordinator]
    pub fn receive<CM>(&self) -> Result<CM>
    where
        CM: From<String> + Display {
        trace!("Client waiting for message from coordinator");

        let msg = self
            .requester
            .recv_msg(WAIT)
            .map_err(|e| format!("Error receiving from coordinator: {e}"))?;

        let message_string = msg.as_str()
            .ok_or("Could not get Message as String")?.to_owned();
        trace!("Client Received <--- {}", message_string);
        Ok(message_string.into())
    }

    /// Send a [ClientMessage][crate::gui::client_message] to the
    /// [Coordinator][flowrlib::coordinator::Coordinator]
    pub fn send<CM>(&self, message: CM) -> Result<()>
    where
        CM: Into<String> + Display {
        trace!("Client Sent     ---> {}", message);
        self.requester
            .send(&message.into(), WAIT)
            .chain_err(|| "Error sending to coordinator")
    }
}
