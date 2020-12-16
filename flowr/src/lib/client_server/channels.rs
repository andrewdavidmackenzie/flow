/// This is the channel-based implementation of the lib.client_server communications
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::mpsc;

#[cfg(feature = "debugger")]
use crate::debug::Event as DebugEvent;
#[cfg(feature = "debugger")]
use crate::debug::Response as DebugResponse;
use crate::errors::*;
use crate::runtime::{Event, Response};

pub struct RuntimeClientConnection {
    channels: (Arc<Mutex<Receiver<Event>>>, Sender<Response>)
}

impl RuntimeClientConnection {
    pub fn new(runtime_server_context: &RuntimeServerConnection) -> Self {
        RuntimeClientConnection {
            channels: runtime_server_context.get_client_channels()
        }
    }

    pub fn start(&mut self) -> Result<()> {
        Ok(())
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

pub struct DebugClientConnection {
    channels: (Arc<Mutex<Receiver<DebugEvent>>>, Sender<DebugResponse>),
}

impl DebugClientConnection {
    pub fn new(debug_server_context: &DebugServerConnection) -> Self {
        DebugClientConnection {
            channels: debug_server_context.get_channels()
        }
    }

    pub fn start(&mut self) -> Result<()> {
        Ok(())
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
pub struct RuntimeServerConnection {
    /// A channel to sent events to a client on
    client_event_channel_tx: Sender<Event>,
    /// The other end of the channel a client can receive events of
    client_event_channel_rx: Arc<Mutex<Receiver<Event>>>,
    /// A channel to for a client to send responses on
    client_response_channel_tx: Sender<Response>,
    /// This end of the channel where coordinator will receive events from a client on
    client_response_channel_rx: Receiver<Response>,
}

impl RuntimeServerConnection {
    pub fn new(_server_hostname: Option<&str>) -> Self {
        let (client_event_channel_tx, client_event_channel_rx) = mpsc::channel();
        let (client_response_channel_tx, client_response_channel_rx) = mpsc::channel();

        RuntimeServerConnection {
            client_event_channel_tx,
            client_event_channel_rx: Arc::new(Mutex::new(client_event_channel_rx)),
            client_response_channel_tx,
            client_response_channel_rx,
        }
    }

    pub fn start(&self) -> Result<()> {
        Ok(())
    }

    /// Get the channels a client should use to send to the server
    fn get_client_channels(&self) -> (Arc<Mutex<Receiver<Event>>>, Sender<Response>) {
        // Clone of Arc and Sender is OK
        (self.client_event_channel_rx.clone(), self.client_response_channel_tx.clone())
    }

    /// Get a response from the client to the server
    pub fn get_response(&self) -> Result<Response> {
        self.client_response_channel_rx.recv()
            .chain_err(|| "Error receiving response from client")
    }

    /// Send a server event to the client
    pub fn send_event(&mut self, event: Event) -> Result<Response> {
        self.client_event_channel_tx.send(event)
            .map_err(|e| format!("Error sending to client: '{}'", e))?;

        self.get_response()
    }
}

#[derive(Debug)]
pub struct DebugServerConnection {
    /// A channel to send events to a debug client on
    debug_event_channel_tx: Sender<DebugEvent>,
    /// The other end of the channel a debug client can receive events on
    debug_event_channel_rx: Arc<Mutex<Receiver<DebugEvent>>>,
    /// A channel to for a debug client to send responses on
    debug_response_channel_tx: Sender<DebugResponse>,
    /// This end of the channel where coordinator will receive events from a debug client on
    debug_response_channel_rx: Receiver<DebugResponse>,
}

impl DebugServerConnection {
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

    pub fn start(&self) -> Result<()> {
        Ok(())
    }

    fn get_channels(&self) -> (Arc<Mutex<Receiver<DebugEvent>>>, Sender<DebugResponse>) {
        // Clone of Arc and Sender is OK
        (self.debug_event_channel_rx.clone(), self.debug_response_channel_tx.clone())
    }

    pub fn get_response(&self) -> Result<DebugResponse> {
        self.debug_response_channel_rx.recv()
            .chain_err(|| "Error receiving response from debug client")
    }

    pub fn send_event(&self, event: DebugEvent) -> Result<()> {
        self.debug_event_channel_tx.send(event)
            .chain_err(|| "Could not send Debug event from Debug server")
    }
}