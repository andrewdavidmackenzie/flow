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
        &fd.flow_init_once,
        &fd.flow_init_always,
    );

    let dir = manifest_path
        .parent()
        .ok_or("Cannot get manifest directory")?;
    let tla_path = dir.join(format!("{module_name}.tla"));
    let cfg_path = dir.join(format!("{module_name}.cfg"));

    fs::write(&tla_path, tla).map_err(|e| format!("Cannot write .tla: {e}"))?;
    fs::write(
        &cfg_path,
        "SPECIFICATION Spec\n\nCONSTRAINT\n    StateConstraint\n\nINVARIANTS\n    TypeOK\n    InternalCountBound\n    AncestorConsistency\n",
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
    flow_init_once: Vec<String>,
    flow_init_always: Vec<String>,
}

fn extract_functions(functions: &serde_json::Map<String, Value>) -> Result<FuncData, String> {
    let mut procs = Vec::new();
    let mut inputs_of = Vec::new();
    let mut conns = Vec::new();
    let mut parent_entries = Vec::new();
    let mut init_once = Vec::new();
    let mut init_always = Vec::new();
    let mut flow_init_once = Vec::new();
    let mut flow_init_always = Vec::new();

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
        let mut flow_once_parts = Vec::new();
        let mut flow_always_parts = Vec::new();
        for (idx, input) in inputs.iter().enumerate() {
            let func_once = extract_initializer(input, "initializer", "once");
            let func_always = extract_initializer(input, "initializer", "always");
            let fl_once = extract_initializer(input, "flow_initializer", "once");
            let fl_always = extract_initializer(input, "flow_initializer", "always");
            once_parts.push(format!(
                "{idx} :> {}",
                func_once.as_deref().unwrap_or("NoInit")
            ));
            always_parts.push(format!(
                "{idx} :> {}",
                func_always.as_deref().unwrap_or("NoInit")
            ));
            flow_once_parts.push(format!(
                "{idx} :> {}",
                fl_once.as_deref().unwrap_or("NoInit")
            ));
            flow_always_parts.push(format!(
                "{idx} :> {}",
                fl_always.as_deref().unwrap_or("NoInit")
            ));
        }
        if once_parts.is_empty() {
            init_once.push(format!("{fid} :> <<>>"));
            init_always.push(format!("{fid} :> <<>>"));
            flow_init_once.push(format!("{fid} :> <<>>"));
            flow_init_always.push(format!("{fid} :> <<>>"));
        } else {
            init_once.push(format!("{fid} :> ({})", once_parts.join(" @@ ")));
            init_always.push(format!("{fid} :> ({})", always_parts.join(" @@ ")));
            flow_init_once.push(format!("{fid} :> ({})", flow_once_parts.join(" @@ ")));
            flow_init_always.push(format!("{fid} :> ({})", flow_always_parts.join(" @@ ")));
        }

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
        flow_init_once,
        flow_init_always,
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
    flow_init_once: &[String],
    flow_init_always: &[String],
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
    FlowInitOnce <- {flow_init_once},
    FlowInitAlways <- {flow_init_always},
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
InternalCountBound == FR!InternalCountBound
AncestorConsistency == FR!AncestorConsistency
StateConstraint == jobCounter <= 8

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
        flow_init_once = flow_init_once.join(" @@ "),
        flow_init_always = flow_init_always.join(" @@ "),
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
    log::warn!(
        "FlowRuntimeBase.tla not found — TLC will fail. \
         Ensure specs/FlowRuntimeBase.tla exists in the working directory."
    );
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use super::*;
    use serde_json::json;
    use std::io::Write;

    #[test]
    fn sanitize_replaces_special_chars() {
        assert_eq!(sanitize_module_name("my-first-flow"), "my_first_flow");
        assert_eq!(sanitize_module_name("hello_world"), "hello_world");
        assert_eq!(sanitize_module_name("a.b.c"), "a_b_c");
    }

    #[test]
    fn value_to_tla_integer() {
        assert_eq!(value_to_tla(&json!(42)), "42");
        assert_eq!(value_to_tla(&json!(-1)), "-1");
        assert_eq!(value_to_tla(&json!(0)), "0");
    }

    #[test]
    fn value_to_tla_non_integer() {
        assert_eq!(value_to_tla(&json!("hello")), "1");
        assert_eq!(value_to_tla(&json!(true)), "1");
        assert_eq!(value_to_tla(&json!([1, 2])), "1");
        assert_eq!(value_to_tla(&json!(2.5)), "1");
    }

    #[test]
    fn extract_initializer_once() {
        let input = json!({"initializer": {"once": 5}});
        assert_eq!(
            extract_initializer(&input, "initializer", "once"),
            Some("5".into())
        );
    }

    #[test]
    fn extract_initializer_always() {
        let input = json!({"flow_initializer": {"always": 9}});
        assert_eq!(
            extract_initializer(&input, "flow_initializer", "always"),
            Some("9".into())
        );
    }

    #[test]
    fn extract_initializer_missing() {
        let input = json!({"name": "i1"});
        assert_eq!(extract_initializer(&input, "initializer", "once"), None);
    }

    #[test]
    fn generate_tla_from_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = json!({
            "metadata": {"name": "test_flow", "version": "1.0.0", "description": "", "authors": []},
            "lib_references": [],
            "context_references": [],
            "source_urls": [],
            "functions": {
                "1": {
                    "process_id": 1,
                    "parent_id": 0,
                    "implementation_location": "lib://test/add",
                    "inputs": [
                        {"name": "i1", "initializer": {"once": 1}},
                        {"name": "i2", "initializer": {"once": 2}}
                    ],
                    "output_connections": [
                        {"destination_id": 2, "destination_io_number": 0, "internal": true}
                    ]
                },
                "2": {
                    "process_id": 2,
                    "parent_id": 0,
                    "implementation_location": "context://stdio/stdout",
                    "inputs": [{"generic": true}],
                    "output_connections": []
                }
            },
            "flows": {
                "0": {
                    "process_id": 0,
                    "parent_id": null,
                    "sub_flow_ids": [],
                    "name": "test_flow",
                    "route": "/test_flow"
                }
            }
        });

        let manifest_path = dir.path().join("manifest.json");
        let mut f = std::fs::File::create(&manifest_path).unwrap();
        f.write_all(serde_json::to_string_pretty(&manifest).unwrap().as_bytes())
            .unwrap();

        // Create a fake FlowRuntimeBase.tla so the copy succeeds
        std::fs::write(
            dir.path().join("FlowRuntimeBase.tla"),
            "---- MODULE FlowRuntimeBase ----\n====",
        )
        .unwrap();

        generate_tla(&manifest_path).expect("TLA generation should succeed");

        let tla_path = dir.path().join("test_flow.tla");
        let cfg_path = dir.path().join("test_flow.cfg");
        assert!(tla_path.exists(), ".tla file should be created");
        assert!(cfg_path.exists(), ".cfg file should be created");

        let tla_content = std::fs::read_to_string(&tla_path).unwrap();
        assert!(tla_content.contains("MODULE test_flow"));
        assert!(tla_content.contains("Procs <- {1, 2}"));
        assert!(tla_content.contains("Flows <- {0}"));
        assert!(tla_content.contains("INSTANCE FlowRuntimeBase"));
        assert!(tla_content.contains("internal |-> TRUE"));
        assert!(tla_content.contains("FlowInitOnce <-"));
        assert!(tla_content.contains("FlowInitAlways <-"));

        let cfg_content = std::fs::read_to_string(&cfg_path).unwrap();
        assert!(cfg_content.contains("SPECIFICATION Spec"));
        assert!(cfg_content.contains("TypeOK"));
        assert!(!cfg_content.contains("CompletedNeverRuns"));
        assert!(cfg_content.contains("InternalCountBound"));
        assert!(cfg_content.contains("AncestorConsistency"));
    }

    #[test]
    fn generate_tla_derives_name_from_directory() {
        let dir = tempfile::tempdir().unwrap();
        let subdir = dir.path().join("my_example");
        std::fs::create_dir(&subdir).unwrap();

        let manifest = json!({
            "metadata": {"name": "", "version": "1.0.0", "description": "", "authors": []},
            "functions": {
                "1": {
                    "process_id": 1, "parent_id": 0,
                    "implementation_location": "test",
                    "inputs": [],
                    "output_connections": []
                }
            },
            "flows": {
                "0": {"process_id": 0, "parent_id": null, "sub_flow_ids": [], "name": "", "route": "/"}
            }
        });

        let manifest_path = subdir.join("manifest.json");
        std::fs::write(&manifest_path, serde_json::to_string(&manifest).unwrap()).unwrap();
        std::fs::write(subdir.join("FlowRuntimeBase.tla"), "stub").unwrap();

        generate_tla(&manifest_path).unwrap();
        assert!(subdir.join("my_example.tla").exists());
    }

    #[test]
    fn generate_tla_empty_inputs_function() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = json!({
            "metadata": {"name": "empty_test", "version": "1.0.0", "description": "", "authors": []},
            "functions": {
                "1": {
                    "process_id": 1, "parent_id": 0,
                    "implementation_location": "test",
                    "inputs": [],
                    "output_connections": []
                }
            },
            "flows": {
                "0": {"process_id": 0, "parent_id": null, "sub_flow_ids": [], "name": "", "route": "/"}
            }
        });

        let manifest_path = dir.path().join("manifest.json");
        std::fs::write(&manifest_path, serde_json::to_string(&manifest).unwrap()).unwrap();
        std::fs::write(dir.path().join("FlowRuntimeBase.tla"), "stub").unwrap();

        generate_tla(&manifest_path).unwrap();
        let tla = std::fs::read_to_string(dir.path().join("empty_test.tla")).unwrap();
        assert!(tla.contains("InputsOf <- 1 :> {}"));
        assert!(tla.contains("1 :> <<>>"));
    }
}
