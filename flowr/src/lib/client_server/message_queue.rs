/// This is the message-queue implementation of the lib.client_server communications
use log::{debug, info};
use zmq::Socket;
use zmq::{Message, DONTWAIT};

#[cfg(feature = "debugger")]
use crate::debug_messages::DebugClientMessage;
#[cfg(feature = "debugger")]
use crate::debug_messages::DebugServerMessage;
use crate::errors::*;
use crate::runtime_messages::{ClientMessage, ServerMessage};

impl From<ServerMessage> for Message {
    fn from(event: ServerMessage) -> Self {
        match serde_json::to_string(&event) {
            Ok(message_string) => Message::from(&message_string),
            _ => Message::new(),
        }
    }
}

impl From<Message> for ServerMessage {
    fn from(msg: Message) -> Self {
        match msg.as_str() {
            Some(message_string) => match serde_json::from_str(message_string) {
                Ok(message) => message,
                _ => ServerMessage::Invalid,
            },
            _ => ServerMessage::Invalid,
        }
    }
}

impl From<ClientMessage> for Message {
    fn from(msg: ClientMessage) -> Self {
        match serde_json::to_string(&msg) {
            Ok(message_string) => Message::from(&message_string),
            _ => Message::new(),
        }
    }
}

impl From<Message> for ClientMessage {
    fn from(msg: Message) -> Self {
        match msg.as_str() {
            Some(message_string) => match serde_json::from_str(message_string) {
                Ok(message) => message,
                _ => ClientMessage::Invalid,
            },
            _ => ClientMessage::Invalid,
        }
    }
}

#[cfg(feature = "debugger")]
impl From<DebugServerMessage> for Message {
    fn from(debug_event: DebugServerMessage) -> Self {
        match serde_json::to_string(&debug_event) {
            Ok(message_string) => Message::from(&message_string),
            _ => Message::new(),
        }
    }
}

#[cfg(feature = "debugger")]
impl From<Message> for DebugServerMessage {
    fn from(msg: Message) -> Self {
        match msg.as_str() {
            Some(message_string) => match serde_json::from_str(message_string) {
                Ok(message) => message,
                _ => DebugServerMessage::Invalid,
            },
            _ => DebugServerMessage::Invalid,
        }
    }
}

#[cfg(feature = "debugger")]
impl From<DebugClientMessage> for Message {
    fn from(msg: DebugClientMessage) -> Self {
        match serde_json::to_string(&msg) {
            Ok(message_string) => Message::from(&message_string),
            _ => Message::new(),
        }
    }
}

#[cfg(feature = "debugger")]
impl From<Message> for DebugClientMessage {
    fn from(msg: Message) -> Self {
        match msg.as_str() {
            Some(message_string) => match serde_json::from_str(message_string) {
                Ok(message) => message,
                _ => DebugClientMessage::Invalid,
            },
            _ => DebugClientMessage::Invalid,
        }
    }
}

/// `RuntimeClientConnection` stores information related to the connection from a runtime client
/// to the runtime server and is used each time a message is to be sent or received.
pub struct RuntimeClientConnection {
    context: zmq::Context,
    host: String,
    port: usize,
    requester: Option<Socket>,
}

impl RuntimeClientConnection {
    /// Create a new connection between client and server
    pub fn new(runtime_server_connection: &RuntimeServerConnection) -> Self {
        RuntimeClientConnection {
            context: zmq::Context::new(),
            host: runtime_server_connection.host.clone(),
            port: runtime_server_connection.port,
            requester: None,
        }
    }

    /// Start the client side of the client/server connection by connecting to TCP Socket
    /// server is listening on.
    pub fn start(&mut self) -> Result<()> {
        self.requester = Some(
            self.context
                .socket(zmq::REQ)
                .chain_err(|| "Runtime client could not create client ZMQ socket")?,
        );

        if let Some(ref requester) = self.requester {
            requester
                .connect(&format!("tcp://{}:{}", self.host, self.port))
                .chain_err(|| "Runtime client could not connect to server")?;
        }

        info!(
            "Runtime client connected to Server on {}:{}",
            self.host, self.port
        );

        Ok(())
    }

