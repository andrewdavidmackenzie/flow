//! Flow file operations: loading, saving, compiling, and editor preferences.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::{Path, PathBuf};

use simpath::Simpath;
use url::Url;

use crate::canvas_view::{derive_short_name, FlowCanvasState, PortInfo};
use crate::history::EditHistory;
use crate::{FunctionViewer, WindowState};
use flowcore::meta_provider::MetaProvider;
use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::input::InputInitializer;
use flowcore::model::name::HasName;
use flowcore::model::process::Process;

/// Result of loading a flow definition file.
pub(crate) struct LoadedFlow {
    pub(crate) flow_def: FlowDefinition,
    pub(crate) lib_references: BTreeSet<Url>,
    pub(crate) context_references: BTreeSet<Url>,
}

/// Editor window size and position preferences.
pub(crate) struct EditorPrefs {
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) x: Option<f32>,
    pub(crate) y: Option<f32>,
}

/// Save the current flow to the given path.
pub(crate) fn perform_save(win: &mut WindowState, path: &PathBuf) {
    match save_flow_toml(&win.flow_definition, path) {
        Ok(()) => {
            win.unsaved_edits = 0;
            win.set_file_path(path);
            save_editor_prefs(path, win.last_size, win.last_position);
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                win.status = format!("Saved to {name}");
            } else {
                win.status = String::from("Saved");
            }
        }
        Err(e) => {
            win.status = format!("Save failed: {e}");
        }
    }
}

/// Prompt the user with a save dialog and save to the chosen path.
pub(crate) fn perform_save_as(win: &mut WindowState) {
    let dialog = rfd::FileDialog::new()
        .add_filter("Flow", &["toml"])
        .set_file_name(format!("{}.toml", win.flow_definition.name));
    if let Some(path) = dialog.save_file() {
        perform_save(win, &path);
    }
}

/// Handle save message -- saves to existing path or prompts with save dialog.
pub(crate) fn handle_save(win: &mut WindowState) {
    if let Some(path) = win.file_path() {
        perform_save(win, &path);
    } else {
        perform_save_as(win);
    }
}

/// Handle save-as message -- prompts with save dialog.
pub(crate) fn handle_save_as(win: &mut WindowState) {
    perform_save_as(win);
}

/// Prompt the user with an open dialog and load the selected flow file.
/// Open a flow file and update the window state.
/// Returns the lib and context references if successful, for rebuilding the library cache.
pub(crate) fn perform_open(win: &mut WindowState) -> Option<(BTreeSet<Url>, BTreeSet<Url>)> {
    let dialog = rfd::FileDialog::new().add_filter("Flow", &["toml"]);
    if let Some(path) = dialog.pick_file() {
        match load_flow(&path) {
            Ok(loaded) => {
                let nc = loaded.flow_def.process_refs.len();
                let ec = loaded.flow_def.connections.len();
                win.flow_definition = loaded.flow_def;
                win.set_file_path(&path);
                win.selected_node = None;
                win.selected_connection = None;
                win.history = EditHistory::default();
                win.unsaved_edits = 0;
                win.auto_fit_pending = true;
                win.auto_fit_enabled = true;
                win.canvas_state = FlowCanvasState::default();
                win.status = format!("Loaded - {nc} nodes, {ec} connections");
                return Some((loaded.lib_references, loaded.context_references));
            }
            Err(e) => {
                win.status = format!("Open failed: {e}");
            }
        }
    }
    None
}

/// Clear the canvas and reset to an empty flow state.
pub(crate) fn perform_new(win: &mut WindowState) {
    win.flow_definition = FlowDefinition::default();
    win.flow_definition.name = String::from("(new flow)");
    win.clear_file_path();
    win.selected_node = None;
    win.selected_connection = None;
    win.history = EditHistory::default();
    win.unsaved_edits = 0;
    win.auto_fit_pending = false;
    win.auto_fit_enabled = true;
    win.canvas_state = FlowCanvasState::default();
    win.status = String::from("New flow");
}

