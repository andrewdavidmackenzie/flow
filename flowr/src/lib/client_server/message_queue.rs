use log::error;
use zmq;
/// This is the message-queue implementation of the lib.client_server communications
use zmq::Message;
use zmq::Socket;

#[cfg(feature = "debugger")]
use crate::debug::Event as DebugEvent;
#[cfg(feature = "debugger")]
use crate::debug::Response as DebugResponse;
use crate::errors::*;
use crate::runtime::{Event, Response};

impl From<Event> for Message {
    fn from(event: Event) -> Self {
        match serde_json::to_string(&event) {
            Ok(message_string) => Message::from(&message_string),
            _ => Message::new()
        }
    }
}

impl From<&Message> for Event {
    fn from(msg: &Message) -> Self {
        match msg.as_str() {
            Some(message_string) => {
                match serde_json::from_str(message_string) {
                    Ok(message) => message,
                    _ => Event::Invalid
                }
            }
            _ => Event::Invalid
        }
    }
}

impl From<Response> for Message {
    fn from(msg: Response) -> Self {
        match serde_json::to_string(&msg) {
            Ok(message_string) => Message::from(&message_string),
            _ => Message::new()
        }
    }
}

impl From<&Message> for Response {
    fn from(msg: &Message) -> Self {
        match msg.as_str() {
            Some(message_string) => {
                match serde_json::from_str(message_string) {
                    Ok(message) => message,
                    _ => Response::Invalid
                }
            }
            _ => Response::Invalid
        }
    }
}

impl From<DebugEvent> for Message {
    fn from(debug_event: DebugEvent) -> Self {
        match serde_json::to_string(&debug_event) {
            Ok(message_string) => Message::from(&message_string),
            _ => Message::new()
        }
    }
}

impl From<&Message> for DebugEvent {
    fn from(msg: &Message) -> Self {
        match msg.as_str() {
            Some(message_string) => {
                match serde_json::from_str(message_string) {
                    Ok(message) => message,
                    _ => DebugEvent::Invalid
                }
            }
            _ => DebugEvent::Invalid
        }
    }
}

impl From<DebugResponse> for Message {
    fn from(msg: DebugResponse) -> Self {
        match serde_json::to_string(&msg) {
            Ok(message_string) => Message::from(&message_string),
            _ => Message::new()
        }
    }
}

impl From<&Message> for DebugResponse {
    fn from(msg: &Message) -> Self {
        match msg.as_str() {
            Some(message_string) => {
                match serde_json::from_str(message_string) {
                    Ok(message) => message,
                    _ => DebugResponse::Invalid
                }
            }
            _ => DebugResponse::Invalid
        }
    }
}

pub struct RuntimeClientConnection {
    host: String,
    port: usize,
    context: Option<zmq::Context>,
    requester: Option<Socket>,
}

impl RuntimeClientConnection {
    pub fn new(runtime_server_context: &RuntimeServerContext) -> Self {
        RuntimeClientConnection {
            host: "localhost".into(),
            port: runtime_server_context.port,
            context: None,
            requester: None,
        }
    }

    pub fn start(&mut self) -> Result<()> {
        self.context = Some(zmq::Context::new());

        if let Some(ref context) = self.context {
            self.requester = Some(context.socket(zmq::REQ)
                .chain_err(|| "Could not connect to server")?);

            if let Some(ref requester) = self.requester {
                requester.connect(&format!("tcp://{}:{}", self.host, self.port))
                    .chain_err(|| "Could not connect to server")?;
            }
        }

        Ok(())
    }

    /// Receive an event from the runtime
    pub fn client_recv(&self) -> Result<Event> {
        if let Some(ref requester) = self.requester {
            let msg = requester.recv_msg(0)
                .map_err(|e| format!("Error receiving from Server: {}", e))?;
            Ok(Event::from(&msg))
        } else {
            bail!("Client connection has not been started")
        }
    }

    pub fn client_send(&self, response: Response) -> Result<()> {
        if let Some(ref requester) = self.requester {
            requester.send(response, 0).chain_err(|| "Error sending to server")
        } else {
            bail!("Client connection has not been started")
        }
    }
}

