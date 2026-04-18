use std::fmt::Display;

use flowcore::errors::{Result, ResultExt};
pub use flowrlib::discovery::discover_service;
use log::{info, trace};
use zmq::Socket;

use crate::coordinator::coordinator_connection::WAIT;

/// Client side of the client/coordinator connection (ZMQ REQ socket)
pub struct ClientConnection {
    requester: Socket,
}

impl ClientConnection {
    /// Connect to the coordinator at the given address
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
        info!("Client connected to coordinator at '{coordinator_address}'");
        Ok(ClientConnection { requester })
    }

    /// Receive a message from the coordinator
    pub fn receive<CM>(&self) -> Result<CM>
    where
        CM: From<String> + Display,
    {
        trace!("Client waiting for message from coordinator");
        let msg = self
            .requester
            .recv_msg(WAIT)
            .map_err(|e| format!("Error receiving from coordinator: {e}"))?;
        let message_string = msg
            .as_str()
            .ok_or("Could not get Message as String")?
            .to_owned();
        trace!("Client Received <--- {message_string}");
        Ok(message_string.into())
    }

    /// Send a message to the coordinator
    pub fn send<CM>(&self, message: CM) -> Result<()>
    where
        CM: Into<String> + Display,
    {
        trace!("Client Sending     ---> {message}");
        self.requester
            .send(&message.into(), WAIT)
            .chain_err(|| "Error sending to coordinator")
    }
}
