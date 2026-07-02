//! Reads a JSON execution trace and generates a TLA+ trace spec for TLC verification.
//!
//! Usage: flowr-tla-check <trace.json> [output-dir]
//!
//! Generates a `.tla` trace spec and `.cfg` file that TLC can check against
//! `FlowRuntimeBase.tla` invariants.

#![allow(clippy::indexing_slicing)]

use std::fmt::Write;
use std::fs;
use std::path::Path;

use flowcore::model::trace::{Trace, TraceState, TraceTopology};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: flowr-tla-check <trace.json> [output-dir]");
        std::process::exit(1);
    }

    let trace_path = args.get(1).expect("trace path argument required");
    let output_dir = args.get(2).map_or(".", String::as_str);

    let json = fs::read_to_string(trace_path).unwrap_or_else(|e| {
        eprintln!("Cannot read {trace_path}: {e}");
        std::process::exit(1);
    });

    let trace = Trace::from_json(&json).unwrap_or_else(|e| {
        eprintln!("Cannot parse trace JSON: {e}");
        std::process::exit(1);
    });

    if trace.events.is_empty() {
        eprintln!("Trace has no events");
        std::process::exit(1);
    }

    let tla = generate_trace_tla(&trace);
    let cfg = generate_trace_cfg();

    let tla_path = Path::new(output_dir).join("TraceCheck.tla");
    let cfg_path = Path::new(output_dir).join("TraceCheck.cfg");

    fs::write(&tla_path, &tla).unwrap_or_else(|e| {
        eprintln!("Cannot write {}: {e}", tla_path.display());
        std::process::exit(1);
    });
    fs::write(&cfg_path, &cfg).unwrap_or_else(|e| {
        eprintln!("Cannot write {}: {e}", cfg_path.display());
        std::process::exit(1);
    });

    eprintln!("Generated: {}", tla_path.display());
    eprintln!("Generated: {}", cfg_path.display());
    eprintln!(
        "Trace has {} events across {} procs, {} flows",
        trace.events.len(),
        trace.topology.procs.len(),
        trace.topology.flows.len()
    );
}

fn format_topology(topo: &TraceTopology) -> (String, String, String, String, String, String) {
    let procs = topo
        .procs
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let flows = topo
        .flows
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");

    let inputs_of: Vec<String> = topo
        .inputs_of
        .iter()
        .map(|(p, inputs)| {
            let indices: Vec<String> = inputs.iter().map(ToString::to_string).collect();
            format!("{p} :> {{{}}}", indices.join(", "))
        })
        .collect();

    let conns: Vec<String> = topo
        .conns
        .iter()
        .map(|c| {
            format!(
                "[src |-> {}, dst |-> {}, dstInput |-> {}, internal |-> {}]",
                c.src,
                c.dst,
                c.dst_input,
                if c.internal { "TRUE" } else { "FALSE" }
            )
        })
        .collect();

    let parent: Vec<String> = topo
        .parent
        .iter()
        .map(|(id, pid)| match pid {
            Some(p) => format!("{id} :> {p}"),
            None => format!("{id} :> NoParent"),
        })
        .collect();

    let init_stubs: Vec<String> = topo
        .procs
        .iter()
        .map(|p| {
            let inputs = topo.inputs_of.get(p).cloned().unwrap_or_default();
            if inputs.is_empty() {
                format!("{p} :> <<>>")
            } else {
                let parts: Vec<String> = inputs.iter().map(|i| format!("{i} :> NoInit")).collect();
                format!("{p} :> ({})", parts.join(" @@ "))
            }
        })
        .collect();

    (
        procs,
        flows,
        inputs_of.join(" @@ "),
        conns.join(",\n                  "),
        parent.join(" @@ "),
        init_stubs.join(" @@ "),
    )
}

fn generate_trace_tla(trace: &Trace) -> String {
    let topo = &trace.topology;
    let (procs, flows, inputs_of, conns, parent, init_stubs) = format_topology(topo);
    let n = trace.events.len();

    let mut trace_states = String::new();
    for (i, event) in trace.events.iter().enumerate() {
        let _ = writeln!(
            trace_states,
            "TraceState{i} ==\n{}\n",
            state_to_tla(&event.state, topo)
        );
    }

    let mut trace_next_cases = Vec::new();
    for i in 0..n - 1 {
        trace_next_cases.push(format!(
            "    \\/ /\\ traceIdx = {i}\n       /\\ traceIdx' = {next}\n{primed}",
            next = i + 1,
            primed = state_to_tla_primed(&trace.events[i + 1].state, topo)
        ));
    }
    let trace_next = if trace_next_cases.is_empty() {
        "    FALSE".to_string()
    } else {
        trace_next_cases.join("\n")
    };

    format!(
        "\
--------------------------- MODULE TraceCheck ---------------------------
(* Auto-generated trace spec for TLC verification. *)

EXTENDS Integers, Sequences, FiniteSets, TLC

NoParent == -1
NoInit == -2

VARIABLES inputQ, intCount, busyCount, ready, running, done, jobCounter, traceIdx

FR == INSTANCE FlowRuntimeBase WITH
    Procs <- {{{procs}}},
    Flows <- {{{flows}}},
    InputsOf <- {inputs_of},
    Conns <- {{{conns}}},
    Parent <- {parent},
    InitOnce <- {init_stubs},
    InitAlways <- {init_stubs},
    FlowInitOnce <- {init_stubs},
    FlowInitAlways <- {init_stubs},
    NoParent <- NoParent,
    NoInit <- NoInit,
    inputQ <- inputQ,
    intCount <- intCount,
    busyCount <- busyCount,
    ready <- ready,
    running <- running,
    done <- done,
    jobCounter <- jobCounter

---------------------------------------------------------------------------
(* Trace states — used by TraceInit *)

{trace_states}\
---------------------------------------------------------------------------
(* Trace specification *)

TraceInit == TraceState0 /\\ traceIdx = 0

TraceNext ==
{trace_next}

TraceSpec == TraceInit /\\ [][TraceNext]_<<inputQ, intCount, busyCount, ready, running, done, jobCounter, traceIdx>>

---------------------------------------------------------------------------
(* Invariants to check at each recorded state *)

TypeOK == FR!TypeOK
InternalCountBound == FR!InternalCountBound
AncestorConsistency == FR!AncestorConsistency

==========================================================================
",
    )
}