/// Compile the current flow to a manifest.
///
/// Writes a temporary copy of the current editor state for the compiler
/// to parse -- the user's flow definition file is never modified.
///
/// Returns the path to the generated manifest on success, or a human-readable
/// error message on failure.
pub(crate) fn perform_compile(win: &mut WindowState) -> Result<PathBuf, String> {
    // New flows must be saved first so the compiler has a real file path
    if win.file_path().is_none() {
        perform_save_as(win);
    }
    let Some(flow_path) = win.file_path() else {
        return Err("Flow must be saved before compiling".to_string());
    };

    // Save any unsaved edits so the file on disk matches the editor state
    if win.unsaved_edits > 0 {
        perform_save(win, &flow_path);
        if win.unsaved_edits > 0 {
            return Err("Save failed — cannot compile stale content".to_string());
        }
    }

    let flow_path = &flow_path;
    let abs_path = if flow_path.is_absolute() {
        flow_path.clone()
    } else {
        std::env::current_dir()
            .map_err(|e| format!("Could not get current directory: {e}"))?
            .join(flow_path)
    };

    let provider = build_meta_provider();

    let url = Url::from_file_path(&abs_path)
        .map_err(|()| format!("Invalid file path: {}", abs_path.display()))?;
    let process = flowrclib::compiler::parser::parse(&url, &provider)
        .map_err(|e| format!("Parse error: {e}"))?;
    let flow = match process {
        Process::FlowProcess(f) => f,
        Process::FunctionProcess(_) => return Err("Not a flow definition".to_string()),
    };

    let output_dir = abs_path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let mut source_urls = BTreeMap::<String, Url>::new();
    let tables =
        flowrclib::compiler::compile::compile(&flow, &output_dir, false, false, &mut source_urls)
            .map_err(|e| e.to_string())?;

    let manifest_path = flowrclib::generator::generate::write_flow_manifest(
        &flow,
        false,
        &output_dir,
        &tables,
        source_urls,
    )
    .map_err(|e| format!("Manifest error: {e}"))?;

    Ok(manifest_path)
}

/// Build a `MetaProvider` with `FLOW_LIB_PATH` (plus `~/.flow/lib` default)
/// and the default flowrcli context root.
pub(crate) fn build_meta_provider() -> MetaProvider {
    let mut lib_search_path = Simpath::new_with_separator("FLOW_LIB_PATH", ',');
    if let Ok(home) = std::env::var("HOME") {
        let default_lib = PathBuf::from(&home).join(".flow").join("lib");
        if default_lib.exists() {
            if let Some(path_str) = default_lib.to_str() {
                lib_search_path.add_directory(path_str);
            }
        }
    }
    let context_root = std::env::var("HOME").map_or_else(
        |_| PathBuf::from("/"),
        |h| {
            PathBuf::from(h)
                .join(".flow")
                .join("runner")
                .join("flowrcli")
        },
    );
    MetaProvider::new(lib_search_path, context_root)
}

/// Resolve the library search paths from the `FLOW_LIB_PATH` environment variable
/// and the default `~/.flow/lib` directory.
pub(crate) fn resolve_lib_paths() -> Vec<String> {
    let mut paths = Vec::new();

    if let Ok(env_path) = std::env::var("FLOW_LIB_PATH") {
        for p in env_path.split(',') {
            let trimmed = p.trim();
            if !trimmed.is_empty() {
                paths.push(trimmed.to_string());
            }
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        let default_lib = format!("{home}/.flow/lib");
        if std::path::Path::new(&default_lib).is_dir() && !paths.contains(&default_lib) {
            paths.push(default_lib);
        }
    }

    paths
}

/// Extract input and output port information from IO definitions.
pub(crate) fn extract_ports(
    inputs: &[flowcore::model::io::IO],
    outputs: &[flowcore::model::io::IO],
) -> (Vec<PortInfo>, Vec<PortInfo>) {
    let input_ports = inputs
        .iter()
        .map(|io| PortInfo {
            name: io.name().clone(),
            datatypes: io.datatypes().iter().map(ToString::to_string).collect(),
        })
        .collect();
    let output_ports = outputs
        .iter()
        .map(|io| PortInfo {
            name: io.name().clone(),
            datatypes: io.datatypes().iter().map(ToString::to_string).collect(),
        })
        .collect();
    (input_ports, output_ports)
}

/// Load a flow definition file and return the flow name, node layouts, edge layouts,
/// the original `FlowDefinition`, and the library/context references for catalog loading.
pub(crate) fn load_flow(path: &PathBuf) -> Result<LoadedFlow, String> {
    let abs_path = if path.is_absolute() {
        path.clone()
    } else {
        std::env::current_dir()
            .map_err(|e| format!("Could not get current directory: {e}"))?
            .join(path)
    };

    let url = Url::from_file_path(&abs_path)
        .map_err(|()| format!("Invalid file path: {}", abs_path.display()))?;

    let provider = build_meta_provider();
    let process = flowrclib::compiler::parser::parse(&url, &provider)
        .map_err(|e| format!("Could not parse flow definition: {e}"))?;

    match process {
        Process::FlowProcess(mut flow) => {
            // Assign default positions to nodes that don't have saved x/y
            assign_default_positions(&mut flow);

            let lib_references = flow.lib_references.clone();
            let context_references = flow.context_references.clone();
            Ok(LoadedFlow {
                flow_def: flow,
                lib_references,
                context_references,
            })
        }
        Process::FunctionProcess(_) => Err(
            "The specified file defines a Function, not a Flow. flowedit requires a flow definition."
                .to_string(),
        ),
    }
}

/// Escape a string value for embedding inside a TOML quoted string (`"..."`).
///
/// Handles backslash, double-quote, and common control characters (newline,
/// carriage return, tab, backspace, form feed).
fn escape_toml_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            other => out.push(other),
        }
    }
    out
}

