use std::fmt::Debug;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::{Receiver, Sender};

use log::error;

use crate::coordinator::Submission;
use crate::errors::*;
#[cfg(feature = "metrics")]
use crate::metrics::Metrics;

/// A run-time Event sent from the run-time to a runtime_client
pub enum Event {
    /// A flow has started executing
    FlowStart,
    /// A flow has stopped executing
    #[cfg(feature = "metrics")]
    FlowEnd(Metrics),
    #[cfg(not(feature = "metrics"))]
    FlowEnd,
    /// A String of contents was sent to stdout
    Stdout(String),
    /// A String of contents was sent to stderr
    Stderr(String),
    /// A Request to read from Stdin
    GetStdin,
    /// A Request to read a line of characters from Stdin
    GetLine,
    /// A Request to get the arguments for the flow
    GetArgs,
    /// A Request to write a series of bytes to a file
    Write(String, Vec<u8>),
    /// A Request to write a pixel to an ImageBuffer
    PixelWrite((u32, u32), (u8, u8, u8), (u32, u32), String),
    /// A Request to snd EOF to Stdout
    StdoutEOF,
    /// A Request to snd EOF to Stderr
    StderrEOF,
}

unsafe impl Send for Event {}

unsafe impl Sync for Event {}

/// A `Response` from the runtime_client to the run-time
#[derive(PartialEq)]
pub enum Response {
    /// Simple acknowledgement
    Ack,
    /// A String read from Stdin
    Stdin(String),
    /// A line of text read from Stdin using readline
    Line(String),
    /// A Vector of Strings that are the flow's arguments
    Args(Vec<String>),
    /// An Error occurred in the runtime_client
    Error(String),
    /// EOF was detected on input reading using Stdin
    GetStdinEOF,
    /// EOF was detected on input reading Stdin using Readline
    GetLineEOF,
    /// Client is exiting Event loop
    ClientExiting,
    /// A submission from the client for execution
    ClientSubmission(Submission),
    /// Client requests that server enters the ddebugger at the next opportunity
    EnterDebugger
}

unsafe impl Send for Response {}

unsafe impl Sync for Response {}

/// runtime_clients must implement this trait
pub trait RuntimeClient: Sync + Send + Debug {
    /// Called to send the event to the runtime_client and get the response
    fn send_event(&mut self, event: Event) -> Response;
}

#[derive(Debug)]
pub struct ChannelRuntimeClient {
    /// A channel to sent events to a client on
    client_event_channel_tx: Sender<Event>,
    /// The other end of the channel a client can receive events of
    client_event_channel_rx: Arc<Mutex<Receiver<Event>>>,
    /// A channel to for a client to send responses on
    client_response_channel_tx: Sender<Response>,
    /// This end of the channel where coordinator will receive events from a client on
    client_response_channel_rx: Receiver<Response>,
}

impl ChannelRuntimeClient {
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
}

unsafe impl Send for ChannelRuntimeClient {}

unsafe impl Sync for ChannelRuntimeClient {}

impl Default for ChannelRuntimeClient {
    fn default() -> Self {
        let (client_event_channel_tx, client_event_channel_rx) = mpsc::channel();
        let (client_response_channel_tx, client_response_channel_rx) = mpsc::channel();

        ChannelRuntimeClient {
            client_event_channel_tx,
            client_event_channel_rx: Arc::new(Mutex::new(client_event_channel_rx)),
            client_response_channel_tx,
            client_response_channel_rx,
        }
    }
}

impl RuntimeClient for ChannelRuntimeClient {
    fn send_event(&mut self, event: Event) -> Response {
        match self.client_event_channel_tx.send(event) {
            Ok(()) => self.get_response(),
            Err(err) => {
                error!("Error sending to client: '{}'", err);
                Response::Error(err.to_string())
            }
        }
    }
}