use zmq;
/// This is the message-queue implementation of the lib.client_server communications
use zmq::Message;

#[cfg(feature = "debugger")]
use crate::debug::Event as DebugEvent;
#[cfg(feature = "debugger")]
use crate::debug::Response as DebugResponse;
use crate::errors::*;
use crate::runtime::{Event, Response};

impl From<&Event> for Message {
    fn from(msg: &Event) -> Self {
        match serde_json::to_string(msg) {
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
            },
            _ => Event::Invalid
        }
    }
}

impl From<&Response> for Message {
    fn from(msg: &Response) -> Self {
        match serde_json::to_string(msg) {
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
            },
            _ => Response::Invalid
        }
    }
}

impl From<&DebugEvent> for Message {
    fn from(msg: &DebugEvent) -> Self {
        match serde_json::to_string(msg) {
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
            },
            _ => DebugEvent::Invalid
        }
    }
}

impl From<&DebugResponse> for Message {
    fn from(msg: &DebugResponse) -> Self {
        match serde_json::to_string(msg) {
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
            },
            _ => DebugResponse::Invalid
        }
    }
}

pub struct RuntimeClientConnection {
}

impl RuntimeClientConnection {
    pub fn new(_runtime_server_context: &RuntimeServerContext) -> Self {
        RuntimeClientConnection {
        }
    }

    /// Receive an event from the runtime
    pub fn client_recv(&self) -> Result<Event> {
        Ok(Event::Invalid)
    }

    pub fn client_send(&self, _response: Response) -> Result<()> {
        Ok(())
    }
}

pub struct DebuggerClientConnection {
}

impl DebuggerClientConnection {
    pub fn new(_debug_server_context: &DebugServerContext) -> Self {
        DebuggerClientConnection {
        }
    }

    /// Receive an Event from the debugger
    pub fn client_recv(&self) -> Result<DebugEvent> {
        Ok(DebugEvent::Invalid)
    }

    /// Send an Event to the debugger
    pub fn client_send(&self, _response: DebugResponse) -> Result<()> {
        Ok(())
    }
}

pub struct RuntimeServerContext {
    _context: zmq::Context,
    _responder: zmq::Socket,
}

impl RuntimeServerContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn send_response(&self, _response: Response) -> Result<()> {
        Ok(())
    }

    pub fn get_response(&self) -> Response {
        Response::Invalid
    }

    pub fn send_event(&mut self, _event: Event) -> Response {
        Response::Invalid
    }
}

unsafe impl Send for RuntimeServerContext {}

unsafe impl Sync for RuntimeServerContext {}

impl Default for RuntimeServerContext {
    fn default() -> Self {
        let _context = zmq::Context::new();
        let _responder = _context.socket(zmq::REP).unwrap();

        _responder.bind("tcp://*:5555").unwrap();

        RuntimeServerContext {
            _context,
            _responder
        }
    }
}

pub struct DebugServerContext {
    _context: zmq::Context,
    _responder: zmq::Socket,
}

impl DebugServerContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_response(&self) -> DebugResponse {
        DebugResponse::Invalid
    }

    pub fn send_debug_event(&self, _event: DebugEvent) {

    }
}

impl Default for DebugServerContext {
    fn default() -> DebugServerContext {
        let _context = zmq::Context::new();
        let _responder = _context.socket(zmq::REP).unwrap();

        _responder.bind("tcp://*:5556").unwrap();

        DebugServerContext {
            _context,
            _responder
        }
    }
}

unsafe impl Send for DebugServerContext {}

unsafe impl Sync for DebugServerContext {}