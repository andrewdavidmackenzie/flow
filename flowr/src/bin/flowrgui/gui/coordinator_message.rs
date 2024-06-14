use std::fmt;

use serde_derive::{Deserialize, Serialize};

use flowcore::errors::Result;
use flowcore::model::metrics::Metrics;

use crate::gui::client_message::ClientMessage;

/// An Message sent from the runtime server to a `runtime_client`
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CoordinatorMessage {
    #[serde(skip_deserializing, skip_serializing)]
    /// ** These messages are used to communicate to the app the connection status to the Coordinator
    /// A connection has been made
    Connected(tokio::sync::mpsc::Sender<ClientMessage>),
    /// Connection with the Coordinator has been lost
    Disconnected(String),
    /// ** These messages are used to implement the `SubmissionProtocol` between the coordinator
    /// and the `cli_client`
    /// A flow has started executing
    FlowStart,
    /// A flow has stopped executing
    FlowEnd(Metrics),
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
            "{}",
            match self {
                CoordinatorMessage::Connected(_) => "Connected",
                CoordinatorMessage::Disconnected(_) => "Disconnected",
                CoordinatorMessage::FlowEnd(_) => "FlowEnd",
                CoordinatorMessage::FlowStart => "FlowStart",
                CoordinatorMessage::CoordinatorExiting(_) => "CoordinatorExiting",
                CoordinatorMessage::Stdout(_) => "Stdout",
                CoordinatorMessage::Stderr(_) => "Stderr",
                CoordinatorMessage::GetStdin => "GetStdIn",
                CoordinatorMessage::GetLine(_) => "GetLine",
                CoordinatorMessage::GetArgs => "GetArgs",
                CoordinatorMessage::Read(_) => "Read",
                CoordinatorMessage::Write(_, _) => "Write",
                CoordinatorMessage::PixelWrite(_, _, _, _) => "PixelWrite",
                CoordinatorMessage::StdoutEof => "StdOutEof",
                CoordinatorMessage::StderrEof => "StdErrEof",
                CoordinatorMessage::Invalid => "Invalid",
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