/// Serialize a `serde_json::Value` into a TOML-compatible inline value string.
pub(crate) fn value_to_toml(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => {
            format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
        }
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "\"null\"".to_string(),
        serde_json::Value::Array(a) => {
            let items: Vec<String> = a.iter().map(value_to_toml).collect();
            format!("[{}]", items.join(", "))
        }
        serde_json::Value::Object(m) => {
            let items: Vec<String> = m
                .iter()
                .map(|(k, val)| format!("{k} = {}", value_to_toml(val)))
                .collect();
            format!("{{ {} }}", items.join(", "))
        }
    }
}

/// Format an `InputInitializer` as a TOML inline table string.
fn initializer_to_toml(init: &InputInitializer) -> String {
    match init {
        InputInitializer::Once(v) => format!("{{ once = {} }}", value_to_toml(v)),
        InputInitializer::Always(v) => format!("{{ always = {} }}", value_to_toml(v)),
    }
}

/// Save a `FlowDefinition` to a TOML file at the given path.
///
/// Builds the TOML text manually to match the expected flow format
/// (the derived `Serialize` on some flowcore types produces struct-style
/// output that is not compatible with the flow deserializer).
pub(crate) fn save_flow_toml(flow: &FlowDefinition, path: &PathBuf) -> Result<(), String> {
    let mut out = String::new();

    // Flow name
    let _ = writeln!(out, "flow = \"{}\"", escape_toml_string(&flow.name));

    // Description
    if !flow.description.is_empty() {
        let _ = writeln!(
            out,
            "description = \"{}\"",
            escape_toml_string(&flow.description)
        );
    }

    // Docs
    if !flow.docs.is_empty() {
        let _ = writeln!(out, "docs = \"{}\"", escape_toml_string(&flow.docs));
    }

    // Metadata (only if any field is non-empty)
    let md = &flow.metadata;
    if !md.version.is_empty() || !md.description.is_empty() || !md.authors.is_empty() {
        out.push_str("\n[metadata]\n");
        if !md.version.is_empty() {
            let _ = writeln!(out, "version = \"{}\"", escape_toml_string(&md.version));
        }
        if !md.description.is_empty() {
            let _ = writeln!(
                out,
                "description = \"{}\"",
                escape_toml_string(&md.description)
            );
        }
        if !md.authors.is_empty() {
            let authors: Vec<String> = md
                .authors
                .iter()
                .map(|a| format!("\"{}\"", escape_toml_string(a)))
                .collect();
            let _ = writeln!(out, "authors = [{}]", authors.join(", "));
        }
    }

    // Flow-level inputs
    for input in &flow.inputs {
        out.push_str("\n[[input]]\n");
        let name = input.name();
        if !name.is_empty() {
            let _ = writeln!(out, "name = \"{name}\"");
        }
        let types = input.datatypes();
        if types.len() == 1 {
            if let Some(t) = types.first() {
                let _ = writeln!(out, "type = \"{t}\"");
            }
        } else if types.len() > 1 {
            let ts: Vec<String> = types.iter().map(|t| format!("\"{t}\"")).collect();
            let _ = writeln!(out, "type = [{}]", ts.join(", "));
        }
    }

    // Flow-level outputs
    for output in &flow.outputs {
        out.push_str("\n[[output]]\n");
        let name = output.name();
        if !name.is_empty() {
            let _ = writeln!(out, "name = \"{name}\"");
        }
        let types = output.datatypes();
        if types.len() == 1 {
            if let Some(t) = types.first() {
                let _ = writeln!(out, "type = \"{t}\"");
            }
        } else if types.len() > 1 {
            let ts: Vec<String> = types.iter().map(|t| format!("\"{t}\"")).collect();
            let _ = writeln!(out, "type = [{}]", ts.join(", "));
        }
    }

    // Processes
    for pref in &flow.process_refs {
        out.push_str("\n[[process]]\n");
        if !pref.alias.is_empty() {
            let _ = writeln!(out, "alias = \"{}\"", pref.alias);
        }
        let _ = writeln!(out, "source = \"{}\"", pref.source);

        // Layout positions
        if let Some(x) = pref.x {
            let _ = writeln!(out, "x = {x}");
        }
        if let Some(y) = pref.y {
            let _ = writeln!(out, "y = {y}");
        }
        if let Some(w) = pref.width {
            let _ = writeln!(out, "width = {w}");
        }
        if let Some(h) = pref.height {
            let _ = writeln!(out, "height = {h}");
        }

        // Initializations
        for (port_name, init) in &pref.initializations {
            let _ = writeln!(out, "input.{port_name} = {}", initializer_to_toml(init));
        }
    }

    // Connections
    for conn in &flow.connections {
        let _ = writeln!(out, "\n[[connection]]");
        if !conn.name().is_empty() {
            let _ = writeln!(out, "name = \"{}\"", conn.name());
        }
        let _ = writeln!(out, "from = \"{}\"", conn.from());
        if let [single] = conn.to().as_slice() {
            let _ = writeln!(out, "to = \"{single}\"");
        } else {
            let to_strs: Vec<String> = conn.to().iter().map(|r| format!("\"{r}\"")).collect();
            let _ = writeln!(out, "to = [{}]", to_strs.join(", "));
        }
    }

    std::fs::write(path, out).map_err(|e| format!("Could not write file: {e}"))
}

