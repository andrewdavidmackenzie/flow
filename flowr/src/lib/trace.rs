//! Runtime trace capture — builds `Trace` from live `RunState` transitions.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use flowcore::model::flow_manifest::FlowManifest;
use flowcore::model::submission::Submission;
use flowcore::model::trace::{Trace, TraceConn, TraceEvent, TraceState, TraceTopology};

use crate::job::Job;

/// Extract the static topology from a submission (called once at construction)
pub(crate) fn topology_from_submission(submission: &Submission) -> Trace {
    let manifest = &submission.manifest;
    let mut procs = BTreeSet::new();
    let mut flows = BTreeSet::new();
    let mut inputs_of = BTreeMap::new();
    let mut conns = Vec::new();
    let mut parent = BTreeMap::new();

    for (id, function) in manifest.functions() {
        procs.insert(*id);
        let input_indices: BTreeSet<usize> = (0..function.inputs().len()).collect();
        inputs_of.insert(*id, input_indices);
        parent.insert(*id, Some(function.get_parent_id()));

        for oc in function.get_output_connections() {
            conns.push(TraceConn {
                src: *id,
                dst: oc.destination_id,
                dst_input: oc.destination_io_number,
                internal: oc.internal,
            });
        }
    }

    for (id, flow_info) in manifest.flows() {
        flows.insert(*id);
        parent.insert(*id, flow_info.parent_id);
    }

    Trace {
        topology: TraceTopology {
            procs,
            flows,
            inputs_of,
            conns,
            parent,
        },
        events: Vec::new(),
    }
}

/// Snapshot the current runtime state and append a trace event.
///
/// Takes individual fields to avoid borrowing the entire `RunState`
/// (which would conflict with the mutable borrow of the `trace` field).
#[allow(clippy::too_many_arguments)]
pub(crate) fn record_event(
    trace: &mut Trace,
    action: &str,
    manifest: &FlowManifest,
    busy_count: &HashMap<usize, usize>,
    ready_jobs: &VecDeque<Job>,
    running_jobs: &HashMap<usize, Job>,
    completed: &HashSet<usize>,
    number_of_jobs_created: usize,
) {
    let state = capture_state(
        manifest,
        busy_count,
        ready_jobs,
        running_jobs,
        completed,
        number_of_jobs_created,
    );
    trace.events.push(TraceEvent {
        action: action.to_string(),
        state,
    });
}

