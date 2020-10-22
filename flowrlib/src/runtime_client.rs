use std::fmt::Debug;

/// A run-time command sent from a run-time function to a runtime_client
pub enum Command {
    /// Command to perform anything needed when a flow starts executing
    FlowStart,
    /// Command to perform anything needed when a flow stops executing
    FlowEnd,
    /// Command to print a String of contents to stdout
    Stdout(String),
    /// Command to print a String of contents to stderr
    Stderr(String),
    /// Read characters possible from Stdin
    GetStdin,
    /// Read a line of characters from Stdin
    GetLine,
    /// Get the arguments for the flow
    GetArgs,
    /// Write to a file
    Write(String, Vec<u8>),
    /// Write a pixel to an ImageBuffer
    PixelWrite((u32, u32), (u8, u8, u8), (u32, u32), String),
    /// End of File sent to Stdout
    StdoutEOF,
    /// End of File sent to Stderr
    StderrEOF,
}

/// A `Response` from the runtime_client to the run-time functions
#[derive(PartialEq)]
pub enum Response {
    /// Simple acknowledgement
    Ack,
    /// A String read from Stdin
    Stdin(String),
    /// A line of text read from Stdin using readline
    Line(String),
    /// An Vector of Strings that are the flow's arguments
    Args(Vec<String>),
    /// An Error occurred on the runtime_client
    Error(String),
    /// EOF was detected on input reading using Stdin
    GetStdinEOF,
    /// EOF was detected on input reading using Readline
    GetLineEOF,
}

/// runtime_clients must implement this trait
pub trait RuntimeClient: Sync + Send + Debug {
    /// Called to send the next command to the runtime_client and get the response
    fn send_command(&mut self, command: Command) -> Response;
}