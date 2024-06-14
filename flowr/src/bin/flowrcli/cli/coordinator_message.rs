use std::fmt;

use serde_derive::{Deserialize, Serialize};

use flowcore::errors::Result;
#[cfg(feature = "metrics")]
use flowcore::model::metrics::Metrics;
use flowcore::model::submission::Submission;

/// An Message sent from the runtime server to a `runtime_client`
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CoordinatorMessage {
    /// ** These messages are used to implement the `SubmissionProtocol` between the coordinator
    /// and the `cli_client`
    /// A flow has started executing
    FlowStart,
    /// A flow has stopped executing
    #[cfg(feature = "metrics")]
    FlowEnd(Metrics),
    /// A flow has stopped executing
    #[cfg(not(feature = "metrics"))]
    FlowEnd,
    /// Coordinator is exiting, with a result (OK, or Err)
    CoordinatorExiting(Result<()>),

    /// ** These messages are used to implement the context functions between the `cli_runtime_server`
    /// that runs as part of the `Coordinator` and the `cli_runtime_client` that interacts with
    /// STDIO
    /// A String of contents was sent to stdout
    Stdout(String),
    /// A String of contents was sent to stderr
    Stderr(String),
    /// A Request to read from Stdin
    GetStdin,
    /// A Request to read a line of characters from Stdin, with a String prompt to show
    GetLine(String),
    /// A Request to get the arguments for the flow
    GetArgs,
    /// A Request to read bytes from a file
    Read(String),
    /// A Request to write a series of bytes to a file
    Write(String, Vec<u8>),
    /// A Request to write a pixel to an `ImageBuffer`
    PixelWrite((u32, u32), (u8, u8, u8), (u32, u32), String),
    /// A Request to snd EOF to Stdout
    StdoutEof,
    /// A Request to snd EOF to Stderr
    StderrEof,
    /// Invalid - used when deserialization goes wrong
    Invalid,
}

impl fmt::Display for CoordinatorMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ServerMessage {}",
            match self {
                #[cfg(feature = "metrics")]
                CoordinatorMessage::FlowEnd(_) => "FlowEnd".into(),
                #[cfg(not(feature = "metrics"))]
                CoordinatorMessage::FlowEnd => "FlowEnd".into(),
                CoordinatorMessage::FlowStart => "FlowStart".into(),
                CoordinatorMessage::CoordinatorExiting(result) =>
                    format!("CoordinatorExiting with result: {result:?}"),
                CoordinatorMessage::Stdout(_) => "Stdout".into(),
                CoordinatorMessage::Stderr(_) => "Stderr".into(),
                CoordinatorMessage::GetStdin => "GetStdIn".into(),
                CoordinatorMessage::GetLine(_) => "GetLine".into(),
                CoordinatorMessage::GetArgs => "GetArgs".into(),
                CoordinatorMessage::Read(_) => "Read".into(),
                CoordinatorMessage::Write(_, _) => "Write".into(),
                CoordinatorMessage::PixelWrite(_, _, _, _) => "PixelWrite".into(),
                CoordinatorMessage::StdoutEof => "StdOutEof".into(),
                CoordinatorMessage::StderrEof => "StdErrEof".into(),
                CoordinatorMessage::Invalid => "Invalid".into(),
            }
        )
    }
}

/// A simple struct with File `MetaData` for passing from Client to Coordinator - `std::fs::MetaData`
/// Doesn't Serialize/Deserialize etc.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct FileMetaData {
    /// Was the Path inspected a file or not
    pub is_file: bool,
    /// Was the Path inspected a directory or not
    pub is_dir: bool,
}

/// A Message from the a client to the Coordinator
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientMessage {
    /// ** These messages are used to implement the `SubmissionProtocol` between the Coordinator
    /// and the client
    /// A submission from the client for execution
    ClientSubmission(Submission),
    /// Client requests that server enters the ddebugger at the next opportunity
    EnterDebugger,

    /// ** These messages are used to implement the context functions between the `cli_runtime_client`
    /// and the `cli_runtime_server` that runs as part of the `Coordinator`
    /// Simple acknowledgement from Client to a `ServerMessage`
    Ack,
    /// A String read from Stdin on Client, sent to the Server
    Stdin(String),
    /// A line of text read from Stdin using readline from Stdin on Client, sent to the Server
    Line(String),
    /// A Vector of Strings that are the flow's arguments from Client, sent to the Server
    Args(Vec<String>),
    /// An Error occurred in the `runtime_client`
    Error(String),
    /// EOF was detected on input reading using Stdin
    GetStdinEof,
    /// EOF was detected on input reading Stdin using Readline
    GetLineEof,
    /// Invalid - used when deserialization goes wrong
    Invalid,
    /// Contents read from a file
    FileContents(String, Vec<u8>),

    /// ** This message is just internal to the client and not sent to the Coordinator
    /// Client is exiting Event loop
    ClientExiting(Result<()>),
}

impl fmt::Display for ClientMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ClientMessage {}",
            match self {
                ClientMessage::Ack => "Ack".into(),
                ClientMessage::Stdin(_) => "Stdin".into(),
                ClientMessage::Line(_) => "Line".into(),
                ClientMessage::Args(_) => "Args".into(),
                ClientMessage::Error(_) => "Error".into(),
                ClientMessage::GetStdinEof => "GetStdinEof".into(),
                ClientMessage::GetLineEof => "GetLineEof".into(),
                ClientMessage::ClientExiting(result) =>
                    format!("ClientExiting with server result: {result:?}"),
                ClientMessage::ClientSubmission(_) => "ClientSubmission".into(),
                ClientMessage::EnterDebugger => "EnterDebugger".into(),
                ClientMessage::Invalid => "Invalid".into(),
                ClientMessage::FileContents(_, _) => "FileContents".into(),
            }
        )
    }
}

impl From<CoordinatorMessage> for String {
    fn from(msg: CoordinatorMessage) -> Self {
        serde_json::to_string(&msg).unwrap_or_default()
    }
}

impl From<String> for CoordinatorMessage {
    fn from(string: String) -> Self {
        match serde_json::from_str(&string) {
            Ok(message) => message,
            _ => CoordinatorMessage::Invalid,
        }
    }
}

impl From<ClientMessage> for String {
    fn from(msg: ClientMessage) -> Self {
        serde_json::to_string(&msg).unwrap_or_default()
    }
}

impl From<String> for ClientMessage {
    fn from(string: String) -> Self {
        match serde_json::from_str(&string) {
            Ok(message) => message,
            _ => ClientMessage::Invalid,
        }
    }
}
