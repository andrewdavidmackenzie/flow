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
    /// A flow has started executing
    FlowStart,
    /// A flow has stopped executing
    #[cfg(feature = "metrics")]
    FlowEnd(Metrics),
    /// A flow has stopped executing
    #[cfg(not(feature = "metrics"))]
    FlowEnd,
    /// Server is exiting, with a result (OK, or Err)
    ServerExiting(Result<()>),
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
    /// A Request to read bytes from a file
    Read(PathBuf),
    /// Request File Metadata on a file on the client
    GetFileMetaData(PathBuf),
    /// A Request to write a series of bytes to a file
    Write(String, Vec<u8>),
    /// A Request to write a pixel to an ImageBuffer
    PixelWrite((u32, u32), (u8, u8, u8), (u32, u32), String),
    /// A Request to snd EOF to Stdout
    StdoutEof,
    /// A Request to snd EOF to Stderr
    StderrEof,
    /// Invalid - used when deserialization goes wrong
    Invalid,
}

impl fmt::Display for ServerMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ServerMessage {}",
            match self {
                #[cfg(not(feature = "metrics"))]
                ServerMessage::FlowEnd => "FlowEnd".into(),
                #[cfg(feature = "metrics")]
                ServerMessage::FlowEnd(_) => "FlowEnd".into(),
                ServerMessage::FlowStart => "FlowStart".into(),
                ServerMessage::ServerExiting(result) =>
                    format!("ServerExiting with result: {:?}", result),
                ServerMessage::Stdout(_) => "Stdout".into(),
                ServerMessage::Stderr(_) => "Stderr".into(),
                ServerMessage::GetStdin => "GetStdIn".into(),
                ServerMessage::GetLine => "GetLine".into(),
                ServerMessage::GetArgs => "GetArgs".into(),
                ServerMessage::Read(_) => "Read".into(),
                ServerMessage::GetFileMetaData(_) => "GetFileMetaData".into(),
                ServerMessage::Write(_, _) => "Write".into(),
                ServerMessage::PixelWrite(_, _, _, _) => "PixelWrite".into(),
                ServerMessage::StdoutEof => "StdOutEof".into(),
                ServerMessage::StderrEof => "StdErrEof".into(),
                ServerMessage::Invalid => "Invalid".into(),
            }
        )
    }
}

unsafe impl Send for ServerMessage {}

unsafe impl Sync for ServerMessage {}

/// A simple struct with File MetaData for passing from Client to Server - std::fs::MetaData
/// Doesn't Serialize/Deserialize etc.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct FileMetaData {
    /// Was the Path inspected a file or not
    pub is_file: bool,
    /// Was the Path inspected a directory or not
    pub is_dir: bool,
}

/// A Message from the a runtime_client to the runtime server
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientMessage {
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
    GetStdinEof,
    /// EOF was detected on input reading Stdin using Readline
    GetLineEof,
    /// Client is exiting Event loop
    ClientExiting(Result<()>),
    /// A submission from the client for execution
    ClientSubmission(Submission),
    /// Client requests that server enters the ddebugger at the next opportunity
    EnterDebugger,
    /// Invalid - used when deserialization goes wrong
    Invalid,
    /// Contents read from a file
    FileContents(PathBuf, Vec<u8>),
    /// MetaData for a file on the client
    FileMetaDate(PathBuf, FileMetaData),
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
                    format!("ClientExiting with server result: {:?}", result),
                ClientMessage::ClientSubmission(_) => "ClientSubmission".into(),
                ClientMessage::EnterDebugger => "EnterDebugger".into(),
                ClientMessage::Invalid => "Invalid".into(),
                ClientMessage::FileContents(_, _) => "FileContents".into(),
                ClientMessage::FileMetaDate(_, _) => "FileMetaDate".into(),
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
    fn from(msg: String) -> Self {
        match serde_json::from_str(&msg) {
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
    fn from(msg: String) -> Self {
        match serde_json::from_str(&msg) {
            Ok(message) => message,
            _ => ClientMessage::Invalid,
        }
    }
}
