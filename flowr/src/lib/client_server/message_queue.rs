use std::marker::PhantomData;

/// This is the message-queue implementation of the lib.client_server communications
use log::info;
use zmq::Socket;
use zmq::{Message, DONTWAIT};

use crate::errors::*;

/// `ClientConnection` stores information related to the connection from a runtime client
/// to the runtime server and is used each time a message is to be sent or received.
pub struct ClientConnection<'a, SM, CM> {
    host: String,
    port: usize,
    requester: Option<Socket>,
    phantom: PhantomData<&'a SM>,
    phantom2: PhantomData<&'a CM>,
}

// TODO change the type returned by start to be StartedConnection or similar to enforce protocol
// TODO and put send/receive/close methods on that - and avoid the Option on requester/responder

// TODO use combinators instead of if then else for returning errors.

impl<'a, SM, CM> ClientConnection<'a, SM, CM>
where
    SM: From<Message>,
    CM: Into<Message>,
{
    /// Create a new connection between client and server
    pub fn new(server_connection: &ServerConnection<SM, CM>) -> Self {
        ClientConnection {
            host: server_connection.host.clone(),
            port: server_connection.port,
            requester: None,
            phantom: PhantomData,
            phantom2: PhantomData,
        }
    }

    /// Start the client side of the client/server connection by connecting to TCP Socket
    /// server is listening on.
    pub fn start(&mut self) -> Result<()> {
        let context = zmq::Context::new();

        self.requester = Some(
            context
                .socket(zmq::REQ)
                .chain_err(|| "Runtime client could not connect to server")?,
        );

        if let Some(ref requester) = self.requester {
            requester
                .connect(&format!("tcp://{}:{}", self.host, self.port))
                .chain_err(|| "Could not connect to server")?;
        }

        info!("client connected to Server on {}:{}", self.host, self.port);

        Ok(())
    }

    /// Receive a ServerMessage from the server
    pub fn client_recv(&self) -> Result<SM> {
        if let Some(ref requester) = self.requester {
            let msg = requester
                .recv_msg(0)
                .map_err(|e| format!("Error receiving from Server: {}", e))?;
            Ok(SM::from(msg))
        } else {
            bail!("Client runtime connection has not been started")
        }
    }

    /// Send a ClientMessage to the  Server
    pub fn client_send(&self, message: CM) -> Result<()> {
        if let Some(ref requester) = self.requester {
            requester
                .send(message, 0)
                .chain_err(|| "Error sending to Runtime server")
        } else {
            bail!("Runtime client connection has not been started")
        }
    }
}

/// `ServerConnection` store information about the server side of the client/server
/// communications between a runtime client and a runtime server and is used each time a message
/// needs to be sent or received.
pub struct ServerConnection<SM, CM> {
    host: String,
    port: usize,
    responder: Option<zmq::Socket>,
    phantom: PhantomData<SM>,
    phantom2: PhantomData<CM>,
}

/// Implement a server connection for sending server messages of type <SM> and receiving
/// back client messages of type <CM>
impl<'a, SM, CM> ServerConnection<SM, CM>
where
    SM: Into<Message>,
    CM: From<Message>,
{
    /// Create a new Server side of the client/server Connection
    pub fn new(server_hostname: &Option<String>, port: usize) -> Self {
        ServerConnection {
            host: server_hostname
                .as_ref()
                .unwrap_or(&"localhost".to_string())
                .to_string(),
            port,
            responder: None,
            phantom: PhantomData,
            phantom2: PhantomData,
        }
    }

    /// Start the Server side of client/server connection, by creating a Socket and Binding to it
    pub fn start(&mut self) -> Result<()> {
        let context = zmq::Context::new();

        self.responder = Some(
            context
                .socket(zmq::REP)
                .chain_err(|| "Runtime Server Connection - could not create Socket")?,
        );

        if let Some(ref responder) = self.responder {
            responder
                .bind(&format!("tcp://*:{}", self.port))
                .chain_err(|| "Server Connection - could not bind on Socket")?;
        }

        info!(
            "'flowr' server process listening on {}:{}",
            self.host, self.port
        );
        info!(
            "Use 'flowr -c {}:{} $flow_url' to send a job for execution",
            self.host, self.port
        );

        Ok(())
    }

    /// Get a Message sent from the client to the server
    pub fn get_message(&self) -> Result<CM> {
        let responder = self
            .responder
            .as_ref()
            .chain_err(|| "Runtime server connection not started")?;
        let msg = responder
            .recv_msg(0)
            .map_err(|e| format!("Runtime server error getting message: '{}'", e))?;
        Ok(CM::from(msg))
    }

    /// Try to get a Message sent from the client to the server but without blocking
    pub fn get_message_no_wait(&self) -> Result<CM> {
        let responder = self
            .responder
            .as_ref()
            .chain_err(|| "Runtime server connection not started")?;
        let msg = responder
            .recv_msg(DONTWAIT)
            .chain_err(|| "Runtime server could not receive message")?;

        Ok(CM::from(msg))
    }

    /// Send a Message from the server to the Client and wait for it's response
    pub fn send_message(&mut self, message: SM) -> Result<CM> {
        let responder = self
            .responder
            .as_ref()
            .chain_err(|| "Runtime server connection not started")?;

        responder
            .send(message, 0)
            .map_err(|e| format!("Runtime server error sending to client: '{}'", e))?;

        self.get_message()
    }

    /// Send a Message from the server to the Client but don't wait for it's response
    pub fn send_message_only(&mut self, message: SM) -> Result<()> {
        let responder = self
            .responder
            .as_ref()
            .chain_err(|| "Runtime server connection not started")?;

        responder
            .send(message, 0)
            .map_err(|e| format!("Runtime server error sending to client: '{}'", e))?;

        Ok(())
    }

    /// Close the Server side of the Runtime client/server Connection
    pub fn close(&mut self) -> Result<()> {
        let responder = self
            .responder
            .as_ref()
            .chain_err(|| "Runtime server connection not started")?;

        responder
            .disconnect("")
            .chain_err(|| "Error trying to disconnect responder")
    }
}
