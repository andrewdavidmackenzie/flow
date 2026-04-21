//! Library catalog management: loading manifests, adding functions, resolving sources.

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use log::{info, warn};
use url::Url;

use flowcore::model::lib_manifest::LibraryManifest;
use flowcore::model::process::Process;
use flowcore::model::process_reference::ProcessReference;
use flowcore::provider::Provider;

use crate::canvas_view::NodeLayout;
use crate::flow_io;
use crate::history::EditAction;
use crate::undo_redo;
use crate::WindowState;

/// Load full library catalogs and cache all definitions.
///
/// For each unique library root URL found in `lib_references`, loads the library
/// manifest and parses every function/flow definition in that library. For each
/// URL in `context_references`, parses the context function definition.
pub(crate) fn load_library_catalogs(
    lib_references: &BTreeSet<Url>,
) -> (
    HashMap<Url, LibraryManifest>,
    HashMap<Url, Process>,
    HashMap<Url, Process>,
) {
    let provider = flow_io::build_meta_provider();
    let arc_provider: Arc<dyn Provider> = Arc::new(provider);
    let mut library_cache = HashMap::new();
    let mut lib_definitions = HashMap::new();
    let mut context_definitions = HashMap::new();

    // Extract unique library root URLs from lib_references
    // e.g., "lib://flowstdlib/math/add" -> "lib://flowstdlib"
    let mut lib_roots: BTreeSet<Url> = BTreeSet::new();
    for lib_ref in lib_references {
        if let Some(host) = lib_ref.host_str() {
            if let Ok(root_url) = Url::parse(&format!("lib://{host}")) {
                lib_roots.insert(root_url);
            }
        }
    }

    // Load each library's full manifest
    for lib_root in &lib_roots {
        match LibraryManifest::load(&arc_provider, lib_root) {
            Ok((manifest, _manifest_url)) => {
                info!(
                    "Loaded library manifest for '{}' with {} locators",
                    lib_root,
                    manifest.locators.len()
                );

                // Parse each function/flow in the manifest
                let meta_provider = flow_io::build_meta_provider();
                for locator_url in manifest.locators.keys() {
                    match flowrclib::compiler::parser::parse(locator_url, &meta_provider) {
                        Ok(process) => {
                            lib_definitions.insert(locator_url.clone(), process);
                        }
                        Err(e) => {
                            warn!(
                                "Could not parse library definition '{}': {}",
                                locator_url, e
                            );
                        }
                    }
                }

                library_cache.insert(lib_root.clone(), manifest);
            }
            Err(e) => {
                warn!("Could not load library manifest for '{}': {}", lib_root, e);
            }
        }
    }

    // Discover and parse context functions from the flowrcli runner directory.
    // Only scan ~/.flow/runner/flowrcli/ since flowedit only supports the flowrcli runner,
    // and the MetaProvider is configured with that context root.
    let ctx_provider = flow_io::build_meta_provider();
    let runner_dir = std::env::var("HOME")
        .map(|h| {
            std::path::PathBuf::from(h)
                .join(".flow")
                .join("runner")
                .join("flowrcli")
        })
        .unwrap_or_default();
    if runner_dir.is_dir() {
        if let Ok(cats) = std::fs::read_dir(&runner_dir) {
            for cat_entry in cats.flatten() {
                let cat_path = cat_entry.path();
                if !cat_path.is_dir() {
                    continue;
                }
                let cat_name = cat_entry.file_name().to_string_lossy().to_string();
                if let Ok(funcs) = std::fs::read_dir(&cat_path) {
                    for func_entry in funcs.flatten() {
                        let func_path = func_entry.path();
                        if func_path.extension().and_then(|e| e.to_str()) == Some("toml") {
                            let func_name = func_path
                                .file_stem()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_default();
                            if !func_name.is_empty() {
                                let ctx_url_str = format!("context://{cat_name}/{func_name}");
                                if let Ok(ctx_url) = Url::parse(&ctx_url_str) {
                                    if !context_definitions.contains_key(&ctx_url) {
                                        match flowrclib::compiler::parser::parse(
                                            &ctx_url,
                                            &ctx_provider,
                                        ) {
                                            Ok(process) => {
                                                context_definitions.insert(ctx_url, process);
                                            }
                                            Err(e) => {
                                                warn!(
                                                    "Could not parse context function '{}': {}",
                                                    ctx_url_str, e
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    (library_cache, lib_definitions, context_definitions)
}

/// Add a function from the library panel as a new node on the canvas.
///
/// Creates a `NodeLayout` at a default position and a `ProcessReference`
/// in the flow definition, and records the action in the edit history.
pub(crate) fn add_library_function(win: &mut WindowState, source: &str, func_name: &str) {
    // Generate a unique alias: if the name already exists, append a number
    let alias = flow_io::generate_unique_alias(func_name, &win.nodes);

    // Place the new node at a default position offset from existing nodes
    let (x, y) = flow_io::next_node_position(&win.nodes);

    // Resolve port info and description by parsing the function/flow definition
    let (inputs, outputs, description) = match Url::parse(source) {
        Ok(url) => {
            let provider = flow_io::build_meta_provider();
            match flowrclib::compiler::parser::parse(&url, &provider) {
                Ok(Process::FunctionProcess(func)) => {
                    let ports = flow_io::extract_ports(&func.inputs, &func.outputs);
                    (ports.0, ports.1, func.description.clone())
                }
                Ok(Process::FlowProcess(flow)) => {
                    let ports = flow_io::extract_ports(&flow.inputs, &flow.outputs);
                    (ports.0, ports.1, flow.description.clone())
                }
                Err(e) => {
                    info!("add_library_function: could not parse '{source}': {e}");
                    (Vec::new(), Vec::new(), String::new())
                }
            }
        }
        Err(e) => {
            info!("add_library_function: could not parse URL '{source}': {e}");
            (Vec::new(), Vec::new(), String::new())
        }
    };

    let node = NodeLayout {
        alias: alias.clone(),
        source: source.to_string(),
        description,
        x,
        y,
        width: 180.0,
        height: 120.0,
        inputs,
        outputs,
        initializers: HashMap::new(),
    };

    let index = win.nodes.len();
    win.nodes.push(node.clone());

    // Also add to the flow definition
    let pref = ProcessReference {
        alias: alias.clone(),
        source: source.to_string(),
        initializations: std::collections::BTreeMap::new(),
        x: Some(x),
        y: Some(y),
        width: Some(180.0),
        height: Some(120.0),
    };
    win.flow_definition.process_refs.push(pref);

    undo_redo::record_edit(
        win,
        EditAction::DeleteNode {
            index,
            node,
            removed_edges: Vec::new(),
        },
    );
    // Note: We record a DeleteNode so that *undo* removes the added node.
    // This is intentional: undoing an "add" means deleting what was added.

    win.selected_node = Some(index);
    win.canvas_state.request_redraw();
    // Trigger auto-fit if enabled so the new node is visible
    if win.auto_fit_enabled {
        win.auto_fit_pending = true;
    }
    let nc = win.nodes.len();
    win.status = format!("Added {alias} from library - {nc} nodes");
}

/// Resolve a node's source path relative to the current flow file.
pub(crate) fn resolve_node_source(win: &WindowState, source: &str) -> Option<PathBuf> {
    let base_dir = win.file_path.as_ref()?.parent()?;
    let canonicalize = |p: PathBuf| std::fs::canonicalize(&p).unwrap_or(p);
    let candidate = base_dir.join(source);
    if candidate.exists() {
        return Some(canonicalize(candidate));
    }
    let with_ext = base_dir.join(format!("{source}.toml"));
    if with_ext.exists() {
        return Some(canonicalize(with_ext));
    }
    let dir_default = base_dir.join(source).join("default.toml");
    if dir_default.exists() {
        return Some(canonicalize(dir_default));
    }
    None
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod test {
    use super::*;
    use crate::canvas_view::{FlowCanvasState, PortInfo};
    use crate::hierarchy_panel::FlowHierarchy;
    use crate::history::EditHistory;
    use crate::WindowKind;

    fn test_node(alias: &str, source: &str) -> NodeLayout {
        NodeLayout {
            alias: alias.into(),
            source: source.into(),
            description: String::new(),
            x: 100.0,
            y: 100.0,
            width: 180.0,
            height: 120.0,
            inputs: Vec::new(),
            outputs: Vec::new(),
            initializers: HashMap::new(),
        }
    }

    fn test_win_state() -> WindowState {
        WindowState {
            kind: WindowKind::FlowEditor,
            flow_name: String::from("test"),
            nodes: vec![
                test_node("add", "lib://flowstdlib/math/add"),
                test_node("stdout", "context://stdio/stdout"),
            ],
            edges: Vec::new(),
            canvas_state: FlowCanvasState::default(),
            status: String::new(),
            selected_node: None,
            selected_connection: None,
            history: EditHistory::default(),
            auto_fit_pending: false,
            auto_fit_enabled: false,
            unsaved_edits: 0,
            compiled_manifest: None,
            file_path: None,
            flow_definition: flowcore::model::flow_definition::FlowDefinition::default(),
            tooltip: None,
            initializer_editor: None,
            is_root: true,
            flow_inputs: Vec::new(),
            flow_outputs: Vec::new(),
            context_menu: None,
            show_metadata: false,
            flow_hierarchy: FlowHierarchy::empty(),
            last_size: None,
            last_position: None,
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("flowedit_tests").join(name);
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn resolve_node_source_toml_extension() {
        let dir = temp_dir("resolve_src");
        let flow_path = dir.join("root.toml");
        std::fs::write(&flow_path, "flow = \"root\"").expect("write");
        let sub_path = dir.join("sub.toml");
        std::fs::write(&sub_path, "flow = \"sub\"").expect("write");

        let win = WindowState {
            file_path: Some(flow_path),
            ..test_win_state()
        };

        let resolved = resolve_node_source(&win, "sub");
        assert!(resolved.is_some());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_node_source_not_found() {
        let win = WindowState {
            file_path: Some(PathBuf::from("/tmp/flowedit_tests/nonexistent/root.toml")),
            ..test_win_state()
        };
        let resolved = resolve_node_source(&win, "missing");
        assert!(resolved.is_none());
    }
}
