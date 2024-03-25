use std::fmt::Display;
use std::time::Duration;

use flowcore::errors::{Result, ResultExt, bail};

/// This is the message-queue implementation of the Client<-->[Coordinator][flowrlib::coordinator::Coordinator]
/// communications
use log::{debug, info, trace};
use simpdiscoverylib::BeaconSender;
use zmq::Socket;

/// WAIT for a message to arrive when performing a `receive()`
pub const WAIT:i32 = 0;

/// Do NOT WAIT for a message to arrive when performing a `receive()`
pub static DONT_WAIT:i32 = zmq::DONTWAIT;

/// Use this to discover the coordinator service by name
pub const COORDINATOR_SERVICE_NAME: &str = "runtime._flowr._tcp.local";

/// Use this to discover the debug service by name
pub const DEBUG_SERVICE_NAME: &str = "debug._flowr._tcp.local";

/// Start a background thread that sends out beacons for service discovery by a client every second
pub fn enable_service_discovery(discovery_port: u16, name: &str, service_port: u16) -> Result<()> {
    match BeaconSender::new(service_port, name.as_bytes(), discovery_port) {
        Ok(beacon) => {
            info!(
                    "Discovery beacon announcing service named '{}', on port: {}",
                    name, service_port
                );
            std::thread::spawn(move || {
                let _ = beacon.send_loop(Duration::from_secs(1));
            });
        }
        Err(e) => bail!("Error starting discovery beacon: {}", e.to_string()),
    }

    Ok(())
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

        info!("Service '{}' listening on *:{}", service_name, port);

        Ok(CoordinatorConnection {
            responder
        })
    }

    /// Receive a Message sent from the client to the [Coordinator][flowrlib::coordinator::Coordinator]
    pub fn receive<CM>(&self, flags: i32) -> Result<CM>
    where
        CM: From<String> + Display {
        trace!("Coordinator waiting for message from client");

        let msg = self
            .responder
            .recv_msg(flags)
            .map_err(|e| format!("Coordinator error getting message: '{e}'"))?;

        let message_string = msg.as_str().ok_or("Could not get message as str")?
            .to_string();
        let message = message_string.into();
        trace!("                ---> Coordinator Received {}", message);
        Ok(message)
    }

    /// Send a Message from the [Coordinator][flowrlib::coordinator::Coordinator]
    /// to the Client and wait for it's response
    pub fn send_and_receive_response<SM, CM>(&mut self, message: SM) -> Result<CM>
    where
        SM: Into<String> + Display,
        CM: From<String> + Display {
        self.send(message)?;
        self.receive(WAIT)
    }

    /// Send a Message from the [Coordinator][flowrlib::coordinator::Coordinator]
    /// to the Client but don't wait for it's response
    pub fn send<SM>(&mut self, message: SM) -> Result<()>
    where
        SM: Into<String> + Display {
        trace!("                <--- Coordinator Sending {}", message);
        self.responder
            .send(&message.into(), 0)
            .chain_err(|| "Coordinator error sending to client".to_string())
    }
}
