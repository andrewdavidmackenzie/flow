use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender};

use crate::coordinator::{Coordinator, Submission};
#[cfg(feature = "debugger")]
use crate::debug_client::Event as DebugEvent;
#[cfg(feature = "debugger")]
use crate::debug_client::Response as DebugResponse;
use crate::errors::*;
use crate::runtime_client::{Event, Response};
use crate::runtime_client::Response::ClientSubmission;

pub struct RuntimeConnection {
    channels: (Arc<Mutex<Receiver<Event>>>, Sender<Response>)
}

impl RuntimeConnection {
    pub fn new(coordinator: &Coordinator) -> Self {
        RuntimeConnection {
            channels: coordinator.get_client_channels()
        }
    }

    /// Send a `Submission` of a flow to the `Coordinator` for execution
    pub fn client_submit(&self, submission: Submission) -> Result<()> {
        self.channels.1.send(ClientSubmission(submission))
            .chain_err(|| "Could not send Submission to the Coordinator")
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

pub struct DebuggerConnection {
    channels: (Arc<Mutex<Receiver<DebugEvent>>>, Sender<DebugResponse>),
}

impl DebuggerConnection {
    pub fn new(coordinator: &Coordinator) -> Self {
        DebuggerConnection {
            channels: coordinator.get_debug_channels()
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
