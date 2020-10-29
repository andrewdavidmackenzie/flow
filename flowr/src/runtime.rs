use crate::coordinator::Submission;
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