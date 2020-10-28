use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::{Receiver, Sender};

use log::error;
use serde_json::Value;

use crate::run_state::{Block, Job};

/// Types of `Params` used in communications between the debugger and the debug_client
pub enum Param {
    /// A "*" style parameter - meaning will depend on the `Command` it's use with
    Wildcard,
    /// A positive integer was specified
    Numeric(usize),
    /// A descriptor for the `Output` of a `Function` was specified
    Output((usize, String)),
    /// A descriptor for the `Inout` of a `Function` was specified
    Input((usize, usize)),
    /// A description of a "block" (when one function is blocked from running by another) was specified
    Block((usize, usize)),
}

/// A debugger Response sent by the debug_client the debugger runtime
pub enum Response {
    /// Acknowledge event processed correctly
    Ack,
    /// Set a `breakpoint` - with an optional parameter
    Breakpoint(Option<Param>),
    /// `continue` execution of the flow
    Continue,
    /// `delete` an existing breakpoint - with an optional parameter
    Delete(Option<Param>),
    /// `enter` the debugger at the next opportunity the runtime has
    EnterDebugger,
    /// `exit` the debugger and runtime
    ExitDebugger,
    /// `inspect` the current state
    Inspect,
    /// `list` existing breakpoints
    List,
    /// `print` a function or functions state
    Print(Option<Param>),
    /// `reset` flow execution back to the initial state
    RunReset,
    /// `step` forward in flow execution by executing one (default) or more `Jobs`
    Step(Option<Param>),
    /// Get the state of the `Flow`
    GetState,
    /// Get the state of a specific `Function`
    GetFunctionState(usize),
    /// An error on the client side
    Error(String)
}

/// A run-time event that the debugger communicates to the debug_client for it to decide
/// what to do, or what to request of the user
pub enum Event {
    /// A `Job` ran to completion by a function - includes:  job_id, function_id
    JobCompleted(usize, usize, Option<Value>),
    /// Entering the debugger
    EnteringDebugger,
    /// The debugger/run-time is exiting
    ExitingDebugger,
    /// The run-time is about to send a `Job` for execution - an opportunity to break
    /// includes: job_id, function_id
    PriorToSendingJob(usize, usize),
    /// A breakpoint on a `Block` between two functions was encountered
    /// includes: blocked_id, blocking_id, blocking_io_number
    BlockBreakpoint(Block),
    /// A breakpoint on a `Value` being sent between two functions was encountered
    /// includes: source_process_id, output_route, value, destination_id, input_number));
    DataBreakpoint(usize, String, Value, usize, usize),
    /// A panic occurred executing a `Flows` `Job` -  includes the output of the job that panicked
    Panic(String, usize),
    /// There was an error executing the Job
    JobError(Job),
    /// Execution of the flow has started
    ExecutionStarted,
    /// Execution of the flow has ended
    ExecutionEnded,
    /// A check has detected that there is a deadlock between functions impeding more execution
    Deadlock(String),
    /// A value is being sent from the output of one function to the input of another
    /// includes: source_process_id, value, destination_id, input_number
    SendingValue(usize, Value, usize, usize),
    /// An error was detected - includes: A string describing the error
    Error(String),
    /// A message for display to the user of the debug_client
    Message(String),
    /// The run-time is resetting the status back to the initial state
    Resetting,
    /// Debugger is blocked waiting for a command before proceeding
    WaitingForCommand(usize)
}

/// debug_clients must implement this trait
pub trait DebugClient: Sync + Send {
    /// Called to send an event to the debug_client
    fn send_event(&self, event: Event);
}

#[derive(Debug)]
pub struct ChannelDebugClient {
    /// A channel to send events to a debug client on
    debug_event_channel_tx: Sender<Event>,
    /// The other end of the channel a debug client can receive events on
    debug_event_channel_rx: Arc<Mutex<Receiver<Event>>>,
    /// A channel to for a debug client to send responses on
    debug_response_channel_tx: Sender<Response>,
    /// This end of the channel where coordinator will receive events from a debug client on
    debug_response_channel_rx: Receiver<Response>,
}

impl ChannelDebugClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_channels(&self) -> (Arc<Mutex<Receiver<Event>>>, Sender<Response>) {
        (self.debug_event_channel_rx.clone(), self.debug_response_channel_tx.clone())
    }

    pub fn get_response(&self) -> Response {
        match self.debug_response_channel_rx.recv() {
            Ok(response) => response,
            Err(err) => {
                error!("Error receiving response from debug client: '{}'", err);
                Response::Error(err.to_string())
            }
        }
    }
}

impl Default for ChannelDebugClient {
    fn default() -> ChannelDebugClient {
        let (debug_event_channel_tx, debug_event_channel_rx) = mpsc::channel();
        let (debug_response_channel_tx, debug_response_channel_rx) = mpsc::channel();
        ChannelDebugClient{
            debug_event_channel_tx,
            debug_event_channel_rx: Arc::new(Mutex::new(debug_event_channel_rx)),
            debug_response_channel_tx,
            debug_response_channel_rx,
        }
    }
}

impl DebugClient for ChannelDebugClient {
    fn send_event(&self, event: Event) {
        let _ = self.debug_event_channel_tx.send(event);
    }
}

unsafe impl Send for ChannelDebugClient {}

unsafe impl Sync for ChannelDebugClient {}
