use std::fmt::Debug;

/// A run-time command sent from a run-time function to a runtime_client
pub enum Command {
    /// Command to print a String of contents to stdout
    Stdout(String),
    /// Command to print a String of contents to stderr
    Stderr(String),
    /// Read characters possible from Stdin
    Stdin,
    /// Read a line of characters from Stdlin
    Readline,
    /// Get the arguments for the flow
    Args,
    /// Write to a file
    Write(String, Vec<u8>)
}

/// A `Response` from the runtime_client to the run-time functions
#[derive(PartialEq)]
pub enum Response {
    /// Simple acknowledgement
    Ack,
    /// A String read from Stdin
    Stdin(String),
    /// A libne of text read from Stdin using readline
    Readline(String),
    /// An Vector of Strings that are the flow's arguments
    Args(Vec<String>),
    /// An Error occurred on the runtime_client
    Error(String),
    /// EOF was detected on input
    EOF
}

/// runtime_clients must implement this trait
pub trait RuntimeClient: Sync + Send + Debug {
    /// Called at init to initalize the client
    fn init(&self);
    /// Called to send the next command to the runtime_client and get the response
    fn send_command(&self, command: Command) -> Response;
}