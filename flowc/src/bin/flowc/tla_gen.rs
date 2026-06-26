//! Generate TLA+ scenario files from compiled flow manifests.
//!
//! Reads a `manifest.json` and produces a `.tla` file that INSTANCEs
//! `FlowRuntimeBase` with the concrete topology from the manifest.

use std::fs;
use std::path::Path;

use log::info;
use serde_json::Value;

/// Generate a TLA+ scenario file from a compiled manifest.
///
/// # Errors
/// Returns an error if the manifest cannot be read or parsed.
pub fn generate_tla(manifest_path: &Path) -> Result<(), String> {
    let manifest_str =
        fs::read_to_string(manifest_path).map_err(|e| format!("Cannot read manifest: {e}"))?;
    let manifest: Value =
        serde_json::from_str(&manifest_str).map_err(|e| format!("Cannot parse manifest: {e}"))?;

    let functions = manifest
        .get("functions")
        .and_then(Value::as_object)
        .ok_or("No functions in manifest")?;
    let flows = manifest
        .get("flows")
        .and_then(Value::as_object)
        .ok_or("No flows in manifest")?;

    let mut fd = extract_functions(functions)?;
    let flow_ids = extract_flows(flows, &mut fd.parent_entries);

    let module_name = derive_module_name(&manifest, manifest_path);
    let tla = format_tla(
        &module_name,
        &fd.procs,
        &flow_ids,
        &fd.inputs_of,
        &fd.conns,
        &fd.parent_entries,
        &fd.init_once,
        &fd.init_always,
    );

    let dir = manifest_path
        .parent()
        .ok_or("Cannot get manifest directory")?;
    let tla_path = dir.join(format!("{module_name}.tla"));
    let cfg_path = dir.join(format!("{module_name}.cfg"));

    fs::write(&tla_path, tla).map_err(|e| format!("Cannot write .tla: {e}"))?;
    fs::write(
        &cfg_path,
        "SPECIFICATION Spec\n\nINVARIANTS\n    TypeOK\n    CompletedNeverRuns\n    InternalCountBound\n    AncestorConsistency\n",
    )
    .map_err(|e| format!("Cannot write .cfg: {e}"))?;

    copy_base_tla(dir)?;

    info!("Generated TLA+ spec: {}", tla_path.display());
    info!("Generated TLA+ config: {}", cfg_path.display());
    Ok(())
}

struct FuncData {
    procs: Vec<String>,
    inputs_of: Vec<String>,
    conns: Vec<String>,
    parent_entries: Vec<String>,
    init_once: Vec<String>,
    init_always: Vec<String>,
}

fn extract_functions(functions: &serde_json::Map<String, Value>) -> Result<FuncData, String> {
    let mut procs = Vec::new();
    let mut inputs_of = Vec::new();
    let mut conns = Vec::new();
    let mut parent_entries = Vec::new();
    let mut init_once = Vec::new();
    let mut init_always = Vec::new();

    for (fid_str, func) in functions {
        let fid = fid_str.as_str();
        let parent_id = func
            .get("parent_id")
            .and_then(Value::as_u64)
            .ok_or("Missing parent_id")?;
        procs.push(fid.to_string());
        parent_entries.push(format!("{fid} :> {parent_id}"));

        let inputs = func
            .get("inputs")
            .and_then(Value::as_array)
            .ok_or("Missing inputs")?;
        let indices: Vec<String> = (0..inputs.len()).map(|i| i.to_string()).collect();
        inputs_of.push(format!("{fid} :> {{{}}}", indices.join(", ")));

        let mut once_parts = Vec::new();
        let mut always_parts = Vec::new();
        for (idx, input) in inputs.iter().enumerate() {
            let once_val = extract_initializer(input, "initializer", "once")
                .or_else(|| extract_initializer(input, "flow_initializer", "once"));
            let always_val = extract_initializer(input, "initializer", "always")
                .or_else(|| extract_initializer(input, "flow_initializer", "always"));
            once_parts.push(format!(
                "{idx} :> {}",
                once_val.as_deref().unwrap_or("NoInit")
            ));
            always_parts.push(format!(
                "{idx} :> {}",
                always_val.as_deref().unwrap_or("NoInit")
            ));
        }
        init_once.push(format!("{fid} :> ({})", once_parts.join(" @@ ")));
        init_always.push(format!("{fid} :> ({})", always_parts.join(" @@ ")));

        if let Some(ocs) = func.get("output_connections").and_then(Value::as_array) {
            for oc in ocs {
                let dst = oc
                    .get("destination_id")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                let dst_input = oc
                    .get("destination_io_number")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                let internal = oc.get("internal").and_then(Value::as_bool).unwrap_or(false);
                conns.push(format!(
                    "[src |-> {fid}, dst |-> {dst}, dstInput |-> {dst_input}, internal |-> {}]",
                    if internal { "TRUE" } else { "FALSE" }
                ));
            }
        }
    }

    Ok(FuncData {
        procs,
        inputs_of,
        conns,
        parent_entries,
        init_once,
        init_always,
    })
}

