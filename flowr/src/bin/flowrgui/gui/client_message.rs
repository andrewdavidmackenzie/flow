use std::fmt;

use flowcore::errors::Result;
use flowcore::model::submission::Submission;
use serde_derive::{Deserialize, Serialize};

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
        match serde_json::to_string(&msg) {
            Ok(message_string) => message_string,
            _ => String::new(),
        }
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