fn state_to_tla(state: &TraceState, topo: &TraceTopology) -> String {
    format_state_lines(state, topo, false)
}

fn state_to_tla_primed(state: &TraceState, topo: &TraceTopology) -> String {
    format_state_lines(state, topo, true)
}

fn format_state_lines(state: &TraceState, topo: &TraceTopology, primed: bool) -> String {
    let p = if primed { "'" } else { "" };
    let mut lines = Vec::new();

    lines.push(format!(
        "       /\\ inputQ{p} = {}",
        format_input_queues(state, topo)
    ));
    lines.push(format!(
        "       /\\ intCount{p} = {}",
        format_int_counts(state, topo)
    ));
    lines.push(format_busy_count_with(state, p));
    lines.push(format_ready_with(state, p));
    lines.push(format_running_with(state, p));
    lines.push(format_done_with(state, p));
    lines.push(format!("       /\\ jobCounter{p} = {}", state.job_counter));

    lines.join("\n")
}

fn format_input_queues(state: &TraceState, topo: &TraceTopology) -> String {
    let parts: Vec<String> = topo
        .procs
        .iter()
        .map(|p| {
            let inputs = topo.inputs_of.get(p).cloned().unwrap_or_default();
            if inputs.is_empty() {
                format!("{p} :> <<>>")
            } else {
                let input_parts: Vec<String> = inputs
                    .iter()
                    .map(|i| {
                        let q = state
                            .input_q
                            .get(p)
                            .and_then(|m| m.get(i))
                            .cloned()
                            .unwrap_or_default();
                        let seq = if q.is_empty() {
                            "<<>>".to_string()
                        } else {
                            format!(
                                "<<{}>>",
                                q.iter()
                                    .map(ToString::to_string)
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        };
                        format!("{i} :> {seq}")
                    })
                    .collect();
                format!("{p} :> ({})", input_parts.join(" @@ "))
            }
        })
        .collect();
    parts.join(" @@ ")
}

fn format_int_counts(state: &TraceState, topo: &TraceTopology) -> String {
    let parts: Vec<String> = topo
        .procs
        .iter()
        .map(|p| {
            let inputs = topo.inputs_of.get(p).cloned().unwrap_or_default();
            if inputs.is_empty() {
                format!("{p} :> <<>>")
            } else {
                let input_parts: Vec<String> = inputs
                    .iter()
                    .map(|i| {
                        let c = state
                            .int_count
                            .get(p)
                            .and_then(|m| m.get(i))
                            .copied()
                            .unwrap_or(0);
                        format!("{i} :> {c}")
                    })
                    .collect();
                format!("{p} :> ({})", input_parts.join(" @@ "))
            }
        })
        .collect();
    parts.join(" @@ ")
}

fn format_busy_count_with(state: &TraceState, prime: &str) -> String {
    if state.busy_count.is_empty() {
        format!("       /\\ busyCount{prime} = [id \\in {{}} |-> 0]")
    } else {
        let bc: Vec<String> = state
            .busy_count
            .iter()
            .map(|(k, v)| format!("{k} :> {v}"))
            .collect();
        format!("       /\\ busyCount{prime} = {}", bc.join(" @@ "))
    }
}

fn format_ready_with(state: &TraceState, prime: &str) -> String {
    if state.ready.is_empty() {
        format!("       /\\ ready{prime} = <<>>")
    } else {
        let parts: Vec<String> = state
            .ready
            .iter()
            .map(|j| format!("[func |-> {}, jobId |-> {}]", j[0], j[1]))
            .collect();
        format!("       /\\ ready{prime} = <<{}>>", parts.join(", "))
    }
}

fn format_running_with(state: &TraceState, prime: &str) -> String {
    if state.running.is_empty() {
        format!("       /\\ running{prime} = {{}}")
    } else {
        let parts: Vec<String> = state
            .running
            .iter()
            .map(|j| format!("[func |-> {}, jobId |-> {}]", j[0], j[1]))
            .collect();
        format!("       /\\ running{prime} = {{{}}}", parts.join(", "))
    }
}

fn format_done_with(state: &TraceState, prime: &str) -> String {
    if state.done.is_empty() {
        format!("       /\\ done{prime} = {{}}")
    } else {
        let parts: Vec<String> = state.done.iter().map(ToString::to_string).collect();
        format!("       /\\ done{prime} = {{{}}}", parts.join(", "))
    }
}

fn generate_trace_cfg() -> String {
    "\
SPECIFICATION TraceSpec

INVARIANTS
    TypeOK
    InternalCountBound
    AncestorConsistency
"
    .to_string()
}