/// Generate a unique alias for a new node, appending a numeric suffix if needed.
pub(crate) fn generate_unique_alias(
    base_name: &str,
    process_refs: &[flowcore::model::process_reference::ProcessReference],
) -> String {
    let existing: Vec<String> = process_refs
        .iter()
        .map(|pr| {
            if pr.alias.is_empty() {
                derive_short_name(&pr.source)
            } else {
                pr.alias.clone()
            }
        })
        .collect();
    if !existing.contains(&base_name.to_string()) {
        return base_name.to_string();
    }
    let mut counter = 2u32;
    loop {
        let candidate = format!("{base_name}_{counter}");
        if !existing.contains(&candidate) {
            return candidate;
        }
        counter = counter.saturating_add(1);
    }
}

/// Compute a default position for a new node, offset from the last node or at a default origin.
pub(crate) fn next_node_position(
    process_refs: &[flowcore::model::process_reference::ProcessReference],
) -> (f32, f32) {
    if process_refs.is_empty() {
        return (100.0, 100.0);
    }
    // Find the rightmost node and place the new one to its right
    let max_right = process_refs
        .iter()
        .map(|pr| pr.x.unwrap_or(100.0) + pr.width.unwrap_or(180.0))
        .fold(0.0_f32, f32::max);
    (max_right + 50.0, 100.0)
}

/// Assign default positions to process references that don't have saved x/y coordinates.
///
/// Uses topological layout based on connections to determine column placement.
fn assign_default_positions(flow: &mut FlowDefinition) {
    use crate::canvas_view;

    let needs_layout = flow
        .process_refs
        .iter()
        .any(|pr| pr.x.is_none() || pr.y.is_none());
    if !needs_layout {
        return;
    }

    // Build render nodes (which computes topo positions) and copy back positions
    let render_nodes = canvas_view::build_render_nodes(flow);
    for (pref, node) in flow.process_refs.iter_mut().zip(render_nodes.iter()) {
        if pref.x.is_none() {
            pref.x = Some(node.x);
        }
        if pref.y.is_none() {
            pref.y = Some(node.y);
        }
    }
}

/// Format a connection endpoint for display, omitting "default" or empty port names.
pub(crate) fn format_endpoint(node: &str, port: &str) -> String {
    if port.is_empty() || port == "default" || port == "output" {
        node.to_string()
    } else {
        format!("{node}/{port}")
    }
}

