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

use crate::file_ops;
use crate::history::EditAction;
use crate::WindowState;

/// Load full library catalogs and cache all definitions.
///
/// For each unique library root URL found in `lib_references`, loads the library
/// manifest and parses every function/flow definition in that library. For each
/// URL in `context_references`, parses the context function definition.
pub(crate) fn load_library_catalogs(
    lib_references: &BTreeSet<Url>,
) -> (HashMap<Url, LibraryManifest>, HashMap<Url, Process>) {
    let provider = file_ops::build_meta_provider();
    let arc_provider: Arc<dyn Provider> = Arc::new(provider);
    let mut library_cache = HashMap::new();
    let mut all_definitions = HashMap::new();

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
                let meta_provider = file_ops::build_meta_provider();
                for locator_url in manifest.locators.keys() {
                    match flowrclib::compiler::parser::parse(locator_url, &meta_provider) {
                        Ok(process) => {
                            all_definitions.insert(locator_url.clone(), process);
                        }
                        Err(e) => {
                            warn!("Could not parse library definition '{locator_url}': {e}");
                        }
                    }
                }

                library_cache.insert(lib_root.clone(), manifest);
            }
            Err(e) => {
                warn!("Could not load library manifest for '{lib_root}': {e}");
            }
        }
    }

    // Discover and parse context functions from the flowrcli runner directory.
    // Only scan ~/.flow/runner/flowrcli/ since flowedit only supports the flowrcli runner,
    // and the MetaProvider is configured with that context root.
    let ctx_provider = file_ops::build_meta_provider();
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
                                    if let std::collections::hash_map::Entry::Vacant(entry) =
                                        all_definitions.entry(ctx_url)
                                    {
                                        match flowrclib::compiler::parser::parse(
                                            entry.key(),
                                            &ctx_provider,
                                        ) {
                                            Ok(process) => {
                                                entry.insert(process);
                                            }
                                            Err(e) => {
                                                warn!(
                                                    "Could not parse context function '{ctx_url_str}': {e}"
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

    (library_cache, all_definitions)
}

impl WindowState {
    /// Add a library function as a new node on the canvas.
    pub(crate) fn add_library_function(&mut self, source: &str, func_name: &str) {
        // Generate a unique alias: if the name already exists, append a number
        let alias = file_ops::generate_unique_alias(func_name, &self.flow_definition.process_refs);

        // Place the new node at a default position offset from existing nodes
        let (x, y) = file_ops::next_node_position(&self.flow_definition.process_refs);

        // Resolve the subprocess definition by parsing the function/flow
        let resolved_process = match Url::parse(source) {
            Ok(url) => {
                let provider = file_ops::build_meta_provider();
                match flowrclib::compiler::parser::parse(&url, &provider) {
                    Ok(proc) => Some(proc),
                    Err(e) => {
                        info!("add_library_function: could not parse '{source}': {e}");
                        None
                    }
                }
            }
            Err(e) => {
                info!("add_library_function: could not parse URL '{source}': {e}");
                None
            }
        };

        let pref = ProcessReference {
            alias: alias.clone(),
            source: source.to_string(),
            initializations: std::collections::BTreeMap::new(),
            x: Some(x),
            y: Some(y),
            width: Some(180.0),
            height: Some(120.0),
        };

        let index = self.flow_definition.process_refs.len();
        self.flow_definition.process_refs.push(pref.clone());

        // Add the resolved subprocess definition if we have one
        if let Some(proc) = resolved_process {
            self.flow_definition
                .subprocesses
                .insert(alias.clone(), proc);
        }

        self.history.record(EditAction::CreateNode {
            index,
            process_ref: pref,
            subprocess: self
                .flow_definition
                .subprocesses
                .get(&alias)
                .map(|p| (alias.clone(), p.clone())),
        });

        self.selected_node = Some(index);
        self.canvas_state.request_redraw();
        // Trigger auto-fit if enabled so the new node is visible
        if self.auto_fit_enabled {
            self.auto_fit_pending = true;
        }
        let nc = self.flow_definition.process_refs.len();
        self.status = format!("Added {alias} from library - {nc} nodes");
    }

    /// Resolve a node's source path relative to the current flow file.
    pub(crate) fn resolve_node_source(&self, source: &str) -> Option<PathBuf> {
        let base_dir = self.file_path()?.parent()?.to_path_buf();
        let base_dir = &base_dir;
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
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod test {
    use std::path::Path;

    use super::*;
    use crate::hierarchy_panel::FlowHierarchy;
    use crate::history::EditHistory;
    use crate::window_state::FlowCanvasState;
    use crate::WindowKind;

    fn test_win_state() -> WindowState {
        use flowcore::model::process_reference::ProcessReference;
        use std::collections::BTreeMap;

        let flow_def = flowcore::model::flow_definition::FlowDefinition {
            name: String::from("test"),
            process_refs: vec![
                ProcessReference {
                    alias: "add".into(),
                    source: "lib://flowstdlib/math/add".into(),
                    initializations: BTreeMap::new(),
                    x: Some(100.0),
                    y: Some(100.0),
                    width: Some(180.0),
                    height: Some(120.0),
                },
                ProcessReference {
                    alias: "stdout".into(),
                    source: "context://stdio/stdout".into(),
                    initializations: BTreeMap::new(),
                    x: Some(100.0),
                    y: Some(100.0),
                    width: Some(180.0),
                    height: Some(120.0),
                },
            ],
            ..flowcore::model::flow_definition::FlowDefinition::default()
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

        let mut win = test_win_state();
        win.set_file_path(&flow_path);

        let resolved = win.resolve_node_source("sub");
        assert!(resolved.is_some());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_node_source_not_found() {
        let mut win = test_win_state();
        win.set_file_path(Path::new("/tmp/flowedit_tests/nonexistent/root.toml"));
        let resolved = win.resolve_node_source("missing");
        assert!(resolved.is_none());
    }
}
