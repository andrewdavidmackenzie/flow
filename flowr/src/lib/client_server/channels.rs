/// This is the channel-based implementation of the lib.client_server communications
use std::fmt::Debug;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

#[cfg(feature = "debugger")]
use crate::debug_messages::DebugClientMessage;
#[cfg(feature = "debugger")]
use crate::debug_messages::DebugServerMessage;
use crate::errors::*;
use crate::runtime_messages::{ClientMessage, ServerMessage};

/// `RuntimeClientConnection` stores information related to the connection from a runtime client
/// to the runtime server and is used each time a message is to be sent or received.
pub struct RuntimeClientConnection {
    channels: (Arc<Mutex<Receiver<ServerMessage>>>, Sender<ClientMessage>),
}

impl RuntimeClientConnection {
    /// Create a new connection between client and server
    pub fn new(runtime_server_context: &ServerConnection) -> Self {
        RuntimeClientConnection {
            channels: runtime_server_context.get_client_channels(),
        }
    }

    /// Start the client side of the client/server connection by connecting to TCP Socket
    /// server is listening on.
    pub fn start(&mut self) -> Result<()> {
        Ok(())
    }

    /// Receive a Message from the runtime Server
    pub fn client_recv(&self) -> Result<ServerMessage> {
        let guard = self
            .channels
            .0
            .lock()
            .map_err(|_| "Could not lock client Event reception channel")?;
        guard
            .recv()
            .chain_err(|| "Error receiving Event from client channel")
    }

    /// Send a Message from the runtime client to the runtime server
    pub fn client_send(&self, message: ClientMessage) -> Result<()> {
        self.channels
            .1
            .send(message)
            .chain_err(|| "Error sending on client channel")
    }
}

/// `DebugServerConnection` store information about the server side of the client/server
/// communications between a debug client and a debug server and is used each time a message
/// needs to be sent or received.
#[cfg(feature = "debugger")]
pub struct DebugClientConnection {
    channels: (
        Arc<Mutex<Receiver<DebugServerMessage>>>,
        Sender<DebugClientMessage>,
    ),
}

/// `DebugClientConnection` stores information related to the connection from a debug client
/// to the debug server and is used each time a message is to be sent or received.
#[cfg(feature = "debugger")]
impl DebugClientConnection {
    /// Create a new Server side of theRuntime client/server Connection
    pub fn new(debug_server_context: &DebugServerConnection) -> Self {
        DebugClientConnection {
            channels: debug_server_context.get_channels(),
        }
    }

    /// Start the connection to the debug server, making it ready to be used for sending and
    /// receiving messages between debug_client and debug_server
    pub fn start(&mut self) -> Result<()> {
        Ok(())
    }

    /// Receive a Message from the debug Server
    pub fn client_recv(&self) -> Result<DebugServerMessage> {
        let guard = self
            .channels
            .0
            .lock()
            .map_err(|_| "Could not lock debug Event reception channel")?;
        guard
            .recv()
            .chain_err(|| "Error receiving Event from debug channel")
    }

    /// Send a Message to the debugger
    pub fn client_send(&self, response: DebugClientMessage) -> Result<()> {
        self.channels
            .1
            .send(response)
            .chain_err(|| "Error sending on Debug channel")
    }
}

/// `ServerConnection` store information about the server side of the client/server
/// communications between a runtime client and a runtime server and is used each time a message
/// needs to be sent or received.
#[derive(Debug)]
pub struct ServerConnection {
    /// A channel to sent events to a client on
    client_event_channel_tx: Sender<ServerMessage>,
    /// The other end of the channel a client can receive events of
    client_event_channel_rx: Arc<Mutex<Receiver<ServerMessage>>>,
    /// A channel to for a client to send responses on
    client_response_channel_tx: Sender<ClientMessage>,
    /// This end of the channel where coordinator will receive events from a client on
    client_response_channel_rx: Receiver<ClientMessage>,
}

impl ServerConnection {
    /// Create a new Server side of theRuntime client/server Connection
    pub fn new(_server_hostname: Option<&str>) -> Self {
        let (client_event_channel_tx, client_event_channel_rx) = mpsc::channel();
        let (client_response_channel_tx, client_response_channel_rx) = mpsc::channel();

        ServerConnection {
            client_event_channel_tx,
            client_event_channel_rx: Arc::new(Mutex::new(client_event_channel_rx)),
            client_response_channel_tx,
            client_response_channel_rx,
        }
    }