/// Save a function definition to disk (TOML, skeleton .rs, and function.toml).
pub(crate) fn save_function_definition(viewer: &FunctionViewer) -> Result<(), String> {
    let dir = viewer
        .toml_path
        .parent()
        .ok_or_else(|| "Invalid path".to_string())?;
    std::fs::create_dir_all(dir).map_err(|e| format!("Could not create directory: {e}"))?;

    // 1. Write the function definition TOML
    let mut toml = format!(
        "function = \"{}\"\nsource = \"{}\"\ntype = \"rust\"\n",
        escape_toml_string(&viewer.name),
        escape_toml_string(&viewer.source_file)
    );
    if !viewer.description.is_empty() {
        let _ = writeln!(
            toml,
            "description = \"{}\"",
            escape_toml_string(&viewer.description)
        );
    }
    for input in &viewer.inputs {
        let dtype = input.datatypes.first().map_or("", String::as_str);
        if input.name.is_empty() || input.name == "input" || input.name == "name" {
            let _ = write!(toml, "\n[[input]]\ntype = \"{dtype}\"\n");
        } else {
            let _ = write!(
                toml,
                "\n[[input]]\nname = \"{}\"\ntype = \"{dtype}\"\n",
                input.name
            );
        }
    }
    for output in &viewer.outputs {
        let dtype = output.datatypes.first().map_or("", String::as_str);
        if output.name.is_empty() || output.name == "output" || output.name == "name" {
            let _ = write!(toml, "\n[[output]]\ntype = \"{dtype}\"\n");
        } else {
            let _ = write!(
                toml,
                "\n[[output]]\nname = \"{}\"\ntype = \"{dtype}\"\n",
                output.name
            );
        }
    }
    std::fs::write(&viewer.toml_path, &toml)
        .map_err(|e| format!("Could not write {}: {e}", viewer.toml_path.display()))?;

    // 2. Generate skeleton .rs if it doesn't exist
    let rs_path = dir.join(&viewer.source_file);
    if !rs_path.exists() {
        let input_count = viewer.inputs.len();
        let input_bindings = (0..input_count).fold(String::new(), |mut acc, i| {
            use std::fmt::Write;
            let _ = writeln!(acc, "    let _input{i} = &inputs[{i}];");
            acc
        });
        let skeleton = format!(
            "use flowcore::{{RUN_AGAIN, RunAgain}};\n\
             use flowcore::errors::*;\n\
             use flowmacro::flow_function;\n\
             use serde_json::Value;\n\
             \n\
             #[flow_function]\n\
             fn _{name}(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {{\n\
             {input_bindings}\
             \n    // TODO: implement function logic\n\
             \n    Ok((None, RUN_AGAIN))\n\
             }}\n",
            name = viewer.name,
        );
        std::fs::write(&rs_path, &skeleton)
            .map_err(|e| format!("Could not write {}: {e}", rs_path.display()))?;
    }

    // 3. Generate function.toml (Cargo manifest) if it doesn't exist
    let cargo_path = dir.join("function.toml");
    if !cargo_path.exists() {
        let stem = viewer
            .source_file
            .strip_suffix(".rs")
            .unwrap_or(&viewer.source_file);
        let cargo = format!(
            "[package]\n\
             name = \"{name}\"\n\
             version = \"0.1.0\"\n\
             edition = \"2021\"\n\
             \n\
             [lib]\n\
             name = \"{name}\"\n\
             crate-type = [\"cdylib\"]\n\
             path = \"{source}\"\n\
             \n\
             [dependencies]\n\
             flowcore = {{version = \"0\"}}\n\
             flowmacro = {{version = \"0\"}}\n\
             serde_json = {{version = \"1.0\", default-features = false}}\n",
            name = escape_toml_string(&viewer.name),
            source = escape_toml_string(stem),
        );
        std::fs::write(&cargo_path, &cargo)
            .map_err(|e| format!("Could not write {}: {e}", cargo_path.display()))?;
    }

    Ok(())
}

/// Compute the path for the editor preferences file alongside the flow file.
pub(crate) fn editor_prefs_path(flow_path: &Path) -> PathBuf {
    let mut p = flow_path.to_path_buf();
    let name = p.file_name().map_or_else(
        || ".flowedit".to_string(),
        |n| format!(".{}.flowedit", n.to_string_lossy()),
    );
    p.set_file_name(name);
    p
}

/// Save editor preferences (window size and position) alongside the flow file.
pub(crate) fn save_editor_prefs(
    flow_path: &Path,
    size: Option<iced::Size>,
    position: Option<iced::Point>,
) {
    let prefs_path = editor_prefs_path(flow_path);
    let mut map = serde_json::Map::new();
    if let Some(s) = size {
        map.insert("width".into(), serde_json::json!(s.width));
        map.insert("height".into(), serde_json::json!(s.height));
    }
    if let Some(p) = position {
        map.insert("x".into(), serde_json::json!(p.x));
        map.insert("y".into(), serde_json::json!(p.y));
    }
    let json = serde_json::Value::Object(map).to_string();
    let _ = std::fs::write(prefs_path, json);
}

