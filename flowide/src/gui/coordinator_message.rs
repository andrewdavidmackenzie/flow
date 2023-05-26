use std::fmt;

use serde_derive::{Deserialize, Serialize};

use flowcore::errors::*;
use flowcore::model::metrics::Metrics;

/// An Message sent from the runtime server to a runtime_client
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CoordinatorMessage {
    /// ** These messages are used to implement the `SubmissionProtocol` between the coordinator
    /// and the cli_client
    /// A flow has started executing
    FlowStart,
    /// A flow has stopped executing
    FlowEnd(Metrics),
    /// Coordinator is exiting, with a result (OK, or Err)
    CoordinatorExiting(Result<()>),

    /// ** These messages are used to implement the context functions between the cli_runtime_server
    /// that runs as part of the `Coordinator` and the cli_runtime_client that interacts with
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
    /// A Request to write a pixel to an ImageBuffer
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
                CoordinatorMessage::FlowEnd(_) => "FlowEnd".into(),
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

unsafe impl Send for CoordinatorMessage {}

unsafe impl Sync for CoordinatorMessage {}

/// A simple struct with File MetaData for passing from Client to Coordinator - std::fs::MetaData
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
        match serde_json::to_string(&msg) {
            Ok(message_string) => message_string,
            _ => String::new(),
        }
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