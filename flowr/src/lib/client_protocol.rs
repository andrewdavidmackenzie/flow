//! Unified protocol messages for client-coordinator communication.
//!
//! These types are used on the ZMQ REP/REQ connection between a client
//! (CLI, GUI, or another coordinator delegating a sub-flow) and a coordinator.
//! Both root flow submissions and sub-flow delegations use the same protocol.

use std::fmt;

use flowcore::errors::Result;
#[cfg(feature = "metrics")]
use flowcore::model::metrics::Metrics;
use flowcore::model::output_connection::OutputConnection;
use flowcore::model::submission::Submission;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;

/// Messages sent from a coordinator to a client.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum CoordinatorMessage {
    /// A flow has started executing
    FlowStart,
    /// A flow has stopped executing, with boundary outputs (empty for root flows)
    #[cfg(feature = "metrics")]
    FlowEnd(Vec<BoundaryOutputEntry>, Metrics),
    /// A flow has stopped executing, with boundary outputs (empty for root flows)
    #[cfg(not(feature = "metrics"))]
    FlowEnd(Vec<BoundaryOutputEntry>),
    /// Coordinator is exiting, with a result (OK, or Err)
    CoordinatorExiting(Result<()>),

    // --- Context function requests (sent to the client for local execution) ---
    /// A String of contents was sent to stdout
    Stdout(String),
    /// A String of contents was sent to stderr
    Stderr(String),
    /// A Request to read from Stdin
    GetStdin,
    /// A Request to read a line of characters from Stdin, with a String prompt
    GetLine(String),
    /// A Request to get the arguments for the flow
    GetArgs,
    /// A Request to read bytes from a file
    Read(String),
    /// A Request to write a series of bytes to a file
    Write(String, Vec<u8>),
    /// A Request to write a pixel to an `ImageBuffer`
    PixelWrite((u32, u32), (u8, u8, u8), (u32, u32), String),
    /// A Request to write an entire image grid (2D array of 0/1 values)
    ImageWrite(Vec<Vec<u8>>, String),
    /// EOF on Stdout
    StdoutEof,
    /// EOF on Stderr
    StderrEof,
    /// Invalid - used when deserialization goes wrong
    Invalid,
}

