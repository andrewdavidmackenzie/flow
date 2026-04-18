use std::fmt;

use serde_derive::{Deserialize, Serialize};

use flowcore::errors::Result;
use flowcore::model::metrics::Metrics;

use crate::gui::client_message::ClientMessage;

/// A message sent from the coordinator to the client
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CoordinatorMessage {
    #[serde(skip_deserializing, skip_serializing)]
    /// A connection has been made
    Connected(tokio::sync::mpsc::Sender<ClientMessage>),
    /// Connection with the Coordinator has been lost
    Disconnected(String),
    /// A flow has started executing
    FlowStart,
    /// A flow has stopped executing
    FlowEnd(Metrics),
    /// Coordinator is exiting
    CoordinatorExiting(Result<()>),
    /// A String sent to stdout
    Stdout(String),
    /// A String sent to stderr
    Stderr(String),
    /// Request to read from stdin
    GetStdin,
    /// Request to read a line with prompt
    GetLine(String),
    /// Request to get flow arguments
    GetArgs,
    /// Request to read a file
    Read(String),
    /// Request to write a file
    Write(String, Vec<u8>),
    /// Request to write a pixel
    PixelWrite((u32, u32), (u8, u8, u8), (u32, u32), String),
    /// EOF on stdout
    StdoutEof,
    /// EOF on stderr
    StderrEof,
    /// Invalid message
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