    /// Receive a Message from the runtime server
    pub fn client_recv(&self) -> Result<ServerMessage> {
        if let Some(ref requester) = self.requester {
            let msg = requester
                .recv_msg(0)
                .map_err(|e| format!("Error receiving from Server: {}", e))?;
            Ok(ServerMessage::from(msg))
        } else {
            bail!("Runtime Client connection has not been started")
        }
    }

    /// Send a Message to the Runtime Server
    pub fn client_send(&self, message: ClientMessage) -> Result<()> {
        if let Some(ref requester) = self.requester {
            requester
                .send(message, 0)
                .chain_err(|| "Error sending to Runtime server")
        } else {
            bail!("Runtime client connection has not been started")
        }
    }
}

/// `DebugClientConnection` stores information related to the connection from a debug client
/// to the debug server and is used each time a message is to be sent or received.
#[cfg(feature = "debugger")]
pub struct DebugClientConnection {
    host: String,
    port: usize,
    requester: Option<Socket>,
}

#[cfg(feature = "debugger")]
impl DebugClientConnection {
    /// Create a new connection to the debug server represented in the `DebugServerConnection`
    pub fn new(debug_server_context: &DebugServerConnection) -> Self {
        DebugClientConnection {
            host: debug_server_context.host.clone(),
            port: debug_server_context.port,
            requester: None,
        }
    }

    /// Start the connection to the debug server, making it ready to be used for sending and
    /// receiving messages between debug_client and debug_server
    pub fn start(&mut self) -> Result<()> {
        let context = zmq::Context::new();

        self.requester = Some(
            context
                .socket(zmq::REQ)
                .chain_err(|| "Debug client: Socket could not be created")?,
        );

        if let Some(ref requester) = self.requester {
            requester
                .connect(&format!("tcp://{}:{}", self.host, self.port))
                .chain_err(|| "Debug client: Could not connect to debug server")?;
        }

        debug!(
            "Debug client: Connected to debug server on {}:{}",
            self.host, self.port
        );

        // Send an first message to initialize the connection
        self.client_send(DebugClientMessage::Ack)
    }

    /// Receive a Message from the debug server
    pub fn client_recv(&self) -> Result<DebugServerMessage> {
        if let Some(ref requester) = self.requester {
            let msg = requester
                .recv_msg(0)
                .map_err(|e| format!("Debug client: Error receiving from Debug server: {}", e))?;
            Ok(DebugServerMessage::from(msg))
        } else {
            bail!("Debug Client: Connection has not been started")
        }
    }

    /// Send a Message to the debug server
    pub fn client_send(&self, message: DebugClientMessage) -> Result<()> {
        if let Some(ref requester) = self.requester {
            requester
                .send(message, 0)
                .chain_err(|| "Debug client: Error sending to debug server")
        } else {
            bail!("Debug client: Connection has not been started")
        }
    }
}

/// `RuntimeServerConnection` store information about the server side of the client/server
/// communications between a runtime client and a runtime server and is used each time a message
/// needs to be sent or received.
pub struct RuntimeServerConnection {
    host: String,
    port: usize,
    responder: Option<zmq::Socket>,
}

impl RuntimeServerConnection {
    /// Create a new Server side of theRuntime client/server Connection
    pub fn new(server_hostname: Option<&str>) -> Self {
        RuntimeServerConnection {
            host: server_hostname.unwrap_or("localhost").into(),
            port: 5555,
            responder: None,
        }
    }

