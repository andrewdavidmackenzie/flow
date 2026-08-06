//! Protocol messages for peer coordinator communication.
//!
//! When a parent coordinator delegates a sub-flow to a peer coordinator,
//! these messages are exchanged over ZMQ.

use flowcore::model::flow_manifest::FlowManifest;
use flowcore::model::output_connection::OutputConnection;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;

/// Messages sent from parent coordinator to peer coordinator.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum PeerRequest {
    /// Submit a sub-flow for execution with initial input values.
    /// The peer coordinator will create a `RunState` and execute the flow.
    Submit(Box<SubflowSubmission>),

    /// Signal that the parent flow is complete and the peer should clean up.
    Done,
}

/// A sub-flow submission from a parent coordinator.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SubflowSubmission {
    /// The sub-flow manifest to execute
    pub manifest: FlowManifest,
    /// Initial input values mapped to boundary function inputs.
    /// Each entry is (`destination_function_id`, `destination_io_number`, value).
    pub inputs: Vec<(usize, usize, Value)>,
}

/// Messages sent from peer coordinator back to parent coordinator.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum PeerResponse {
    /// A value produced at a boundary connection, destined for a function
    /// in the parent flow.
    BoundaryOutput {
        /// The output connection that produced this value (contains
        /// `destination_id`, `destination_io_number`, etc.)
        connection: OutputConnection,
        /// The value produced
        value: Value,
    },

    /// The sub-flow has processed all current inputs and is idle.
    /// The parent can send more inputs or signal Done.
    Idle,

    /// An error occurred during sub-flow execution.
    Error(String),

    /// Acknowledgement that the peer received the Done signal.
    DoneAck,
}