pub struct DebuggerClientConnection {
    host: String,
    port: usize,
    context: Option<zmq::Context>,
    requester: Option<Socket>,
}

impl DebuggerClientConnection {
    pub fn new(debug_server_context: &DebugServerContext) -> Self {
        DebuggerClientConnection {
            host: "localhost".into(),
            port: debug_server_context.port,
            context: None,
            requester: None,
        }
    }

    pub fn start(&mut self) -> Result<()> {
        self.context = Some(zmq::Context::new());

        if let Some(ref context) = self.context {
            self.requester = Some(context.socket(zmq::REQ).chain_err(|| "Could not connect to server")?);

            if let Some(ref requester) = self.requester {
                requester.connect(&format!("tcp://{}:{}", self.host, self.port))
                    .chain_err(|| "Could not connect to server")?;
            }
        }

        Ok(())
    }

    /// Receive an Event from the debugger
    pub fn client_recv(&self) -> Result<DebugEvent> {
        if let Some(ref requester) = self.requester {
            let msg = requester.recv_msg(0)
                .map_err(|e| format!("Error receiving from Server: {}", e))?;
            Ok(DebugEvent::from(&msg))
        } else {
            bail!("Client connection has not been started")
        }
    }

    /// Send an Event to the debugger
    pub fn client_send(&self, response: DebugResponse) -> Result<()> {
        if let Some(ref requester) = self.requester {
            requester.send(response, 0).chain_err(|| "Error sending to server")
        } else {
            bail!("Client connection has not been started")
        }
    }
}

pub struct RuntimeServerContext {
    port: usize,
    responder: Option<zmq::Socket>,
}

impl RuntimeServerContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&mut self) {
        let context = zmq::Context::new();
        let responder = context.socket(zmq::REP).unwrap();
        responder.bind(&format!("tcp://*:{}", self.port)).unwrap();
        self.responder = Some(responder);
    }

    pub fn get_response(&self) -> Response {
        // TODO use a combinator?
        if let Some(ref responder) = self.responder {
            let mut msg = zmq::Message::new();
            responder.recv(&mut msg, 0).unwrap();
            Response::from(&msg)
        } else {
            Response::Error("Server connection not started".into())
        }
    }

    pub fn send_event(&mut self, event: Event) -> Response {
        if let Some(ref responder) = self.responder {
            let event_message = Message::from(event);
            match responder.send(event_message, 0) {
                Ok(()) => self.get_response(),
                Err(err) => {
                    error!("Error sending to client: '{}'", err);
                    Response::Error(err.to_string())
                }
            }
        } else {
            Response::Error("Server connection not started".into())
        }
    }
}

unsafe impl Send for RuntimeServerContext {}

unsafe impl Sync for RuntimeServerContext {}

impl Default for RuntimeServerContext {
    fn default() -> Self {
        RuntimeServerContext {
            port: 5555,
            responder: None,
        }
    }
}

pub struct DebugServerContext {
    port: usize,
    responder: Option<zmq::Socket>,
}

impl DebugServerContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&mut self) {
        let context = zmq::Context::new();
        let responder = context.socket(zmq::REP).unwrap();
        responder.bind(&format!("tcp://*:{}", self.port)).unwrap();
        self.responder = Some(responder);
    }

    pub fn get_response(&self) -> DebugResponse {
        if let Some(ref responder) = self.responder {
            let mut msg = zmq::Message::new();
            responder.recv(&mut msg, 0).unwrap();
            DebugResponse::from(&msg)
        } else {
            DebugResponse::Error("Server connection not started".into())
        }
    }

    pub fn send_debug_event(&self, event: DebugEvent) {
        if let Some(ref responder) = self.responder {
            let event_message = Message::from(event);
            responder.send(event_message, 0).unwrap();
        }
    }
}

impl Default for DebugServerContext {
    fn default() -> DebugServerContext {
        DebugServerContext {
            port: 5556,
            responder: None,
        }
    }
}

unsafe impl Send for DebugServerContext {}

unsafe impl Sync for DebugServerContext {}