impl fmt::Display for CoordinatorMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "CoordinatorMessage {}",
            match self {
                #[cfg(feature = "metrics")]
                CoordinatorMessage::FlowEnd(..) => "FlowEnd",
                #[cfg(not(feature = "metrics"))]
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
                CoordinatorMessage::ImageWrite(_, _) => "ImageWrite",
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

/// Messages sent from a client to the coordinator.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientMessage {
    /// A submission for execution (root flow or sub-flow)
    ClientSubmission(Box<Submission>),
    /// Client requests that coordinator enters the debugger
    EnterDebugger,

    // --- Context function responses (from the client) ---
    /// Simple acknowledgement
    Ack,
    /// A String read from Stdin
    Stdin(String),
    /// A line of text read from Stdin using readline
    Line(String),
    /// The flow's arguments
    Args(Vec<String>),
    /// An Error occurred in the client
    Error(String),
    /// EOF on Stdin
    GetStdinEof,
    /// EOF on Readline
    GetLineEof,
    /// Invalid - used when deserialization goes wrong
    Invalid,
    /// Contents read from a file
    FileContents(String, Vec<u8>),
    /// Client is exiting
    ClientExiting(Result<()>),
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

/// A single boundary output produced by a sub-flow.
///
/// When a function inside a delegated sub-flow sends a value to a destination
/// outside the sub-flow, it becomes a boundary output. These are collected
/// and returned in `CoordinatorMessage::FlowEnd`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BoundaryOutputEntry {
    /// The output connection (`destination_id`, `destination_io_number`, etc.)
    pub connection: OutputConnection,
    /// The value produced
    pub value: Value,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod test {
    use super::*;

    #[test]
    fn coordinator_message_display() {
        assert!(format!("{}", CoordinatorMessage::FlowStart).contains("FlowStart"));
        assert!(format!("{}", CoordinatorMessage::GetArgs).contains("GetArgs"));
        assert!(format!("{}", CoordinatorMessage::Invalid).contains("Invalid"));
        assert!(format!("{}", CoordinatorMessage::Stdout("hello".into())).contains("Stdout"));
        assert!(format!("{}", CoordinatorMessage::Stderr("err".into())).contains("Stderr"));
        assert!(format!("{}", CoordinatorMessage::GetStdin).contains("GetStdIn"));
        assert!(format!("{}", CoordinatorMessage::GetLine("prompt".into())).contains("GetLine"));
        assert!(format!("{}", CoordinatorMessage::Read("file".into())).contains("Read"));
        assert!(format!("{}", CoordinatorMessage::Write("f".into(), vec![])).contains("Write"));
        assert!(format!("{}", CoordinatorMessage::StdoutEof).contains("StdOutEof"));
        assert!(format!("{}", CoordinatorMessage::StderrEof).contains("StdErrEof"));
    }

    #[test]
    fn coordinator_message_roundtrip() {
        let msg = CoordinatorMessage::Stdout("test".into());
        let json: String = msg.into();
        let back: CoordinatorMessage = json.into();
        assert!(format!("{back}").contains("Stdout"));
    }

    #[test]
    fn coordinator_message_invalid_json() {
        let back: CoordinatorMessage = "not valid json".to_string().into();
        assert!(format!("{back}").contains("Invalid"));
    }

    #[test]
    fn client_message_display() {
        assert!(format!("{}", ClientMessage::Ack).contains("Ack"));
        assert!(format!("{}", ClientMessage::Invalid).contains("Invalid"));
        assert!(format!("{}", ClientMessage::Stdin("text".into())).contains("Stdin"));
        assert!(format!("{}", ClientMessage::Line("line".into())).contains("Line"));
        assert!(format!("{}", ClientMessage::Args(vec!["a".into()])).contains("Args"));
        assert!(format!("{}", ClientMessage::Error("e".into())).contains("Error"));
        assert!(format!("{}", ClientMessage::GetStdinEof).contains("GetStdinEof"));
        assert!(format!("{}", ClientMessage::GetLineEof).contains("GetLineEof"));
        assert!(format!("{}", ClientMessage::EnterDebugger).contains("EnterDebugger"));
        assert!(
            format!("{}", ClientMessage::FileContents("f".into(), vec![])).contains("FileContents")
        );
    }

    #[test]
    fn client_message_roundtrip() {
        let msg = ClientMessage::Ack;
        let json: String = msg.into();
        let back: ClientMessage = json.into();
        assert!(format!("{back}").contains("Ack"));
    }

    #[test]
    fn client_message_invalid_json() {
        let back: ClientMessage = "garbage".to_string().into();
        assert!(format!("{back}").contains("Invalid"));
    }

    #[test]
    fn flow_end_display() {
        #[cfg(feature = "metrics")]
        {
            use flowcore::model::metrics::Metrics;
            let msg = CoordinatorMessage::FlowEnd(vec![], Metrics::new(0, 0));
            assert!(format!("{msg}").contains("FlowEnd"));
        }
        #[cfg(not(feature = "metrics"))]
        {
            let msg = CoordinatorMessage::FlowEnd(vec![]);
            assert!(format!("{msg}").contains("FlowEnd"));
        }
    }

    #[test]
    fn coordinator_exiting_display() {
        let msg = CoordinatorMessage::CoordinatorExiting(Ok(()));
        assert!(format!("{msg}").contains("CoordinatorExiting"));
    }

    #[test]
    fn client_exiting_display() {
        let msg = ClientMessage::ClientExiting(Ok(()));
        assert!(format!("{msg}").contains("ClientExiting"));
    }
}