    /// Start the Server side of client/server connection, by creating a Socket and Binding to it
    pub fn start(&self) -> Result<()> {
        Ok(())
    }

    /// Get the channels a client should use to send to the server
    fn get_client_channels(&self) -> (Arc<Mutex<Receiver<ServerMessage>>>, Sender<ClientMessage>) {
        // Clone of Arc and Sender is OK
        (
            self.client_event_channel_rx.clone(),
            self.client_response_channel_tx.clone(),
        )
    }

    /// Get a Message sent to the client from the server
    pub fn get_message(&self) -> Result<ClientMessage> {
        self.client_response_channel_rx
            .recv()
            .chain_err(|| "Error receiving response from client")
    }

    /// Try to get a Message sent to the client to the server but without blocking
    pub fn get_message_no_wait(&self) -> Result<ClientMessage> {
        self.client_response_channel_rx
            .try_recv()
            .chain_err(|| "Error receiving response from client")
    }

    /// Send a server Message to the client and wait for it's response
    pub fn send_message(&mut self, message: ServerMessage) -> Result<ClientMessage> {
        self.client_event_channel_tx
            .send(message)
            .map_err(|e| format!("Error sending to client: '{}'", e))?;

        self.get_message()
    }

    /// Send a server Message to the client but don't wait for it's response
    pub fn send_message_only(&mut self, message: ServerMessage) -> Result<()> {
        self.client_event_channel_tx
            .send(message)
            .map_err(|e| format!("Error sending to client: '{}'", e))?;

        Ok(())
    }

    /// Close the Server side of the Runtime client/server Connection
    pub fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

/// `DebugServerConnection` store information about the server side of the client/server
/// communications between a debug client and a debug server and is used each time a message
/// needs to be sent or received.
#[cfg(feature = "debugger")]
#[derive(Debug)]
pub struct DebugServerConnection {
    /// A channel to send events to a debug client on
    debug_event_channel_tx: Sender<DebugServerMessage>,
    /// The other end of the channel a debug client can receive events on
    debug_event_channel_rx: Arc<Mutex<Receiver<DebugServerMessage>>>,
    /// A channel to for a debug client to send responses on
    debug_response_channel_tx: Sender<DebugClientMessage>,
    /// This end of the channel where coordinator will receive events from a debug client on
    debug_response_channel_rx: Receiver<DebugClientMessage>,
}

#[cfg(feature = "debugger")]
impl DebugServerConnection {
    /// Create a new `DebugServerConnection` at the Optionally specified hostname. If no server
    /// hostname is supplied `localhost` will be used
    pub fn new(_server_hostname: Option<&str>) -> Self {
        let (debug_event_channel_tx, debug_event_channel_rx) = mpsc::channel();
        let (debug_response_channel_tx, debug_response_channel_rx) = mpsc::channel();
        DebugServerConnection {
            debug_event_channel_tx,
            debug_event_channel_rx: Arc::new(Mutex::new(debug_event_channel_rx)),
            debug_response_channel_tx,
            debug_response_channel_rx,
        }
    }

    /// Start the `DebugServerConnection` making it ready to be connected to by debug clients
    pub fn start(&self) -> Result<()> {
        Ok(())
    }

    fn get_channels(
        &self,
    ) -> (
        Arc<Mutex<Receiver<DebugServerMessage>>>,
        Sender<DebugClientMessage>,
    ) {
        // Clone of Arc and Sender is OK
        (
            self.debug_event_channel_rx.clone(),
            self.debug_response_channel_tx.clone(),
        )
    }

    /// Get a message sent from the debug client to the debug server
    pub fn get_message(&self) -> Result<DebugClientMessage> {
        self.debug_response_channel_rx
            .recv()
            .chain_err(|| "Error receiving response from debug client")
    }

    /// Send a Message from the debug server to the debug client
    pub fn send_message(&self, message: DebugServerMessage) -> Result<()> {
        self.debug_event_channel_tx
            .send(message)
            .chain_err(|| "Could not send Debug event from Debug server")
    }
}