    /// Start the Server side of client/server connection, by creating a Socket and Binding to it
    pub fn start(&mut self) -> Result<()> {
        let context = zmq::Context::new();
        self.responder = Some(
            context
                .socket(zmq::REP)
                .chain_err(|| "Runtime Server Connection: Could not create Socket")?,
        );

        if let Some(ref responder) = self.responder {
            responder
                .bind(&format!("tcp://*:{}", self.port))
                .chain_err(|| "Runtime Server Connection: Could not bind on Socket")?;
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
    pub fn get_message(&self) -> Result<ClientMessage> {
        let responder = self
            .responder
            .as_ref()
            .chain_err(|| "Runtime Server Connection: Connection not started")?;
        let msg = responder
            .recv_msg(0)
            .map_err(|e| format!("Runtime Server Connection: Error getting message: '{}'", e))?;
        Ok(ClientMessage::from(msg))
    }

    /// Try to get a Message sent from the client to the server but without blocking
    pub fn get_message_no_wait(&self) -> Result<ClientMessage> {
        let responder = self
            .responder
            .as_ref()
            .chain_err(|| "Runtime Server Connection: Connection not started")?;
        let msg = responder
            .recv_msg(DONTWAIT)
            .chain_err(|| "Runtime Server Connection: Could not receive message")?;

        Ok(ClientMessage::from(msg))
    }

    /// Send a Message from the server to the Client and wait for it's response
    pub fn send_message(&mut self, message: ServerMessage) -> Result<ClientMessage> {
        let responder = self
            .responder
            .as_ref()
            .chain_err(|| "Runtime Server Connection: Connection not started")?;

        responder.send(message, 0).map_err(|e| {
            format!(
                "Runtime Server Connection: Error sending to client: '{}'",
                e
            )
        })?;

        self.get_message()
    }

    /// Send a Message from the server to the Client but don't wait for it's response
    pub fn send_message_only(&mut self, event: ServerMessage) -> Result<()> {
        let responder = self
            .responder
            .as_ref()
            .chain_err(|| "Runtime Server Connection: Connection not started")?;

        responder.send(event, 0).map_err(|e| {
            format!(
                "Runtime Server Connection: Error sending to client: '{}'",
                e
            )
        })?;

        Ok(())
    }

    /// Close the Server side of the Runtime client/server Connection
    pub fn close(&mut self) -> Result<()> {
        let responder = self
            .responder
            .as_ref()
            .chain_err(|| "Runtime Server Connection: Connection not started")?;

        responder
            .disconnect("")
            .chain_err(|| "Runtime Server Connection: Error trying to disconnect responder")
    }
}

/// `DebugServerConnection` store information about the server side of the client/server
/// communications between a debug client and a debug server and is used each time a message
/// needs to be sent or received.
#[cfg(feature = "debugger")]
pub struct DebugServerConnection {
    host: String,
    port: usize,
    responder: Option<zmq::Socket>,
}

#[cfg(feature = "debugger")]
impl DebugServerConnection {
    /// Create a new `DebugServerConnection` at the Optionally specified hostname. If no server
    /// hostname is supplied `localhost` will be used
    pub fn new(server_hostname: Option<&str>) -> Self {
        DebugServerConnection {
            host: server_hostname.unwrap_or("localhost").into(),
            port: 5556,
            responder: None,
        }
    }

    /// Start the `DebugServerConnection` making it ready to be connected to by debug clients
    pub fn start(&mut self) -> Result<()> {
        let context = zmq::Context::new();
        self.responder = Some(
            context
                .socket(zmq::REP)
                .chain_err(|| "Debug Server not start connection")?,
        );

        if let Some(ref responder) = self.responder {
            responder
                .bind(&format!("tcp://*:{}", self.port))
                .chain_err(|| "Debug Server could not bind connection")?;
        }

        info!(
            "'flowr' debug server listening on {}:{}",
            self.host, self.port
        );

        Ok(())
    }

    /// Get a message sent from the debug client to the debug server
    pub fn get_message(&self) -> Result<DebugClientMessage> {
        let responder = self
            .responder
            .as_ref()
            .chain_err(|| "Runtime server connection not started")?;
        let msg = responder
            .recv_msg(0)
            .chain_err(|| "Runtime server could not receive response")?;

        Ok(DebugClientMessage::from(msg))
    }

    /// Send a Message from the debug server to the debug client
    pub fn send_message(&self, message: DebugServerMessage) -> Result<()> {
        let responder = self
            .responder
            .as_ref()
            .chain_err(|| "Runtime server connection not started")?;
        responder
            .send(message, 0)
            .map_err(|e| format!("Error sending debug event to runtime client: {}", e))?;

        Ok(())
    }
}
