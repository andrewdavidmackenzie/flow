use std::fmt;

use flowcore::errors::Result;
use flowcore::model::submission::Submission;
use serde_derive::{Deserialize, Serialize};

/// A message from the client to the coordinator
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientMessage {
    /// A submission for execution
    ClientSubmission(Submission),
    /// Request to enter debugger
    EnterDebugger,
    /// Acknowledgement
    Ack,
    /// Stdin content
    Stdin(String),
    /// A line of text
    Line(String),
    /// Flow arguments
    Args(Vec<String>),
    /// An error
    Error(String),
    /// EOF on stdin
    GetStdinEof,
    /// EOF on readline
    GetLineEof,
    /// Invalid message
    Invalid,
    /// File contents
    FileContents(String, Vec<u8>),
    /// Client is exiting
    ClientExiting(Result<()>),
}

impl fmt::Display for ClientMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ClientMessage::Ack => "Ack",
                ClientMessage::Stdin(_) => "Stdin",
                ClientMessage::Line(_) => "Line",
                ClientMessage::Args(_) => "Args",
                ClientMessage::Error(_) => "Error",
                ClientMessage::GetStdinEof => "GetStdinEof",
                ClientMessage::GetLineEof => "GetLineEof",
                ClientMessage::ClientExiting(_) => "ClientExiting",
                ClientMessage::ClientSubmission(_) => "ClientSubmission",
                ClientMessage::EnterDebugger => "EnterDebugger",
                ClientMessage::Invalid => "Invalid",
                ClientMessage::FileContents(_, _) => "FileContents",
            }
        )
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