fn capture_state(
    manifest: &FlowManifest,
    busy_count: &HashMap<usize, usize>,
    ready_jobs: &VecDeque<Job>,
    running_jobs: &HashMap<usize, Job>,
    completed: &HashSet<usize>,
    number_of_jobs_created: usize,
) -> TraceState {
    let mut input_q = BTreeMap::new();
    let mut int_count = BTreeMap::new();

    for (id, function) in manifest.functions() {
        let mut q_map = BTreeMap::new();
        let mut ic_map = BTreeMap::new();
        for (idx, input) in function.inputs().iter().enumerate() {
            q_map.insert(idx, vec![1i64; input.values_available()]);
            ic_map.insert(idx, input.internal_count());
        }
        input_q.insert(*id, q_map);
        int_count.insert(*id, ic_map);
    }

    let bc: BTreeMap<usize, usize> = busy_count.iter().map(|(&k, &v)| (k, v)).collect();

    let ready: Vec<[usize; 2]> = ready_jobs
        .iter()
        .map(|j| [j.process_id, j.payload.job_id])
        .collect();

    let running: Vec<[usize; 2]> = running_jobs
        .values()
        .map(|j| [j.process_id, j.payload.job_id])
        .collect();

    let done: BTreeSet<usize> = completed.iter().copied().collect();

    TraceState {
        input_q,
        int_count,
        busy_count: bc,
        ready,
        running,
        done,
        job_counter: number_of_jobs_created,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod test {
    use serde_json::json;

    use flowcore::model::flow_manifest::{FlowInfo, FlowManifest};
    use flowcore::model::input::Input;
    use flowcore::model::input::InputInitializer::Once;
    use flowcore::model::metadata::MetaData;
    use flowcore::model::output_connection::{OutputConnection, Source};
    use flowcore::model::runtime_function::RuntimeFunction;
    use flowcore::model::submission::Submission;

    use crate::run_state::RunState;

    fn test_meta_data() -> MetaData {
        MetaData {
            name: "trace_test".into(),
            version: "0.0.0".into(),
            description: "a trace test".into(),
            authors: vec!["test".into()],
        }
    }

    fn test_manifest(functions: Vec<RuntimeFunction>) -> FlowManifest {
        let mut manifest = FlowManifest::new(test_meta_data());
        for function in functions {
            manifest.add_function(function);
        }
        manifest.add_flow_info(FlowInfo {
            process_id: 0,
            parent_id: None,
            sub_flow_ids: vec![],
            #[cfg(feature = "debugger")]
            name: "root".to_string(),
            #[cfg(feature = "debugger")]
            route: "/".to_string(),
        });
        manifest
    }

    fn test_submission(functions: Vec<RuntimeFunction>) -> Submission {
        Submission::new(
            test_manifest(functions),
            None,
            None,
            #[cfg(feature = "debugger")]
            true,
        )
    }

    fn func_a_sends_to_b() -> RuntimeFunction {
        let conn = OutputConnection::new(
            Source::default(),
            1,
            0,
            0,
            true,
            "/fB".to_string(),
            #[cfg(feature = "debugger")]
            String::default(),
        );
        RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "fA",
            #[cfg(feature = "debugger")]
            "/fA",
            "file://fake/test",
            vec![Input::new(
                #[cfg(feature = "debugger")]
                "",
                0,
                false,
                Some(Once(json!(1))),
                None,
            )],
            0,
            0,
            &[conn],
            false,
        )
    }

    fn func_b_no_init() -> RuntimeFunction {
        RuntimeFunction::new(
            #[cfg(feature = "debugger")]
            "fB",
            #[cfg(feature = "debugger")]
            "/fB",
            "file://fake/test",
            vec![Input::new(
                #[cfg(feature = "debugger")]
                "",
                0,
                false,
                None,
                None,
            )],
            1,
            0,
            &[],
            false,
        )
    }

    #[test]
    fn topology_extracted_from_submission() {
        let submission = test_submission(vec![func_a_sends_to_b(), func_b_no_init()]);
        let trace = super::topology_from_submission(&submission);

        assert!(trace.topology.procs.contains(&0));
        assert!(trace.topology.procs.contains(&1));
        assert!(trace.topology.flows.contains(&0));
        assert_eq!(trace.topology.conns.len(), 1);
        assert_eq!(trace.topology.conns[0].src, 0);
        assert_eq!(trace.topology.conns[0].dst, 1);
        assert!(trace.events.is_empty());
    }

    #[test]
    fn init_records_trace_event() {
        let mut state = RunState::new(test_submission(vec![func_a_sends_to_b(), func_b_no_init()]));
        state.init().expect("init failed");

        let trace = state.take_trace();
        assert!(
            trace.events.iter().any(|e| e.action == "Init"),
            "Expected an Init event in trace"
        );
    }

    #[test]
    fn create_job_records_trace_event() {
        let mut state = RunState::new(test_submission(vec![func_a_sends_to_b(), func_b_no_init()]));
        state.init().expect("init failed");

        let trace = state.take_trace();
        assert!(
            trace.events.iter().any(|e| e.action == "CreateJob"),
            "Expected a CreateJob event in trace"
        );
    }

    #[test]
    fn dispatch_records_trace_event() {
        let mut state = RunState::new(test_submission(vec![func_a_sends_to_b(), func_b_no_init()]));
        state.init().expect("init failed");

        let job = state.get_next_job().expect("expected a ready job");
        state.start_job(job);

        let trace = state.take_trace();
        assert!(
            trace.events.iter().any(|e| e.action == "Dispatch"),
            "Expected a Dispatch event in trace"
        );
    }

    #[test]
    fn trace_serializes_to_valid_json() {
        let mut state = RunState::new(test_submission(vec![func_a_sends_to_b(), func_b_no_init()]));
        state.init().expect("init failed");

        let trace = state.take_trace();
        let json = trace.to_json();
        assert!(!json.is_empty());

        let parsed: flowcore::model::trace::Trace =
            serde_json::from_str(&json).expect("trace JSON should be valid");
        assert_eq!(parsed.events.len(), trace.events.len());
    }

    #[test]
    fn init_state_has_correct_queue_lengths() {
        let mut state = RunState::new(test_submission(vec![func_a_sends_to_b(), func_b_no_init()]));
        state.init().expect("init failed");

        let trace = state.take_trace();
        let init_event = trace
            .events
            .iter()
            .find(|e| e.action == "Init")
            .expect("Init event");

        // After init, func_a's input was consumed (job created), func_b has no values
        let func_b_q = &init_event.state.input_q[&1][&0];
        assert!(
            func_b_q.is_empty(),
            "func_b should have empty queue after init"
        );
    }
}
