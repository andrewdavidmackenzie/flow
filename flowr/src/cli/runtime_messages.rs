use std::fmt;
use std::path::PathBuf;

use serde_derive::{Deserialize, Serialize};

use flowcore::errors::*;
#[cfg(feature = "metrics")]
use flowcore::model::metrics::Metrics;
use flowcore::model::submission::Submission;

/// An Message sent from the runtime server to a runtime_client
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerMessage {
    /// ** These messages are used to implement the `SubmissionProtocol` between the cli_runtime_server
    /// and the cli_runtime_client
    /// A flow has started executing
    #[cfg(feature = "submission")] FlowStart,
    /// A flow has stopped executing
    #[cfg(all(feature = "submission", feature = "metrics"))]
    FlowEnd(Metrics),
    /// A flow has stopped executing
    #[cfg(all(feature = "submission", not(feature = "metrics")))]
    FlowEnd,
    /// Server is exiting, with a result (OK, or Err)
    #[cfg(feature = "submission")] ServerExiting(Result<()>),

    /// ** These messages are used to implement the context functions between the cli_runtime_server
    /// that runs as part of the `Coordinator` and the cli_runtime_client that interacts with
    /// STDIO
    /// A String of contents was sent to stdout
    #[cfg(feature = "context")] Stdout(String),
    /// A String of contents was sent to stderr
    #[cfg(feature = "context")] Stderr(String),
    /// A Request to read from Stdin
    #[cfg(feature = "context")] GetStdin,
    /// A Request to read a line of characters from Stdin
    #[cfg(feature = "context")] GetLine,
    /// A Request to get the arguments for the flow
    #[cfg(feature = "context")] GetArgs,
    /// A Request to read bytes from a file
    #[cfg(feature = "context")] Read(PathBuf),
    /// A Request to write a series of bytes to a file
    #[cfg(feature = "context")] Write(String, Vec<u8>),
    /// A Request to write a pixel to an ImageBuffer
    #[cfg(feature = "context")] PixelWrite((u32, u32), (u8, u8, u8), (u32, u32), String),
    /// A Request to snd EOF to Stdout
    #[cfg(feature = "context")] StdoutEof,
    /// A Request to snd EOF to Stderr
    #[cfg(feature = "context")] StderrEof,
    /// Invalid - used when deserialization goes wrong
    Invalid,
}

impl fmt::Display for ServerMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ServerMessage {}",
            match self {
                #[cfg(all(feature = "submission", feature = "metrics"))]
                ServerMessage::FlowEnd(_) => "FlowEnd".into(),
                #[cfg(all(feature = "submission", not(feature = "metrics")))]
                ServerMessage::FlowEnd => "FlowEnd".into(),
                #[cfg(feature = "submission")]ServerMessage::FlowStart => "FlowStart".into(),
                #[cfg(feature = "submission")]ServerMessage::ServerExiting(result) =>
                    format!("ServerExiting with result: {result:?}"),
                #[cfg(feature = "context")] ServerMessage::Stdout(_) => "Stdout".into(),
                #[cfg(feature = "context")] ServerMessage::Stderr(_) => "Stderr".into(),
                #[cfg(feature = "context")] ServerMessage::GetStdin => "GetStdIn".into(),
                #[cfg(feature = "context")] ServerMessage::GetLine => "GetLine".into(),
                #[cfg(feature = "context")] ServerMessage::GetArgs => "GetArgs".into(),
                #[cfg(feature = "context")] ServerMessage::Read(_) => "Read".into(),
                #[cfg(feature = "context")] ServerMessage::Write(_, _) => "Write".into(),
                #[cfg(feature = "context")] ServerMessage::PixelWrite(_, _, _, _) => "PixelWrite".into(),
                #[cfg(feature = "context")] ServerMessage::StdoutEof => "StdOutEof".into(),
                #[cfg(feature = "context")] ServerMessage::StderrEof => "StdErrEof".into(),

                ServerMessage::Invalid => "Invalid".into(),
            }
        )
    }
}

unsafe impl Send for ServerMessage {}

unsafe impl Sync for ServerMessage {}

/// A simple struct with File MetaData for passing from Client to Server - std::fs::MetaData
/// Doesn't Serialize/Deserialize etc.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct FileMetaData {
    /// Was the Path inspected a file or not
    pub is_file: bool,
    /// Was the Path inspected a directory or not
    pub is_dir: bool,
}

/// A Message from the a runtime_client to the runtime server
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientMessage {
    /// ** These messages are used to implement the `SubmissionProtocol` between the cli_runtime_server
    /// and the cli_runtime_client
    /// A submission from the client for execution
    ClientSubmission(Submission),
    /// Client requests that server enters the ddebugger at the next opportunity
    EnterDebugger,

    /// ** These messages are used to implement the context functions between the cli_runtime_client
    /// and the cli_runtime_server that runs as part of the `Coordinator`
    /// Simple acknowledgement from Client to a ServerMessage
    Ack,
    /// A String read from Stdin on Client, sent to the Server
    Stdin(String),
    /// A line of text read from Stdin using readline from Stdin on Client, sent to the Server
    Line(String),
    /// A Vector of Strings that are the flow's arguments from Client, sent to the Server
    Args(Vec<String>),
    /// An Error occurred in the runtime_client
    Error(String),
    /// EOF was detected on input reading using Stdin
    GetStdinEof,
    /// EOF was detected on input reading Stdin using Readline
    GetLineEof,
    /// Invalid - used when deserialization goes wrong
    Invalid,
    /// Contents read from a file
    FileContents(PathBuf, Vec<u8>),

    /// ** This message is just internal to the client and not sent to the server
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

unsafe impl Send for ClientMessage {}

unsafe impl Sync for ClientMessage {}

impl From<ServerMessage> for String {
    fn from(msg: ServerMessage) -> Self {
        match serde_json::to_string(&msg) {
            Ok(message_string) => message_string,
            _ => String::new(),
        }
    }
}

impl From<String> for ServerMessage {
    fn from(string: String) -> Self {
        match serde_json::from_str(&string) {
            Ok(message) => message,
            _ => ServerMessage::Invalid,
        }
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