fn extract_flows(
    flows: &serde_json::Map<String, Value>,
    parent_entries: &mut Vec<String>,
) -> Vec<String> {
    let mut flow_ids = Vec::new();
    for (fid_str, flow) in flows {
        flow_ids.push(fid_str.clone());
        let parent_id = flow.get("parent_id").and_then(Value::as_u64);
        parent_entries.push(format!(
            "{fid_str} :> {}",
            parent_id.map_or_else(|| "NoParent".to_string(), |p| p.to_string())
        ));
    }
    flow_ids
}

fn derive_module_name(manifest: &Value, manifest_path: &Path) -> String {
    let name = manifest
        .get("metadata")
        .and_then(|m| m.get("name"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            manifest_path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
        })
        .unwrap_or("flow");
    sanitize_module_name(name)
}

#[allow(clippy::too_many_arguments)]
fn format_tla(
    module_name: &str,
    procs: &[String],
    flow_ids: &[String],
    inputs_of: &[String],
    conns: &[String],
    parent_entries: &[String],
    init_once: &[String],
    init_always: &[String],
) -> String {
    let procs_str = procs.join(", ");
    let flows_str = flow_ids.join(", ");
    format!(
        "\
--------------------------- MODULE {module_name} ---------------------------
(* Auto-generated from {module_name} manifest. Do not edit. *)

EXTENDS Integers, Sequences, FiniteSets, TLC

NoParent == -1
NoInit == -2

VARIABLES inputQ, intCount, busyCount, ready, running, done, jobCounter

FR == INSTANCE FlowRuntimeBase WITH
    Procs <- {{{procs_str}}},
    Flows <- {{{flows_str}}},
    InputsOf <- {inputs_of},
    Conns <- {{{conns}}},
    Parent <- {parent},
    InitOnce <- {init_once},
    InitAlways <- {init_always},
    NoParent <- NoParent,
    NoInit <- NoInit,
    inputQ <- inputQ,
    intCount <- intCount,
    busyCount <- busyCount,
    ready <- ready,
    running <- running,
    done <- done,
    jobCounter <- jobCounter

Init == FR!Init
Next == FR!Next
Spec == FR!Spec

TypeOK == FR!TypeOK
CompletedNeverRuns == FR!CompletedNeverRuns
InternalCountBound == FR!InternalCountBound
AncestorConsistency == FR!AncestorConsistency

==========================================================================
",
        module_name = module_name,
        procs_str = procs_str,
        flows_str = flows_str,
        inputs_of = inputs_of.join(" @@ "),
        conns = conns.join(",\n                  "),
        parent = parent_entries.join(" @@ "),
        init_once = init_once.join(" @@ "),
        init_always = init_always.join(" @@ "),
    )
}

fn copy_base_tla(dir: &Path) -> Result<(), String> {
    let base_dst = dir.join("FlowRuntimeBase.tla");
    if base_dst.exists() {
        return Ok(());
    }
    let candidates = [
        Path::new("specs/FlowRuntimeBase.tla").to_path_buf(),
        std::env::current_exe()
            .ok()
            .and_then(|p| {
                p.parent()
                    .map(|d| d.join("../../specs/FlowRuntimeBase.tla"))
            })
            .unwrap_or_default(),
    ];
    for candidate in &candidates {
        if candidate.exists() {
            fs::copy(candidate, &base_dst)
                .map_err(|e| format!("Cannot copy FlowRuntimeBase.tla: {e}"))?;
            info!("Copied FlowRuntimeBase.tla to {}", dir.display());
            return Ok(());
        }
    }
    Ok(())
}

fn extract_initializer(input: &Value, field: &str, init_type: &str) -> Option<String> {
    input.get(field)?.get(init_type).map(value_to_tla)
}

fn value_to_tla(v: &Value) -> String {
    match v {
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else {
                "1".to_string()
            }
        }
        _ => "1".to_string(),
    }
}

fn sanitize_module_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
