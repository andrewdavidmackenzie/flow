//! Structured execution trace types for replay, debugging, and formal verification.

use std::collections::{BTreeMap, BTreeSet};

use serde_derive::{Deserialize, Serialize};
use serde_json::Value;

/// Snapshot of runtime state variables at a point in execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceState {
    /// Per-process, per-input queue contents (actual values)
    pub input_q: BTreeMap<usize, BTreeMap<usize, Vec<Value>>>,
    /// Per-process, per-input count of internal values at front of queue
    pub int_count: BTreeMap<usize, BTreeMap<usize, usize>>,
    /// Reference count of busy markers per process/flow ID
    pub busy_count: BTreeMap<usize, usize>,
    /// Queue of `[func_id, job_id]` pairs ready to run
    pub ready: Vec<[usize; 2]>,
    /// Set of `[func_id, job_id]` pairs currently running
    pub running: Vec<[usize; 2]>,
    /// Set of completed process IDs
    pub done: BTreeSet<usize>,
    /// Monotonically increasing job ID counter
    pub job_counter: usize,
}

/// A single trace event: which action fired and the resulting state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    /// Name of the action (e.g. "Init", "Dispatch")
    pub action: String,
    /// Job ID that triggered this event (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<usize>,
    /// Process (function) ID associated with this event (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_id: Option<usize>,
    /// Input values the job was executed with (for Retire/Complete/Error events)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_inputs: Option<Vec<Value>>,
    /// Output value produced by the job (for Retire/Complete events)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_output: Option<Value>,
    /// State snapshot after the action
    pub state: TraceState,
}

/// A connection in the flow topology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceConn {
    /// Source process ID
    pub src: usize,
    /// Destination process ID
    pub dst: usize,
    /// Destination input index
    pub dst_input: usize,
    /// Whether this is an internal (within-flow) connection
    pub internal: bool,
}

/// The static topology of a flow graph
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TraceTopology {
    /// Set of all process (function) IDs
    pub procs: BTreeSet<usize>,
    /// Set of all flow (container) IDs
    pub flows: BTreeSet<usize>,
    /// Map from process ID to its set of input indices
    pub inputs_of: BTreeMap<usize, BTreeSet<usize>>,
    /// All connections in the flow graph
    pub conns: Vec<TraceConn>,
    /// Map from process/flow ID to parent flow ID (`None` for root)
    pub parent: BTreeMap<usize, Option<usize>>,
}

/// A complete execution trace: topology plus sequence of state transitions.
///
/// **Note:** Trace files may contain the actual data values flowing through
/// the system (input/output values of every job). When writing traces via
/// `FLOW_TRACE`, be aware the file may contain sensitive data from the flow.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Trace {
    /// The static flow topology
    pub topology: TraceTopology,
    /// Ordered sequence of trace events
    pub events: Vec<TraceEvent>,
}

impl Trace {
    /// Serialize the trace to pretty-printed JSON
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Deserialize a trace from JSON
    ///
    /// # Errors
    /// Returns an error if the JSON is malformed.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod test {
    use super::*;

    #[test]
    fn empty_trace_roundtrips() {
        let trace = Trace::default();
        let json = trace.to_json();
        let parsed = Trace::from_json(&json).unwrap();
        assert!(parsed.events.is_empty());
        assert!(parsed.topology.procs.is_empty());
    }

    #[test]
    fn trace_with_events_roundtrips() {
        let mut trace = Trace::default();
        trace.topology.procs.insert(0);
        trace.topology.procs.insert(1);
        trace.topology.flows.insert(10);
        trace.events.push(TraceEvent {
            action: "Init".to_string(),
            job_id: None,
            process_id: None,
            job_inputs: None,
            job_output: None,
            state: TraceState {
                input_q: BTreeMap::new(),
                int_count: BTreeMap::new(),
                busy_count: BTreeMap::new(),
                ready: vec![[0, 1]],
                running: vec![],
                done: BTreeSet::new(),
                job_counter: 1,
            },
        });

        let json = trace.to_json();
        let parsed = Trace::from_json(&json).unwrap();
        assert_eq!(parsed.events.len(), 1);
        assert_eq!(parsed.events[0].action, "Init");
        assert_eq!(parsed.events[0].state.ready, vec![[0, 1]]);
        assert_eq!(parsed.topology.procs.len(), 2);
    }

    #[test]
    fn state_only_event_omits_job_fields() {
        let event = TraceEvent {
            action: "Init".to_string(),
            job_id: None,
            process_id: None,
            job_inputs: None,
            job_output: None,
            state: TraceState {
                input_q: BTreeMap::new(),
                int_count: BTreeMap::new(),
                busy_count: BTreeMap::new(),
                ready: vec![],
                running: vec![],
                done: BTreeSet::new(),
                job_counter: 0,
            },
        };
        let json = serde_json::to_string(&event).unwrap();
        // None fields should be omitted, not serialized as null
        assert!(!json.contains("job_id"), "job_id should be omitted");
        assert!(!json.contains("process_id"), "process_id should be omitted");
        assert!(!json.contains("job_inputs"), "job_inputs should be omitted");
        assert!(!json.contains("job_output"), "job_output should be omitted");
    }

    #[test]
    fn job_event_roundtrips_with_values() {
        use serde_json::json;

        let event = TraceEvent {
            action: "RetireAndSend".to_string(),
            job_id: Some(42),
            process_id: Some(3),
            job_inputs: Some(vec![json!(1), json!([0, 1, 0])]),
            job_output: Some(json!({"chunk": [1, 0, 1], "partial": []})),
            state: TraceState {
                input_q: BTreeMap::new(),
                int_count: BTreeMap::new(),
                busy_count: BTreeMap::new(),
                ready: vec![],
                running: vec![],
                done: BTreeSet::new(),
                job_counter: 42,
            },
        };
        let json = serde_json::to_string_pretty(&event).unwrap();
        let parsed: TraceEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.job_id, Some(42));
        assert_eq!(parsed.process_id, Some(3));
        assert_eq!(parsed.job_inputs.as_ref().unwrap().len(), 2);
        assert!(parsed.job_output.is_some());
    }

    #[test]
    fn job_output_with_real_value_roundtrips() {
        use serde_json::json;

        let event = TraceEvent {
            action: "RetireAndSend".to_string(),
            job_id: Some(1),
            process_id: Some(0),
            job_inputs: Some(vec![json!(1)]),
            job_output: Some(json!(42)),
            state: TraceState {
                input_q: BTreeMap::new(),
                int_count: BTreeMap::new(),
                busy_count: BTreeMap::new(),
                ready: vec![],
                running: vec![],
                done: BTreeSet::new(),
                job_counter: 1,
            },
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("job_output"), "Some(42) should be serialized");
        let parsed: TraceEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.job_output, Some(json!(42)));
    }
}
