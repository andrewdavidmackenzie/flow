use log::error;
/// This is the channel-based implementation of the client_server communications
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::mpsc;

use crate::coordinator::Coordinator;
#[cfg(feature = "debugger")]
use crate::debug::Event as DebugEvent;
#[cfg(feature = "debugger")]
use crate::debug::Response as DebugResponse;
use crate::debugger::Debugger;
use crate::errors::*;
use crate::runtime::{Event, Response};

pub struct RuntimeClientConnection {
    channels: (Arc<Mutex<Receiver<Event>>>, Sender<Response>)
}

impl RuntimeClientConnection {
    pub fn new(coordinator: &Coordinator) -> Self {
        let context = coordinator.get_server_context();
        let guard = context.lock().unwrap();

        RuntimeClientConnection {
            channels: guard.get_client_channels()
        }
    }

    /// Receive an event from the runtime
    pub fn client_recv(&self) -> Result<Event> {
        let guard = self.channels.0.lock()
            .map_err(|_| "Could not lock client Event reception channel")?;
        guard.recv().chain_err(|| "Error receiving Event from client channel")
    }

    pub fn client_send(&self, response: Response) -> Result<()> {
        self.channels.1.send(response).chain_err(|| "Error sending on client channel")
    }
}

pub struct DebuggerClientConnection {
    channels: (Arc<Mutex<Receiver<DebugEvent>>>, Sender<DebugResponse>),
}

impl DebuggerClientConnection {
    pub fn new(debugger: &Debugger) -> Self {
        DebuggerClientConnection {
            channels: debugger.server_context.get_channels()
        }
    }

    /// Receive an Event from the debugger
    pub fn client_recv(&self) -> Result<DebugEvent> {
        let guard = self.channels.0.lock()
            .map_err(|_| "Could not lock debug Event reception channel")?;
        guard.recv().chain_err(|| "Error receiving Event from debug channel")
    }

    /// Send an Event to the debugger
    pub fn client_send(&self, response: DebugResponse) -> Result<()> {
        self.channels.1.send(response).chain_err(|| "Error sending on Debug channel")
    }
}

#[derive(Debug)]
pub struct RuntimeServerContext {
    /// A channel to sent events to a client on
    client_event_channel_tx: Sender<Event>,
    /// The other end of the channel a client can receive events of
    client_event_channel_rx: Arc<Mutex<Receiver<Event>>>,
    /// A channel to for a client to send responses on
    client_response_channel_tx: Sender<Response>,
    /// This end of the channel where coordinator will receive events from a client on
    client_response_channel_rx: Receiver<Response>,
}

impl RuntimeServerContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_client_channels(&self) -> (Arc<Mutex<Receiver<Event>>>, Sender<Response>) {
        (self.client_event_channel_rx.clone(), self.client_response_channel_tx.clone())
    }

    pub fn send_response(&self, response: Response) -> Result<()> {
        self.client_response_channel_tx.send(response)
            .chain_err(|| "Could not send Submission to the Coordinator")
    }

    pub fn get_response(&self) -> Response {
        match self.client_response_channel_rx.recv() {
            Ok(response) => response,
            Err(err) => {
                error!("Error receiving response from client: '{}'", err);
                Response::Error(err.to_string())
            }
        }
    }

    pub fn send_event(&mut self, event: Event) -> Response {
        match self.client_event_channel_tx.send(event) {
            Ok(()) => self.get_response(),
            Err(err) => {
                error!("Error sending to client: '{}'", err);
                Response::Error(err.to_string())
            }
        }
    }
}

unsafe impl Send for RuntimeServerContext {}

unsafe impl Sync for RuntimeServerContext {}

impl Default for RuntimeServerContext {
    fn default() -> Self {
        let (client_event_channel_tx, client_event_channel_rx) = mpsc::channel();
        let (client_response_channel_tx, client_response_channel_rx) = mpsc::channel();

        RuntimeServerContext {
            client_event_channel_tx,
            client_event_channel_rx: Arc::new(Mutex::new(client_event_channel_rx)),
            client_response_channel_tx,
            client_response_channel_rx,
        }
    }
}

#[derive(Debug)]
pub struct DebugServerContext {
    /// A channel to send events to a debug client on
    debug_event_channel_tx: Sender<DebugEvent>,
    /// The other end of the channel a debug client can receive events on
    debug_event_channel_rx: Arc<Mutex<Receiver<DebugEvent>>>,
    /// A channel to for a debug client to send responses on
    debug_response_channel_tx: Sender<DebugResponse>,
    /// This end of the channel where coordinator will receive events from a debug client on
    debug_response_channel_rx: Receiver<DebugResponse>,
}

impl DebugServerContext {
    pub fn new() -> Self {
        Self::default()
    }

    fn get_channels(&self) -> (Arc<Mutex<Receiver<DebugEvent>>>, Sender<DebugResponse>) {
        (self.debug_event_channel_rx.clone(), self.debug_response_channel_tx.clone())
    }

    pub fn get_response(&self) -> DebugResponse {
        match self.debug_response_channel_rx.recv() {
            Ok(response) => response,
            Err(err) => {
                error!("Error receiving response from debug client: '{}'", err);
                DebugResponse::Error(err.to_string())
            }
        }
    }

    pub fn send_debug_event(&self, event: DebugEvent) {
        let _ = self.debug_event_channel_tx.send(event);
    }
}

impl Default for DebugServerContext {
    fn default() -> DebugServerContext {
        let (debug_event_channel_tx, debug_event_channel_rx) = mpsc::channel();
        let (debug_response_channel_tx, debug_response_channel_rx) = mpsc::channel();
        DebugServerContext {
            debug_event_channel_tx,
            debug_event_channel_rx: Arc::new(Mutex::new(debug_event_channel_rx)),
            debug_response_channel_tx,
            debug_response_channel_rx,
        }
    }
}

unsafe impl Send for DebugServerContext {}

unsafe impl Sync for DebugServerContext {}
