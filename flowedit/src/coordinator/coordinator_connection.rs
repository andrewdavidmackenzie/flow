use flowcore::errors::{Result, ResultExt};
use std::fmt::Display;

pub use flowrlib::discovery::enable_service_discovery;
pub use flowrlib::services::COORDINATOR_SERVICE_NAME;
pub use flowrlib::services::DEBUG_SERVICE_NAME;
use log::{debug, info, trace};
use zmq::Socket;

/// WAIT for a message
pub const WAIT: i32 = 0;

/// Do NOT WAIT for a message
pub static DONT_WAIT: i32 = zmq::DONTWAIT;

/// Coordinator side of the client/coordinator connection (ZMQ REP socket)
pub struct CoordinatorConnection {
    responder: Socket,
}

impl CoordinatorConnection {
    /// Create a new coordinator connection bound to the given port
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

    /// Send a message and wait for response
    pub fn send_and_receive_response<SM, CM>(&mut self, message: SM) -> Result<CM>
    where
        SM: Into<String> + Display,
        CM: From<String> + Display,
    {
        self.send(message)?;
        self.receive(WAIT)
    }

    /// Send a message without waiting for response
    pub fn send<SM>(&mut self, message: SM) -> Result<()>
    where
        SM: Into<String> + Display,
    {
        trace!("                <--- Coordinator Sending {message}");
        self.responder
            .send(&message.into(), 0)
            .chain_err(|| "Coordinator error sending to client".to_string())
    }
}
