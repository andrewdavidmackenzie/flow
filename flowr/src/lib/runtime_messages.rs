use std::fmt;

use serde_derive::{Deserialize, Serialize};
#[cfg(feature = "distributed")]
use zmq::Message;

use crate::coordinator::Submission;
#[cfg(feature = "metrics")]
use crate::metrics::Metrics;

/// An Message sent from the runtime server to a runtime_client
#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMessage {
    /// A flow has started executing
    FlowStart,
    /// A flow has stopped executing
    #[cfg(feature = "metrics")]
    FlowEnd(Metrics),
    #[cfg(not(feature = "metrics"))]
    FlowEnd,
    /// Server is exiting
    ServerExiting,
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
    Read(String),
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
                ServerMessage::FlowStart => "FlowStart",
                #[cfg(feature = "metrics")]
                ServerMessage::FlowEnd(_) => "FlowEnd",
                #[cfg(not(feature = "metrics"))]
                ServerMessage::FlowEnd => "FlowEnd",
                ServerMessage::ServerExiting => "ServerExiting",
                ServerMessage::Stdout(_) => "Stdout",
                ServerMessage::Stderr(_) => "Stderr",
                ServerMessage::GetStdin => "GetStdIn",
                ServerMessage::GetLine => "GetLine",
                ServerMessage::GetArgs => "GetArgs",
                ServerMessage::Read(_) => "Read",
                ServerMessage::Write(_, _) => "Write",
                ServerMessage::PixelWrite(_, _, _, _) => "PixelWrite",
                ServerMessage::StdoutEof => "StdOutEof",
                ServerMessage::StderrEof => "StdErrEof",
                ServerMessage::Invalid => "Invalid",
            }
        )
    }
}

unsafe impl Send for ServerMessage {}

unsafe impl Sync for ServerMessage {}

/// A Message from the a runtime_client to the runtime server
#[derive(Serialize, Deserialize, PartialEq, Debug)]
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
    ClientExiting,
    /// A submission from the client for execution
    ClientSubmission(Submission),
    /// Client requests that server enters the ddebugger at the next opportunity
    EnterDebugger,
    /// Invalid - used when deserialization goes wrong
    Invalid,
    /// Contents read from a file
    FileContents(Vec<u8>),
}

impl fmt::Display for ClientMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ClientMessage {}",
            match self {
                ClientMessage::Ack => "Ack",
                ClientMessage::Stdin(_) => "Stdin",
                ClientMessage::Line(_) => "Line",
                ClientMessage::Args(_) => "Args",
                ClientMessage::Error(_) => "Error",
                ClientMessage::GetStdinEof => "GetStdinEof",
                ClientMessage::GetLineEof => "GetLineEof",
                ClientMessage::ClientExiting => "ClientExiting",
                ClientMessage::ClientSubmission(_) => "ClientSubmission",
                ClientMessage::EnterDebugger => "EnterDebugger",
                ClientMessage::Invalid => "Invalid",
                ClientMessage::FileContents(_) => "FileContents",
            }
        )
    }
}

unsafe impl Send for ClientMessage {}

unsafe impl Sync for ClientMessage {}

#[cfg(feature = "distributed")]
impl From<ServerMessage> for Message {
    fn from(event: ServerMessage) -> Self {
        match serde_json::to_string(&event) {
            Ok(message_string) => Message::from(&message_string),
            _ => Message::new(),
        }
    }
}

#[cfg(feature = "distributed")]
impl From<Message> for ServerMessage {
    fn from(msg: Message) -> Self {
        match msg.as_str() {
            Some(message_string) => match serde_json::from_str(message_string) {
                Ok(message) => message,
                _ => ServerMessage::Invalid,
            },
            _ => ServerMessage::Invalid,
        }
    }
}

#[cfg(feature = "distributed")]
impl From<ClientMessage> for Message {
    fn from(msg: ClientMessage) -> Self {
        match serde_json::to_string(&msg) {
            Ok(message_string) => Message::from(&message_string),
            _ => Message::new(),
        }
    }
}

#[cfg(feature = "distributed")]
impl From<Message> for ClientMessage {
    fn from(msg: Message) -> Self {
        match msg.as_str() {
            Some(message_string) => match serde_json::from_str(message_string) {
                Ok(message) => message,
                _ => ClientMessage::Invalid,
            },
            _ => ClientMessage::Invalid,
        }
    }
}