/// Load editor preferences from the prefs file alongside the flow file.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn load_editor_prefs(flow_path: &Path) -> Option<EditorPrefs> {
    let prefs_path = editor_prefs_path(flow_path);
    let content = std::fs::read_to_string(prefs_path).ok()?;
    let val: serde_json::Value = serde_json::from_str(&content).ok()?;
    let w = val.get("width")?.as_f64()? as f32;
    let h = val.get("height")?.as_f64()? as f32;
    let x = val
        .get("x")
        .and_then(serde_json::Value::as_f64)
        .map(|v| v as f32);
    let y = val
        .get("y")
        .and_then(serde_json::Value::as_f64)
        .map(|v| v as f32);
    Some(EditorPrefs {
        width: w,
        height: h,
        x,
        y,
    })
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod test {
    use super::*;

    use flowcore::model::process_reference::ProcessReference;

    use crate::canvas_view::FlowCanvasState;
    use crate::hierarchy_panel::FlowHierarchy;
    use crate::history::EditHistory;
    use crate::WindowKind;

    fn test_pref(alias: &str, source: &str) -> ProcessReference {
        ProcessReference {
            alias: alias.into(),
            source: source.into(),
            initializations: std::collections::BTreeMap::new(),
            x: Some(100.0),
            y: Some(100.0),
            width: Some(180.0),
            height: Some(120.0),
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("flowedit_tests").join(name);
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn unique_alias_no_conflict() {
        let prefs = vec![test_pref("add", "lib://test")];
        assert_eq!(generate_unique_alias("subtract", &prefs), "subtract");
    }

    #[test]
    fn unique_alias_with_conflict() {
        let prefs = vec![test_pref("add", "lib://test")];
        assert_eq!(generate_unique_alias("add", &prefs), "add_2");
    }

    #[test]
    fn unique_alias_multiple_conflicts() {
        let prefs = vec![
            test_pref("add", "lib://test"),
            test_pref("add_2", "lib://test"),
        ];
        assert_eq!(generate_unique_alias("add", &prefs), "add_3");
    }

    #[test]
    fn next_position_empty() {
        let prefs: Vec<ProcessReference> = vec![];
        let (x, y) = next_node_position(&prefs);
        assert!((x - 100.0_f32).abs() < 0.01);
        assert!((y - 100.0_f32).abs() < 0.01);
    }

    #[test]
    fn next_position_after_nodes() {
        let prefs = vec![test_pref("a", "lib://test")];
        let (x, _y) = next_node_position(&prefs);
        assert!(x > 280.0); // right of existing node + gap
    }

    #[test]
    fn format_endpoint_with_port() {
        assert_eq!(format_endpoint("add", "i1"), "add/i1");
    }

    #[test]
    fn format_endpoint_empty_port() {
        assert_eq!(format_endpoint("add", ""), "add");
    }

    #[test]
    fn format_endpoint_default_port() {
        assert_eq!(format_endpoint("add", "default"), "add");
    }

    #[test]
    fn format_endpoint_output_port() {
        assert_eq!(format_endpoint("add", "output"), "add");
    }

    #[test]
    fn escape_toml_string_special_chars() {
        assert_eq!(escape_toml_string("hello"), "hello");
        assert_eq!(escape_toml_string("say \"hi\""), "say \\\"hi\\\"");
        assert_eq!(escape_toml_string("back\\slash"), "back\\\\slash");
        assert_eq!(escape_toml_string("line\nnewline"), "line\\nnewline");
        assert_eq!(escape_toml_string("tab\there"), "tab\\there");
        assert_eq!(escape_toml_string("cr\rreturn"), "cr\\rreturn");
    }

    #[test]
    fn value_to_toml_string() {
        assert_eq!(value_to_toml(&serde_json::json!("hello")), "\"hello\"");
    }

    #[test]
    fn value_to_toml_number() {
        assert_eq!(value_to_toml(&serde_json::json!(42)), "42");
    }

    #[test]
    fn value_to_toml_bool() {
        assert_eq!(value_to_toml(&serde_json::json!(true)), "true");
    }

    #[test]
    fn value_to_toml_array() {
        assert_eq!(value_to_toml(&serde_json::json!([1, 2, 3])), "[1, 2, 3]");
    }

    #[test]
    fn initializer_to_toml_once() {
        let init = InputInitializer::Once(serde_json::json!(42));
        assert_eq!(initializer_to_toml(&init), "{ once = 42 }");
    }

    #[test]
    fn initializer_to_toml_always() {
        let init = InputInitializer::Always(serde_json::json!("hello"));
        assert_eq!(initializer_to_toml(&init), "{ always = \"hello\" }");
    }

    #[test]
    fn editor_prefs_path_format() {
        let path = editor_prefs_path(Path::new("/tmp/test/root.toml"));
        assert_eq!(
            path.file_name().and_then(|n| n.to_str()),
            Some(".root.toml.flowedit")
        );
    }

    #[test]
    fn editor_prefs_roundtrip() {
        let dir = temp_dir("prefs_roundtrip");
        let flow_path = dir.join("test_flow.toml");
        std::fs::write(&flow_path, "flow = \"test\"").expect("write test flow");

        save_editor_prefs(
            &flow_path,
            Some(iced::Size::new(800.0, 600.0)),
            Some(iced::Point::new(100.0, 200.0)),
        );

        let prefs = load_editor_prefs(&flow_path);
        assert!(prefs.is_some());
        let p = prefs.expect("prefs should load");
        assert!((p.width - 800.0).abs() < 0.01);
        assert!((p.height - 600.0).abs() < 0.01);
        assert_eq!(p.x, Some(100.0));
        assert_eq!(p.y, Some(200.0));

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn editor_prefs_no_file() {
        let prefs = load_editor_prefs(Path::new("/nonexistent/path.toml"));
        assert!(prefs.is_none());
    }

    #[test]
    fn save_and_load_flow_roundtrip() {
        use flowcore::model::connection::Connection;

        let dir = temp_dir("save_load");
        let path = dir.join("test.toml");

        let mut flow = FlowDefinition {
            name: "roundtrip_test".into(),
            ..FlowDefinition::default()
        };
        flow.metadata.version = "1.0.0".into();
        flow.metadata.authors = vec!["Test Author".into()];
        flow.process_refs.push(ProcessReference {
            alias: "add1".into(),
            source: "lib://flowstdlib/math/add".into(),
            initializations: std::collections::BTreeMap::new(),
            x: Some(100.0),
            y: Some(200.0),
            width: Some(180.0),
            height: Some(120.0),
        });
        flow.connections.push(Connection::new("add1", "add1/i1"));

        save_flow_toml(&flow, &path).expect("save failed");

        let contents = std::fs::read_to_string(&path).expect("read failed");
        assert!(contents.contains("flow = \"roundtrip_test\""));
        assert!(contents.contains("version = \"1.0.0\""));
        assert!(contents.contains("Test Author"));
        assert!(contents.contains("lib://flowstdlib/math/add"));

        let loaded = load_flow(&path).expect("load failed");
        assert_eq!(loaded.flow_def.name, "roundtrip_test");
        assert_eq!(loaded.flow_def.process_refs.len(), 1);
        assert_eq!(loaded.flow_def.connections.len(), 1);
        assert_eq!(loaded.flow_def.metadata.version, "1.0.0");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_flow_with_metadata() {
        let dir = temp_dir("metadata");
        let path = dir.join("meta.toml");

        let mut flow = FlowDefinition {
            name: "meta_flow".into(),
            ..FlowDefinition::default()
        };
        flow.metadata.description = "A test description".into();

        save_flow_toml(&flow, &path).expect("save failed");
        let contents = std::fs::read_to_string(&path).expect("read failed");
        assert!(contents.contains("description = \"A test description\""));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_flow_with_description() {
        let dir = temp_dir("description");
        let path = dir.join("test_flow.toml");

        let flow = FlowDefinition {
            name: "described_flow".into(),
            description: "A test flow that does something".into(),
            ..FlowDefinition::default()
        };

        save_flow_toml(&flow, &path).expect("Could not save flow");

        let content = std::fs::read_to_string(&path).expect("Could not read saved file");
        assert!(content.contains("description = \"A test flow that does something\""));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_flow_with_initializers() {
        let dir = temp_dir("initializers");
        let path = dir.join("init.toml");

        let mut flow = FlowDefinition {
            name: "init_flow".into(),
            ..FlowDefinition::default()
        };
        let mut inits = std::collections::BTreeMap::new();
        inits.insert(
            "start".to_string(),
            InputInitializer::Once(serde_json::json!(42)),
        );
        flow.process_refs.push(ProcessReference {
            alias: "seq".into(),
            source: "lib://flowstdlib/math/sequence".into(),
            initializations: inits,
            x: Some(50.0),
            y: Some(50.0),
            width: Some(180.0),
            height: Some(120.0),
        });

        save_flow_toml(&flow, &path).expect("save failed");
        let contents = std::fs::read_to_string(&path).expect("read failed");
        assert!(contents.contains("input.start"));
        assert!(contents.contains("once"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_flow_with_connections() {
        use flowcore::model::connection::Connection;

        let dir = temp_dir("connections");
        let path = dir.join("conn.toml");

        let mut flow = FlowDefinition {
            name: "conn_flow".into(),
            ..FlowDefinition::default()
        };
        flow.process_refs.push(ProcessReference {
            alias: "a".into(),
            source: "lib://test/a".into(),
            initializations: std::collections::BTreeMap::new(),
            x: Some(0.0),
            y: Some(0.0),
            width: None,
            height: None,
        });
        flow.process_refs.push(ProcessReference {
            alias: "b".into(),
            source: "lib://test/b".into(),
            initializations: std::collections::BTreeMap::new(),
            x: None,
            y: None,
            width: None,
            height: None,
        });

        flow.connections.push(Connection::new("a/out", "b/in"));

        save_flow_toml(&flow, &path).expect("save failed");
        let contents = std::fs::read_to_string(&path).expect("read failed");
        assert!(contents.contains("from = \"a/out\""));
        assert!(contents.contains("to = \"b/in\""));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_flow_nonexistent() {
        let result = load_flow(&PathBuf::from("/nonexistent/flow.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn load_flow_invalid_toml() {
        let dir = temp_dir("invalid");
        let path = dir.join("bad.toml");
        std::fs::write(&path, "this is not valid toml {{{{").expect("write failed");
        let result = load_flow(&path);
        assert!(result.is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_function_definition_creates_files() {
        let dir = temp_dir("func_def");
        let toml_path = dir.join("myfunc.toml");

        let viewer = FunctionViewer {
            name: "myfunc".into(),
            description: String::new(),
            source_file: "myfunc.rs".into(),
            inputs: vec![PortInfo {
                name: "data".into(),
                datatypes: vec!["string".into()],
            }],
            outputs: vec![PortInfo {
                name: "result".into(),
                datatypes: vec!["number".into()],
            }],
            rs_content: String::new(),
            docs_content: None,
            active_tab: 0,
            toml_path: toml_path.clone(),
            parent_window: None,
            node_source: String::new(),
            read_only: false,
        };

        save_function_definition(&viewer).expect("save failed");

        // Check TOML was created
        let toml = std::fs::read_to_string(&toml_path).expect("read toml");
        assert!(toml.contains("function = \"myfunc\""));
        assert!(toml.contains("source = \"myfunc.rs\""));
        assert!(toml.contains("name = \"data\""));
        assert!(toml.contains("type = \"string\""));
        assert!(toml.contains("type = \"number\""));

        // Check skeleton .rs was created
        let rs_path = dir.join("myfunc.rs");
        assert!(rs_path.exists());
        let rs = std::fs::read_to_string(&rs_path).expect("read rs");
        assert!(rs.contains("#[flow_function]"));
        assert!(rs.contains("_myfunc"));
        assert!(rs.contains("_input0"));

        // Check function.toml was created
        let cargo_path = dir.join("function.toml");
        assert!(cargo_path.exists());
        let cargo = std::fs::read_to_string(&cargo_path).expect("read cargo");
        assert!(cargo.contains("name = \"myfunc\""));
        assert!(cargo.contains("crate-type = [\"cdylib\"]"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_function_no_overwrite_existing_rs() {
        let dir = temp_dir("func_no_overwrite");
        let toml_path = dir.join("existing.toml");
        let rs_path = dir.join("existing.rs");

        // Create existing .rs
        std::fs::write(&rs_path, "// existing code").expect("write rs");

        let viewer = FunctionViewer {
            name: "existing".into(),
            description: String::new(),
            source_file: "existing.rs".into(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            rs_content: String::new(),
            docs_content: None,
            active_tab: 0,
            toml_path,
            parent_window: None,
            node_source: String::new(),
            read_only: false,
        };

        save_function_definition(&viewer).expect("save failed");

        // Existing .rs should NOT be overwritten
        let rs = std::fs::read_to_string(&rs_path).expect("read rs");
        assert_eq!(rs, "// existing code");

        let _ = std::fs::remove_dir_all(&dir);
    }

    fn test_win_state() -> WindowState {
        let flow_def = FlowDefinition {
            name: String::from("test"),
            process_refs: vec![
                test_pref("add", "lib://flowstdlib/math/add"),
                test_pref("stdout", "context://stdio/stdout"),
            ],
            ..FlowDefinition::default()
        };
        WindowState {
            kind: WindowKind::FlowEditor,
            canvas_state: FlowCanvasState::default(),
            status: String::new(),
            selected_node: None,
            selected_connection: None,
            history: EditHistory::default(),
            auto_fit_pending: false,
            auto_fit_enabled: false,
            unsaved_edits: 0,
            compiled_manifest: None,
            flow_definition: flow_def,
            tooltip: None,
            initializer_editor: None,
            is_root: true,
            context_menu: None,
            show_metadata: false,
            flow_hierarchy: FlowHierarchy::empty(),
            last_size: None,
            last_position: None,
        }
    }

    #[test]
    fn perform_save_updates_state() {
        let dir = temp_dir("perform_save");
        let path = dir.join("saved.toml");

        let mut win = test_win_state();
        win.unsaved_edits = 5;
        win.flow_definition.name = "saved_flow".into();

        perform_save(&mut win, &path);
        assert_eq!(win.unsaved_edits, 0);
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        assert_eq!(win.file_path(), Some(canonical));

        let contents = std::fs::read_to_string(&path).expect("read failed");
        assert!(contents.contains("flow = \"saved_flow\""));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
