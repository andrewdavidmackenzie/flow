use std::fmt::Display;
use std::marker::PhantomData;
use std::time::Duration;

/// This is the message-queue implementation of the lib.client_server communications
use log::{info, trace};
use simpdiscoverylib::{BeaconListener, BeaconSender};
use zmq::Socket;
use zmq::{Message, DONTWAIT};

use crate::errors::*;

const BEACON_PORT: u16 = 9001;
const FLOW_SERVICE_NAME: &str = "_flowr._tcp.local";

/// `ClientConnection` stores information related to the connection from a runtime client
/// to the runtime server and is used each time a message is to be sent or received.
pub struct ClientConnection<'a, SM, CM> {
    port: usize,
    requester: Socket,
    phantom: PhantomData<&'a SM>,
    phantom2: PhantomData<&'a CM>,
}

impl<'a, SM, CM> ClientConnection<'a, SM, CM>
where
    SM: From<Message> + Display,
    CM: Into<Message> + Display,
{
    /// Create a new connection between client and server
    pub fn new(name: &str, server_hostname: Option<String>, port: usize) -> Result<Self> {
        let hostname = server_hostname
            .or_else(|| Self::discover_service(name))
            .unwrap_or_else(|| "localhost".into());

        info!(
            "Client will attempt to connect to server at: '{}'",
            hostname
        );

        let context = zmq::Context::new();

        let requester = context
            .socket(zmq::REQ)
            .chain_err(|| "Runtime client could not connect to server")?;

        requester
            .connect(&format!("tcp://{}:{}", hostname, port))
            .chain_err(|| "Could not connect to server")?;

        info!("client connected to Server on {}:{}", hostname, port);

        Ok(ClientConnection {
            port,
            requester,
            phantom: PhantomData,
            phantom2: PhantomData,
        })
    }

    /*
        try to discover a server that a client can send a submission to
    */
    #[cfg(feature = "distributed")]
    fn discover_service(name: &str) -> Option<String> {
        let listener =
            BeaconListener::new(BEACON_PORT, Some(format!("{}.{}", name, FLOW_SERVICE_NAME)))
                .ok()?;
        let beacon = listener.wait(None).ok()?;
        info!("'flowr' server discovered at IP: {}", beacon.source_ip);
        Some(beacon.source_ip)
    }

    /// Receive a ServerMessage from the server
    pub fn receive(&self) -> Result<SM> {
        trace!("Client waiting for message from server");

        let msg = self
            .requester
            .recv_msg(0)
            .map_err(|e| format!("Error receiving from Server: {}", e))?;

        let message = SM::from(msg);
        trace!("Client Received <--- {}", message);
        Ok(message)
    }

    /// Send a ClientMessage to the Server
    pub fn send(&self, message: CM) -> Result<()> {
        trace!("Client Sent     ---> to {} {}", self.port, message);
        self.requester
            .send(message, 0)
            .chain_err(|| "Error sending to Runtime server")
    }
}

/// `ServerConnection` store information about the server side of the client/server
/// communications between a runtime client and a runtime server and is used each time a message
/// needs to be sent or received.
pub struct ServerConnection<SM, CM> {
    port: usize,
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
{
    /// Create a new Server side of the client/server Connection
    pub fn new(name: &str, port: usize) -> Result<Self> {
        let context = zmq::Context::new();
        let responder = context
            .socket(zmq::REP)
            .chain_err(|| "Server Connection - could not create Socket")?;

        responder
            .bind(&format!("tcp://*:{}", port))
            .chain_err(|| "Server Connection - could not bind on Socket")?;

        Self::enable_service_discovery(name)?;

        info!("'flowr' server process listening on port {}", port);

        Ok(ServerConnection {
            port,
            responder,
            phantom: PhantomData,
            phantom2: PhantomData,
        })
    }

    /*
       Start a background thread that sends out beacons for service discovery by a client every second
    */
    fn enable_service_discovery(name: &str) -> Result<()> {
        match BeaconSender::new(BEACON_PORT, &format!("{}.{}", name, FLOW_SERVICE_NAME)) {
            Ok(beacon) => {
                info!(
                    "Discovery beacon announcing service named '{}', on port: {}",
                    FLOW_SERVICE_NAME, BEACON_PORT
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

    /// Close the Server side of the Runtime client/server Connection
    pub fn close(&mut self) -> Result<()> {
        self.responder
            .disconnect("")
            .chain_err(|| "Server error trying to disconnect responder")
    }
}
