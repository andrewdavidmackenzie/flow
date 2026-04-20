//! `flowedit` is a visual editor for flow definition files.
//!
//! Phase 1 provides a read-only viewer that renders the process nodes and connections
//! from a flow definition file onto an iced [`Canvas`][iced::widget::canvas::Canvas].
//!
//! Usage:
//! ```text
//! flowedit [flow-definition-file]
//! ```
//!
//! The flow file (TOML, YAML, or JSON) is parsed using flowcore's deserializer
//! and each [`ProcessReference`][flowcore::model::process_reference::ProcessReference]
//! is displayed as a colored, rounded rectangle on the canvas, with connections
//! drawn as bezier curves between nodes.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use clap::{Arg, ArgAction, Command as ClapCommand};
use iced::keyboard;
use iced::widget::{button, container, pick_list, stack, text_input, Column, Row, Text};
use iced::window;
use iced::{Color, Element, Fill, Subscription, Task, Theme};
use log::{info, warn};
use simpath::Simpath;
use url::Url;

use flowcore::deserializers::deserializer::get;
use flowcore::meta_provider::MetaProvider;
use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::input::InputInitializer;
use flowcore::model::lib_manifest::LibraryManifest;
use flowcore::model::name::HasName;
use flowcore::model::process::Process;
use flowcore::model::process_reference::ProcessReference;
use flowcore::provider::Provider;
mod canvas_view;
mod hierarchy_panel;
mod history;
mod library_panel;
use canvas_view::{
    build_edge_layouts, build_node_layouts, derive_short_name, CanvasMessage, EdgeLayout,
    FlowCanvasState, NodeLayout, PortInfo,
};
use hierarchy_panel::{FlowHierarchy, HierarchyMessage};
use history::{EditAction, EditHistory};
use library_panel::{LibraryAction, LibraryMessage, LibraryTree};

/// Messages handled by the flowedit application
#[derive(Debug, Clone)]
enum Message {
    /// A message from the interactive canvas, tagged with its window ID
    WindowCanvas(window::Id, CanvasMessage),
    /// A message from the library side panel, tagged with the originating window ID
    Library(window::Id, LibraryMessage),
    /// A message from the flow hierarchy panel, tagged with window ID
    Hierarchy(window::Id, HierarchyMessage),
    /// Zoom in by one step, tagged with the originating window ID
    ZoomIn(window::Id),
    /// Zoom out by one step, tagged with the originating window ID
    ZoomOut(window::Id),
    /// Toggle auto-fit mode, tagged with the originating window ID
    ToggleAutoFit(window::Id),
    /// Undo the last edit action
    Undo,
    /// Redo the last undone action
    Redo,
    /// Save the flow to the current file (or prompt if none)
    Save,
    /// Save the flow to a new file (always prompts)
    SaveAs,
    /// Open a flow file
    Open,
    /// Create a new empty flow
    New,
    /// Compile the current flow
    Compile,
    /// Initializer type changed in the editor dialog, tagged with the originating window ID
    InitializerTypeChanged(window::Id, String),
    /// Initializer value changed in the editor dialog, tagged with the originating window ID
    InitializerValueChanged(window::Id, String),
    /// Apply the initializer edit, tagged with the originating window ID
    InitializerApply(window::Id),
    /// Cancel the initializer edit, tagged with the originating window ID
    InitializerCancel(window::Id),
    /// Switch tab in a function viewer window
    FunctionTabSelected(window::Id, usize),
    /// Function name edited
    FunctionNameChanged(window::Id, String),
    /// Browse for source file
    FunctionBrowseSource(window::Id),
    /// Add a new input port to a function
    FunctionAddInput(window::Id),
    /// Add a new output port to a function
    FunctionAddOutput(window::Id),
    /// Delete an input port from a function
    FunctionDeleteInput(window::Id, usize),
    /// Delete an output port from a function
    FunctionDeleteOutput(window::Id, usize),
    /// Input port name changed
    FunctionInputNameChanged(window::Id, usize, String),
    /// Input port type changed
    FunctionInputTypeChanged(window::Id, usize, String),
    /// Output port name changed
    FunctionOutputNameChanged(window::Id, usize, String),
    /// Output port type changed
    FunctionOutputTypeChanged(window::Id, usize, String),
    /// Save function definition to disk
    FunctionSave(window::Id),
    /// Create a new sub-flow and add it to the current flow
    NewSubFlow,
    /// Create a new provided implementation and add it to the current flow
    NewFunction,
    /// Flow name changed
    FlowNameChanged(window::Id, String),
    /// Flow version changed
    FlowVersionChanged(window::Id, String),
    /// Flow description changed
    FlowDescriptionChanged(window::Id, String),
    /// Flow authors changed
    FlowAuthorsChanged(window::Id, String),
    /// Toggle metadata editor visibility
    ToggleMetadataEditor(window::Id),
    /// Add a flow-level input port
    FlowAddInput(window::Id),
    /// Add a flow-level output port
    FlowAddOutput(window::Id),
    /// Delete a flow-level input port
    FlowDeleteInput(window::Id, usize),
    /// Delete a flow-level output port
    FlowDeleteOutput(window::Id, usize),
    /// Flow input port name changed
    FlowInputNameChanged(window::Id, usize, String),
    /// Flow input port type changed
    FlowInputTypeChanged(window::Id, usize, String),
    /// Flow output port name changed
    FlowOutputNameChanged(window::Id, usize, String),
    /// Flow output port type changed
    FlowOutputTypeChanged(window::Id, usize, String),
    /// A window close was requested
    CloseRequested(window::Id),
    /// A window was actually closed (cleanup stale state)
    WindowClosed(window::Id),
    /// Close the currently focused window (Cmd+W)
    CloseActiveWindow,
    /// Quit the entire application (Cmd+Q)
    QuitAll,
    /// A window received focus
    WindowFocused(window::Id),
    /// Window was resized — track the new size
    WindowResized(window::Id, iced::Size),
    /// Window was moved — track the new position
    WindowMoved(window::Id, iced::Point),
    /// Add a library search path via file dialog
    AddLibraryPath,
    /// Remove a library search path by index
    RemoveLibraryPath(usize),
    /// Toggle the library paths editor
    ToggleLibPaths,
}

/// State for the initializer editing dialog.
struct InitializerEditor {
    /// Index of the node being edited
    node_index: usize,
    /// Name of the input port being edited
    port_name: String,
    /// Selected type: "none", "once", or "always"
    init_type: String,
    /// The value as a string (JSON)
    value_text: String,
}

/// State for a function definition viewer/editor window.
struct FunctionViewer {
    name: String,
    source_file: String,
    inputs: Vec<PortInfo>,
    outputs: Vec<PortInfo>,
    rs_content: String,
    docs_content: Option<String>,
    active_tab: usize,
    toml_path: PathBuf,
}

/// What kind of content a window displays.
enum WindowKind {
    FlowEditor,
    FunctionViewer(FunctionViewer),
}

/// Per-window state for the flow editor.
struct WindowState {
    /// What this window displays
    kind: WindowKind,
    /// The name of the flow being viewed
    flow_name: String,
    /// Positioned nodes derived from the flow's process references
    nodes: Vec<NodeLayout>,
    /// Connection edges between nodes
    edges: Vec<EdgeLayout>,
    /// Canvas state for caching rendered geometry
    canvas_state: FlowCanvasState,
    /// Status message displayed in the bottom bar
    status: String,
    /// Index of the currently selected node, if any
    selected_node: Option<usize>,
    /// Index of the currently selected connection, if any
    selected_connection: Option<usize>,
    /// Edit history for undo/redo
    history: EditHistory,
    /// Whether auto-fit should be performed on the next opportunity
    auto_fit_pending: bool,
    /// Whether auto-fit mode is active (continuously fits to window)
    auto_fit_enabled: bool,
    /// Count of unsaved edits (increments on edit/redo, decrements on undo)
    unsaved_edits: i32,
    /// Path to the last compiled manifest (None if not compiled or edited since)
    compiled_manifest: Option<PathBuf>,
    /// Path to the currently loaded flow file, if any
    file_path: Option<PathBuf>,
    /// The original flow definition, used to preserve metadata when saving
    flow_definition: FlowDefinition,
    /// Tooltip text and screen position to display (full source path on hover)
    tooltip: Option<(String, f32, f32)>,
    /// Active initializer editor dialog, if any
    initializer_editor: Option<InitializerEditor>,
    /// Whether this is the root (main) window
    is_root: bool,
    /// Flow-level input ports (for sub-flow display)
    flow_inputs: Vec<PortInfo>,
    /// Flow-level output ports (for sub-flow display)
    flow_outputs: Vec<PortInfo>,
    /// Context menu position (screen coords), if showing
    context_menu: Option<(f32, f32)>,
    /// Whether the metadata editor is visible
    show_metadata: bool,
    /// Flow hierarchy tree for this window's navigation panel
    flow_hierarchy: FlowHierarchy,
    /// Last known window size (tracked via resize events)
    last_size: Option<iced::Size>,
    /// Last known window position (tracked via move events)
    last_position: Option<iced::Point>,
}

/// Top-level application state
struct FlowEdit {
    /// Per-window states, keyed by window ID
    windows: HashMap<window::Id, WindowState>,
    /// The ID of the root (main) window, if known
    root_window: Option<window::Id>,
    /// The ID of the currently focused window (updated on focus events)
    focused_window: Option<window::Id>,
    /// Library panel tree for process discovery
    library_tree: LibraryTree,
    /// Path to the root flow file (for rebuilding hierarchy)
    root_flow_path: Option<PathBuf>,
    show_lib_paths: bool,
    lib_paths: Vec<String>,
    /// Cached library manifests, keyed by library root URL (e.g., `lib://flowstdlib`)
    library_cache: HashMap<Url, LibraryManifest>,
    /// Cached parsed definitions for library functions/flows, keyed by lib:// URL
    lib_definitions: HashMap<Url, Process>,
    /// Cached parsed definitions for context functions, keyed by context:// URL
    context_definitions: HashMap<Url, Process>,
}

/// Main entry point for the flowedit binary.
///
/// Parses CLI arguments, loads the flow definition, and launches the iced GUI.
fn main() -> iced::Result {
    env_logger::init();
    iced::daemon(FlowEdit::new, FlowEdit::update, FlowEdit::view)
        .title(FlowEdit::title)
        .subscription(FlowEdit::subscription)
        .antialiasing(true)
        .run()
}

impl FlowEdit {
    /// Create the application, parsing CLI args and optionally loading a flow file.
    fn new() -> (Self, Task<Message>) {
        let matches = ClapCommand::new("flowedit")
            .version(env!("CARGO_PKG_VERSION"))
            .about("Visual editor for flow definition files")
            .arg(
                Arg::new("flow-file")
                    .required(false)
                    .help("Path to the flow definition file (.toml, .yaml, or .json)"),
            )
            .arg(
                Arg::new("lib_dir")
                    .short('L')
                    .long("libdir")
                    .num_args(1)
                    .action(ArgAction::Append)
                    .value_name("LIB_DIR")
                    .help("Add a directory to the Library Search path"),
            )
            .get_matches();

        // Collect -L library directories, same as flowrgui
        let lib_dirs: Vec<String> = if matches.contains_id("lib_dir") {
            matches
                .get_many::<String>("lib_dir")
                .map(|dirs| dirs.map(std::string::ToString::to_string).collect())
                .unwrap_or_default()
        } else {
            vec![]
        };

        // Build the library search path from FLOW_LIB_PATH + -L args
        let mut lib_search_path = Simpath::new_with_separator("FLOW_LIB_PATH", ',');
        for addition in &lib_dirs {
            lib_search_path.add(addition);
            info!("'{addition}' added to the Library Search Path");
        }
        if lib_search_path.is_empty() {
            if let Ok(home) = std::env::var("HOME") {
                lib_search_path.add(&format!("{home}/.flow/lib"));
            }
        }

        // Set FLOW_LIB_PATH with any -L additions so other code can find libraries
        if !lib_dirs.is_empty() {
            let current = std::env::var("FLOW_LIB_PATH").unwrap_or_default();
            let additions = lib_dirs.join(",");
            if current.is_empty() {
                std::env::set_var("FLOW_LIB_PATH", additions);
            } else {
                std::env::set_var("FLOW_LIB_PATH", format!("{current},{additions}"));
            }
        }

        let (flow_name, nodes, edges, status, file_path, flow_definition, lib_refs, ctx_refs) =
            if let Some(flow_path_str) = matches.get_one::<String>("flow-file") {
                let flow_path = PathBuf::from(flow_path_str);
                match load_flow(&flow_path) {
                    Ok(loaded) => {
                        let nc = loaded.nodes.len();
                        let ec = loaded.edges.len();
                        (
                            loaded.name,
                            loaded.nodes,
                            loaded.edges,
                            format!("Ready - {nc} nodes, {ec} connections"),
                            Some(flow_path),
                            loaded.flow_def,
                            loaded.lib_references,
                            loaded.context_references,
                        )
                    }
                    Err(e) => (
                        String::from("(error)"),
                        Vec::new(),
                        Vec::new(),
                        format!("Error loading flow: {e}"),
                        None,
                        FlowDefinition::default(),
                        BTreeSet::new(),
                        BTreeSet::new(),
                    ),
                }
            } else {
                (
                    String::from("(new flow)"),
                    Vec::new(),
                    Vec::new(),
                    String::from("Ready"),
                    None,
                    FlowDefinition::default(),
                    BTreeSet::new(),
                    BTreeSet::new(),
                )
            };

        let has_nodes = !nodes.is_empty();

        // Load full library catalogs from manifests and parse all definitions
        let (library_cache, lib_definitions, context_definitions) =
            load_library_catalogs(&lib_refs, &ctx_refs);
        let library_tree =
            LibraryTree::from_cache(&library_cache, &lib_definitions, &context_definitions);

        // Open the root window via daemon API
        let saved_prefs = file_path.as_ref().and_then(|p| load_editor_prefs(p));
        let saved_size = saved_prefs
            .as_ref()
            .map(|p| iced::Size::new(p.width, p.height))
            .unwrap_or_else(|| iced::Size::new(1024.0, 768.0));
        let saved_position = saved_prefs
            .as_ref()
            .and_then(|p| match (p.x, p.y) {
                (Some(x), Some(y)) => Some(window::Position::Specific(iced::Point::new(x, y))),
                _ => None,
            })
            .unwrap_or(window::Position::Default);
        let (root_id, open_task) = window::open(window::Settings {
            size: saved_size,
            position: saved_position,
            ..Default::default()
        });

        let root_flow_path = file_path.clone();
        let flow_hierarchy = file_path
            .as_ref()
            .map(|p| FlowHierarchy::build(p))
            .unwrap_or_else(FlowHierarchy::empty);

        let (fi, fo) = extract_ports(&flow_definition.inputs, &flow_definition.outputs);
        let win_state = WindowState {
            kind: WindowKind::FlowEditor,
            flow_name,
            nodes,
            edges,
            canvas_state: FlowCanvasState::default(),
            status,
            selected_node: None,
            selected_connection: None,
            auto_fit_pending: has_nodes,
            auto_fit_enabled: true,
            history: EditHistory::default(),
            unsaved_edits: 0,
            compiled_manifest: None,
            file_path,
            flow_definition,
            tooltip: None,
            initializer_editor: None,
            is_root: true,
            flow_inputs: fi,
            flow_outputs: fo,
            context_menu: None,
            show_metadata: false,
            flow_hierarchy,
            last_size: None,
            last_position: None,
        };

        let mut windows = HashMap::new();
        windows.insert(root_id, win_state);

        let lib_paths = resolve_lib_paths();
        let app = FlowEdit {
            windows,
            root_window: Some(root_id),
            focused_window: Some(root_id),
            library_tree,
            root_flow_path,
            show_lib_paths: false,
            lib_paths,
            library_cache,
            lib_definitions,
            context_definitions,
        };

        (app, open_task.discard())
    }

    /// Return the window title, showing the flow name, file name, and unsaved indicator.
    fn title(&self, window_id: window::Id) -> String {
        if let Some(win) = self.windows.get(&window_id) {
            let modified = if win.unsaved_edits > 0 { " *" } else { "" };
            let file = win
                .file_path
                .as_ref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("untitled");
            format!("flowedit - {} ({}){modified}", win.flow_name, file)
        } else {
            String::from("flowedit")
        }
    }

    /// Handle messages from canvas interactions.
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WindowCanvas(win_id, canvas_msg) => {
                let Some(win) = self.windows.get_mut(&win_id) else {
                    return Task::none();
                };
                match canvas_msg {
                    CanvasMessage::Selected(idx) => {
                        win.selected_node = idx;
                        win.context_menu = None;
                        if win.selected_connection.is_some() {
                            win.selected_connection = None;
                            win.canvas_state.request_redraw();
                        }
                        if let Some(i) = idx {
                            if let Some(node) = win.nodes.get(i) {
                                win.status = format!("Selected: {}", node.alias);
                            }
                        } else {
                            win.status = String::from("Ready");
                        }
                    }
                    CanvasMessage::Moved(idx, x, y) => {
                        if let Some(node) = win.nodes.get_mut(idx) {
                            node.x = x;
                            node.y = y;
                            win.canvas_state.request_redraw();
                        }
                    }
                    CanvasMessage::Resized(idx, x, y, w, h) => {
                        if let Some(node) = win.nodes.get_mut(idx) {
                            node.x = x;
                            node.y = y;
                            node.width = w;
                            node.height = h;
                            win.canvas_state.request_redraw();
                        }
                    }
                    CanvasMessage::MoveCompleted(idx, old_x, old_y, new_x, new_y) => {
                        info!("MoveCompleted: idx={idx}, ({old_x},{old_y}) -> ({new_x},{new_y})");
                        if (old_x - new_x).abs() > 0.5 || (old_y - new_y).abs() > 0.5 {
                            record_edit(
                                win,
                                EditAction::MoveNode {
                                    index: idx,
                                    old_x,
                                    old_y,
                                    new_x,
                                    new_y,
                                },
                            );
                        }
                    }
                    #[allow(clippy::similar_names)]
                    CanvasMessage::ResizeCompleted(
                        idx,
                        old_x,
                        old_y,
                        old_w,
                        old_h,
                        new_x,
                        new_y,
                        new_w,
                        new_h,
                    ) => {
                        record_edit(
                            win,
                            EditAction::ResizeNode {
                                index: idx,
                                old_x,
                                old_y,
                                old_w,
                                old_h,
                                new_x,
                                new_y,
                                new_w,
                                new_h,
                            },
                        );
                    }
                    CanvasMessage::Deleted(idx) => {
                        if idx < win.nodes.len() {
                            let node = if let Some(node) = win.nodes.get(idx) {
                                node.clone()
                            } else {
                                return Task::none();
                            };
                            let alias = node.alias.clone();
                            let removed_edges: Vec<EdgeLayout> = win
                                .edges
                                .iter()
                                .filter(|e| e.references_node(&alias))
                                .cloned()
                                .collect();
                            win.nodes.remove(idx);
                            win.edges.retain(|e| !e.references_node(&alias));
                            record_edit(
                                win,
                                EditAction::DeleteNode {
                                    index: idx,
                                    node,
                                    removed_edges,
                                },
                            );
                            win.selected_node = None;
                            win.selected_connection = None;
                            win.canvas_state.request_redraw();
                            let nc = win.nodes.len();
                            let ec = win.edges.len();
                            win.status = format!("Node deleted - {nc} nodes, {ec} connections");
                            if win.auto_fit_enabled {
                                win.auto_fit_pending = true;
                            }
                        }
                    }
                    CanvasMessage::ConnectionCreated {
                        from_node,
                        from_port,
                        to_node,
                        to_port,
                    } => {
                        let edge = EdgeLayout::new(
                            from_node.clone(),
                            from_port.clone(),
                            to_node.clone(),
                            to_port.clone(),
                        );
                        record_edit(win, EditAction::CreateConnection { edge: edge.clone() });
                        win.edges.push(edge);
                        win.canvas_state.request_redraw();
                        let nc = win.nodes.len();
                        let ec = win.edges.len();
                        win.status = format!(
                            "Connection created: {from_node}/{from_port} -> {to_node}/{to_port} - {nc} nodes, {ec} connections"
                        );
                    }
                    CanvasMessage::ConnectionSelected(idx) => {
                        win.selected_connection = idx;
                        win.selected_node = None;
                        win.canvas_state.request_redraw();
                        if let Some(i) = idx {
                            if let Some(edge) = win.edges.get(i) {
                                win.status = format!(
                                    "Connection: {} -> {}",
                                    format_endpoint(&edge.from_node, &edge.from_port),
                                    format_endpoint(&edge.to_node, &edge.to_port),
                                );
                            }
                        } else {
                            win.status = String::from("Ready");
                        }
                    }
                    CanvasMessage::ConnectionDeleted(idx) => {
                        if idx < win.edges.len() {
                            let edge = win.edges.remove(idx);
                            record_edit(win, EditAction::DeleteConnection { index: idx, edge });
                            win.selected_connection = None;
                            win.canvas_state.request_redraw();
                            let nc = win.nodes.len();
                            let ec = win.edges.len();
                            win.status =
                                format!("Connection deleted - {nc} nodes, {ec} connections");
                        }
                    }
                    CanvasMessage::HoverChanged(data) => {
                        win.tooltip = data;
                    }
                    CanvasMessage::AutoFitViewport(viewport) => {
                        if win.auto_fit_enabled || win.auto_fit_pending {
                            let has_flow_io =
                                !win.flow_inputs.is_empty() || !win.flow_outputs.is_empty();
                            win.canvas_state.auto_fit(&win.nodes, has_flow_io, viewport);
                            win.auto_fit_pending = false;
                        }
                    }
                    CanvasMessage::Pan(dx, dy) => {
                        win.auto_fit_enabled = false; // Manual pan disables auto-fit
                        win.canvas_state.scroll_offset.x += dx;
                        win.canvas_state.scroll_offset.y += dy;
                        win.canvas_state.request_redraw();
                    }
                    CanvasMessage::ZoomBy(factor) => {
                        win.auto_fit_enabled = false; // Manual zoom disables auto-fit
                        win.canvas_state.zoom = (win.canvas_state.zoom * factor).clamp(0.1, 5.0);
                        win.canvas_state.request_redraw();
                        let pct = (win.canvas_state.zoom * 100.0) as u32;
                        win.status = format!("Zoom: {pct}%");
                    }
                    CanvasMessage::InitializerEdit(node_idx, port_name) => {
                        // Look up current initializer from the model (flow definition)
                        let alias = win
                            .nodes
                            .get(node_idx)
                            .map(|n| n.alias.clone())
                            .unwrap_or_default();
                        let (init_type, value_text) = win
                            .flow_definition
                            .process_refs
                            .iter()
                            .find(|pr| {
                                let pr_alias = if pr.alias.is_empty() {
                                    derive_short_name(&pr.source)
                                } else {
                                    pr.alias.to_string()
                                };
                                pr_alias == alias
                            })
                            .and_then(|pr| pr.initializations.get(&port_name))
                            .map(|init| match init {
                                InputInitializer::Once(v) => (
                                    "once".to_string(),
                                    serde_json::to_string(v).unwrap_or_default(),
                                ),
                                InputInitializer::Always(v) => (
                                    "always".to_string(),
                                    serde_json::to_string(v).unwrap_or_default(),
                                ),
                            })
                            .unwrap_or_else(|| ("none".to_string(), String::new()));

                        win.initializer_editor = Some(InitializerEditor {
                            node_index: node_idx,
                            port_name,
                            init_type,
                            value_text,
                        });
                    }
                    CanvasMessage::OpenNode(idx) => {
                        return self.open_node(win_id, idx);
                    }
                    CanvasMessage::ContextMenu(x, y) => {
                        if let Some(win) = self.windows.get_mut(&win_id) {
                            win.context_menu = Some((x, y));
                        }
                    }
                }
            }
            Message::Hierarchy(hier_win_id, ref hier_msg) => {
                let open_result = self
                    .windows
                    .get_mut(&hier_win_id)
                    .and_then(|win| win.flow_hierarchy.update(hier_msg));
                if let Some((_source, path)) = open_result {
                    // Check if already open
                    for (&win_id, win) in &self.windows {
                        if win.file_path.as_ref() == Some(&path) {
                            return window::gain_focus(win_id);
                        }
                    }
                    // Open the flow or function
                    match load_flow(&path) {
                        Ok(loaded) => {
                            let (fi, fo) =
                                extract_ports(&loaded.flow_def.inputs, &loaded.flow_def.outputs);
                            let (new_id, open_task) =
                                window::open(self.child_window_settings(1024.0, 768.0));
                            let has_nodes = !loaded.nodes.is_empty();
                            let nc = loaded.nodes.len();
                            let ec = loaded.edges.len();
                            let child = WindowState {
                                kind: WindowKind::FlowEditor,
                                flow_name: loaded.name,
                                nodes: loaded.nodes,
                                edges: loaded.edges,
                                canvas_state: FlowCanvasState::default(),
                                status: format!("Ready - {nc} nodes, {ec} connections"),
                                selected_node: None,
                                selected_connection: None,
                                history: EditHistory::default(),
                                auto_fit_pending: has_nodes,
                                auto_fit_enabled: true,
                                unsaved_edits: 0,
                                compiled_manifest: None,
                                file_path: Some(path),
                                flow_definition: loaded.flow_def,
                                tooltip: None,
                                initializer_editor: None,
                                is_root: false,
                                flow_inputs: fi,
                                flow_outputs: fo,
                                context_menu: None,
                                show_metadata: false,
                                flow_hierarchy: self.build_hierarchy(),
                                last_size: None,
                                last_position: None,
                            };
                            self.windows.insert(new_id, child);
                            return open_task.discard();
                        }
                        Err(_) => {
                            // Try as function definition
                            let abs = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
                            if let Ok(contents) = std::fs::read_to_string(&abs) {
                                if let Ok(url) = Url::from_file_path(&abs) {
                                    if let Ok(deser) = get::<Process>(&url) {
                                        if let Ok(Process::FunctionProcess(ref func)) =
                                            deser.deserialize(&contents, Some(&url))
                                        {
                                            return self.open_function_viewer(
                                                hier_win_id,
                                                &path,
                                                func,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Message::Library(win_id, ref lib_msg) => match self.library_tree.update(lib_msg) {
                LibraryAction::Add(source, func_name) => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        add_library_function(win, &source, &func_name);
                    }
                }
                LibraryAction::View(source, _name) => {
                    return self.open_library_function(&source);
                }
                LibraryAction::None => {}
            },
            Message::ZoomIn(win_id) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.auto_fit_enabled = false;
                    win.canvas_state.zoom_in();
                    let pct = (win.canvas_state.zoom * 100.0) as u32;
                    win.status = format!("Zoom: {pct}%");
                }
            }
            Message::ZoomOut(win_id) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.auto_fit_enabled = false;
                    win.canvas_state.zoom_out();
                    let pct = (win.canvas_state.zoom * 100.0) as u32;
                    win.status = format!("Zoom: {pct}%");
                }
            }
            Message::ToggleAutoFit(win_id) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.auto_fit_enabled = !win.auto_fit_enabled;
                    if win.auto_fit_enabled {
                        win.auto_fit_pending = true;
                        win.canvas_state.request_redraw();
                        win.status = String::from("Auto-fit enabled");
                    } else {
                        win.status = String::from("Auto-fit disabled");
                    }
                }
            }
            Message::Undo => {
                let target = self.focused_window.or(self.root_window);
                if let Some(win) = target.and_then(|id| self.windows.get_mut(&id)) {
                    apply_undo(win);
                    win.unsaved_edits = (win.unsaved_edits - 1).max(0);
                }
            }
            Message::Redo => {
                let target = self.focused_window.or(self.root_window);
                if let Some(win) = target.and_then(|id| self.windows.get_mut(&id)) {
                    apply_redo(win);
                    win.unsaved_edits += 1;
                }
            }
            Message::Save => {
                let target = self.focused_window.or(self.root_window);
                if let Some(win) = target.and_then(|id| self.windows.get_mut(&id)) {
                    if let Some(path) = win.file_path.clone() {
                        perform_save(win, &path);
                    } else {
                        perform_save_as(win);
                    }
                }
            }
            Message::SaveAs => {
                let target = self.focused_window.or(self.root_window);
                if let Some(win) = target.and_then(|id| self.windows.get_mut(&id)) {
                    perform_save_as(win);
                }
            }
            Message::Open => {
                if let Some(root_id) = self.root_window {
                    if let Some(win) = self.windows.get_mut(&root_id) {
                        if let Some((lib_refs, ctx_refs)) = perform_open(win) {
                            self.root_flow_path = win.file_path.clone();
                            win.flow_hierarchy = win
                                .file_path
                                .as_ref()
                                .map(|p| FlowHierarchy::build(p))
                                .unwrap_or_else(FlowHierarchy::empty);

                            // Rebuild library cache with new flow's references
                            let (lc, ld, cd) = load_library_catalogs(&lib_refs, &ctx_refs);
                            self.library_cache = lc;
                            self.lib_definitions = ld;
                            self.context_definitions = cd;
                            self.library_tree = LibraryTree::from_cache(
                                &self.library_cache,
                                &self.lib_definitions,
                                &self.context_definitions,
                            );
                        }
                    }
                }
            }
            Message::New => {
                if let Some(win) = self.root_window.and_then(|id| self.windows.get_mut(&id)) {
                    perform_new(win);
                    // Clear the library cache for a new (empty) flow
                    self.library_cache.clear();
                    self.lib_definitions.clear();
                    self.context_definitions.clear();
                    self.library_tree = LibraryTree::from_cache(
                        &self.library_cache,
                        &self.lib_definitions,
                        &self.context_definitions,
                    );
                }
            }
            Message::Compile => {
                let target = self.focused_window.or(self.root_window);
                if let Some(win) = target.and_then(|id| self.windows.get_mut(&id)) {
                    if !win.nodes.is_empty() {
                        match perform_compile(win) {
                            Ok(path) => {
                                win.compiled_manifest = Some(path.clone());
                                win.status = format!("Compiled: {}", path.display());
                            }
                            Err(e) => {
                                win.compiled_manifest = None;
                                win.status = e.to_string();
                            }
                        }
                    }
                }
            }
            Message::FlowNameChanged(win_id, new_name) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.flow_name = new_name.clone();
                    win.flow_definition.name = new_name;
                    win.unsaved_edits += 1;
                }
            }
            Message::FlowVersionChanged(win_id, version) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.flow_definition.metadata.version = version;
                    win.unsaved_edits += 1;
                }
            }
            Message::FlowDescriptionChanged(win_id, desc) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.flow_definition.metadata.description = desc;
                    win.unsaved_edits += 1;
                }
            }
            Message::FlowAuthorsChanged(win_id, authors_str) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.flow_definition.metadata.authors = authors_str
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    win.unsaved_edits += 1;
                }
            }
            Message::ToggleMetadataEditor(win_id) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.show_metadata = !win.show_metadata;
                }
            }
            Message::NewSubFlow => {
                for win in self.windows.values_mut() {
                    win.context_menu = None;
                }
                return self.create_new_subflow();
            }
            Message::NewFunction => {
                for win in self.windows.values_mut() {
                    win.context_menu = None;
                }
                return self.create_new_function();
            }
            Message::InitializerTypeChanged(win_id, new_type) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    if let Some(ref mut editor) = win.initializer_editor {
                        editor.init_type = new_type;
                    }
                }
            }
            Message::InitializerValueChanged(win_id, new_value) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    if let Some(ref mut editor) = win.initializer_editor {
                        editor.value_text = new_value;
                    }
                }
            }
            Message::InitializerApply(win_id) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    if let Some(editor) = win.initializer_editor.take() {
                        apply_initializer_edit(win, &editor);
                    }
                }
            }
            Message::InitializerCancel(win_id) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.initializer_editor = None;
                }
            }
            Message::WindowFocused(id) => {
                self.focused_window = Some(id);
            }
            Message::WindowResized(id, size) => {
                if let Some(win) = self.windows.get_mut(&id) {
                    win.last_size = Some(size);
                }
            }
            Message::WindowMoved(id, pos) => {
                if let Some(win) = self.windows.get_mut(&id) {
                    win.last_position = Some(pos);
                }
            }
            Message::AddLibraryPath => {
                let dialog = rfd::FileDialog::new();
                if let Some(dir) = dialog.pick_folder() {
                    let path_str = dir.to_string_lossy().to_string();
                    if !self.lib_paths.contains(&path_str) {
                        self.lib_paths.push(path_str);
                        self.update_lib_paths();
                    }
                }
            }
            Message::RemoveLibraryPath(idx) => {
                if idx < self.lib_paths.len() {
                    self.lib_paths.remove(idx);
                    self.update_lib_paths();
                }
            }
            Message::ToggleLibPaths => {
                self.show_lib_paths = !self.show_lib_paths;
            }
            Message::FunctionTabSelected(win_id, tab) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    if let WindowKind::FunctionViewer(ref mut viewer) = win.kind {
                        viewer.active_tab = tab;
                    }
                }
            }
            Message::FunctionNameChanged(win_id, new_name) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    if let WindowKind::FunctionViewer(ref mut viewer) = win.kind {
                        viewer.name = new_name;
                    }
                    win.unsaved_edits += 1;
                }
            }
            Message::FunctionBrowseSource(win_id) => {
                let dialog = rfd::FileDialog::new().add_filter("Rust", &["rs"]);
                if let Some(selected) = dialog.pick_file() {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        if let WindowKind::FunctionViewer(ref mut viewer) = win.kind {
                            let base = viewer.toml_path.parent().unwrap_or(Path::new("."));
                            let rel = selected
                                .strip_prefix(base)
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_else(|_| selected.to_string_lossy().to_string());
                            viewer.source_file = rel;
                            viewer.rs_content = std::fs::read_to_string(&selected)
                                .unwrap_or_else(|_| String::from("// Could not read file"));
                        }
                        win.unsaved_edits += 1;
                    }
                }
            }
            Message::FunctionAddInput(win_id)
            | Message::FunctionAddOutput(win_id)
            | Message::FunctionDeleteInput(win_id, _)
            | Message::FunctionDeleteOutput(win_id, _) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    if let WindowKind::FunctionViewer(ref mut v) = win.kind {
                        match message {
                            Message::FunctionAddInput(_) => v.inputs.push(PortInfo {
                                name: format!("input{}", v.inputs.len()),
                                datatypes: vec![String::from("string")],
                            }),
                            Message::FunctionAddOutput(_) => v.outputs.push(PortInfo {
                                name: format!("output{}", v.outputs.len()),
                                datatypes: vec![String::from("string")],
                            }),
                            Message::FunctionDeleteInput(_, idx) if idx < v.inputs.len() => {
                                v.inputs.remove(idx);
                            }
                            Message::FunctionDeleteOutput(_, idx) if idx < v.outputs.len() => {
                                v.outputs.remove(idx);
                            }
                            _ => {}
                        }
                    }
                    win.unsaved_edits += 1;
                }
            }
            Message::FunctionInputNameChanged(win_id, idx, name) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    if let WindowKind::FunctionViewer(ref mut v) = win.kind {
                        if let Some(port) = v.inputs.get_mut(idx) {
                            port.name = name;
                        }
                    }
                    win.unsaved_edits += 1;
                }
            }
            Message::FunctionInputTypeChanged(win_id, idx, dtype) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    if let WindowKind::FunctionViewer(ref mut v) = win.kind {
                        if let Some(port) = v.inputs.get_mut(idx) {
                            port.datatypes = vec![dtype];
                        }
                    }
                    win.unsaved_edits += 1;
                }
            }
            Message::FunctionOutputNameChanged(win_id, idx, name) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    if let WindowKind::FunctionViewer(ref mut v) = win.kind {
                        if let Some(port) = v.outputs.get_mut(idx) {
                            port.name = name;
                        }
                    }
                    win.unsaved_edits += 1;
                }
            }
            Message::FunctionOutputTypeChanged(win_id, idx, dtype) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    if let WindowKind::FunctionViewer(ref mut v) = win.kind {
                        if let Some(port) = v.outputs.get_mut(idx) {
                            port.datatypes = vec![dtype];
                        }
                    }
                    win.unsaved_edits += 1;
                }
            }
            Message::FunctionSave(win_id) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    if let WindowKind::FunctionViewer(ref v) = win.kind {
                        match save_function_definition(v) {
                            Ok(()) => {
                                win.status = format!("Saved: {}", v.toml_path.display());
                                win.unsaved_edits = 0;
                            }
                            Err(e) => {
                                win.status = format!("Save failed: {e}");
                            }
                        }
                    }
                }
            }
            Message::FlowAddInput(win_id) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.flow_inputs.push(PortInfo {
                        name: format!("input{}", win.flow_inputs.len()),
                        datatypes: vec![String::from("string")],
                    });
                    win.unsaved_edits += 1;
                    win.canvas_state.request_redraw();
                }
            }
            Message::FlowAddOutput(win_id) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.flow_outputs.push(PortInfo {
                        name: format!("output{}", win.flow_outputs.len()),
                        datatypes: vec![String::from("string")],
                    });
                    win.unsaved_edits += 1;
                    win.canvas_state.request_redraw();
                }
            }
            Message::FlowDeleteInput(win_id, idx) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    if idx < win.flow_inputs.len() {
                        win.flow_inputs.remove(idx);
                        win.unsaved_edits += 1;
                        win.canvas_state.request_redraw();
                    }
                }
            }
            Message::FlowDeleteOutput(win_id, idx) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    if idx < win.flow_outputs.len() {
                        win.flow_outputs.remove(idx);
                        win.unsaved_edits += 1;
                        win.canvas_state.request_redraw();
                    }
                }
            }
            Message::FlowInputNameChanged(win_id, idx, name) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    if let Some(port) = win.flow_inputs.get_mut(idx) {
                        port.name = name;
                    }
                    win.unsaved_edits += 1;
                    win.canvas_state.request_redraw();
                }
            }
            Message::FlowInputTypeChanged(win_id, idx, dtype) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    if let Some(port) = win.flow_inputs.get_mut(idx) {
                        port.datatypes = vec![dtype];
                    }
                    win.unsaved_edits += 1;
                    win.canvas_state.request_redraw();
                }
            }
            Message::FlowOutputNameChanged(win_id, idx, name) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    if let Some(port) = win.flow_outputs.get_mut(idx) {
                        port.name = name;
                    }
                    win.unsaved_edits += 1;
                    win.canvas_state.request_redraw();
                }
            }
            Message::FlowOutputTypeChanged(win_id, idx, dtype) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    if let Some(port) = win.flow_outputs.get_mut(idx) {
                        port.datatypes = vec![dtype];
                    }
                    win.unsaved_edits += 1;
                    win.canvas_state.request_redraw();
                }
            }
            Message::WindowClosed(id) => {
                self.windows.remove(&id);
                if self.focused_window == Some(id) {
                    self.focused_window = self.root_window;
                }
                if self.root_window == Some(id) || self.windows.is_empty() {
                    return iced::exit();
                }
            }
            Message::CloseRequested(_) | Message::CloseActiveWindow => {
                let target = match message {
                    Message::CloseRequested(win_id) => Some(win_id),
                    Message::CloseActiveWindow => self.focused_window.or(self.root_window),
                    _ => None,
                };
                let Some(id) = target else {
                    return Task::none();
                };
                if let Some(win) = self.windows.get(&id) {
                    if win.unsaved_edits > 0 {
                        let dialog = rfd::MessageDialog::new()
                            .set_title("Unsaved Changes")
                            .set_description(
                                "This window has unsaved changes. Close without saving?",
                            )
                            .set_buttons(rfd::MessageButtons::YesNo)
                            .set_level(rfd::MessageLevel::Warning);
                        if dialog.show() != rfd::MessageDialogResult::Yes {
                            return Task::none();
                        }
                    }
                }
                self.windows.remove(&id);
                if self.root_window == Some(id) || self.windows.is_empty() {
                    return iced::exit();
                }
                return window::close(id);
            }
            Message::QuitAll => {
                // Check for unsaved edits in any window
                let has_unsaved = self.windows.values().any(|w| w.unsaved_edits > 0);
                if has_unsaved {
                    let dialog = rfd::MessageDialog::new()
                        .set_title("Unsaved Changes")
                        .set_description("There are unsaved changes. Quit without saving?")
                        .set_buttons(rfd::MessageButtons::YesNo)
                        .set_level(rfd::MessageLevel::Warning);
                    if dialog.show() != rfd::MessageDialogResult::Yes {
                        return Task::none();
                    }
                }
                return iced::exit();
            }
        }
        Task::none()
    }

    /// Build the view for a window, dispatching based on window kind.
    fn view(&self, window_id: window::Id) -> Element<'_, Message> {
        let Some(win) = self.windows.get(&window_id) else {
            return Text::new("Window not found").into();
        };

        if let WindowKind::FunctionViewer(ref viewer) = win.kind {
            return self.view_function(window_id, viewer, &win.status);
        }

        let canvas = win
            .canvas_state
            .view(
                &win.nodes,
                &win.edges,
                &win.flow_name,
                &win.flow_inputs,
                &win.flow_outputs,
                !win.is_root,
                win.auto_fit_pending,
                win.auto_fit_enabled,
            )
            .map(move |msg| Message::WindowCanvas(window_id, msg));

        let zoom_btn = |_theme: &Theme, status: button::Status| -> button::Style {
            let is_hovered = matches!(status, button::Status::Hovered);
            button::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.25, 0.25, 0.30))),
                text_color: Color::WHITE,
                border: iced::Border {
                    color: if is_hovered {
                        Color::WHITE
                    } else {
                        Color::from_rgb(0.4, 0.4, 0.45)
                    },
                    width: 2.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            }
        };
        let zoom_btn_active = |_theme: &Theme, status: button::Status| -> button::Style {
            let is_hovered = matches!(status, button::Status::Hovered);
            button::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.3, 0.35, 0.5))),
                text_color: Color::WHITE,
                border: iced::Border {
                    color: if is_hovered {
                        Color::WHITE
                    } else {
                        Color::from_rgb(0.4, 0.5, 0.7)
                    },
                    width: 2.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            }
        };
        let btn_width = 40;
        let zoom_controls = container(
            Column::new()
                .spacing(4)
                .push(
                    button(Text::new("+").center())
                        .on_press(Message::ZoomIn(window_id))
                        .width(btn_width)
                        .style(zoom_btn),
                )
                .push(
                    button(Text::new("\u{2212}").center())
                        .on_press(Message::ZoomOut(window_id))
                        .width(btn_width)
                        .style(zoom_btn),
                )
                .push(
                    button(Text::new("Fit").center())
                        .on_press(Message::ToggleAutoFit(window_id))
                        .width(btn_width)
                        .style(if win.auto_fit_enabled {
                            zoom_btn_active
                        } else {
                            zoom_btn
                        }),
                ),
        )
        .align_right(Fill)
        .align_bottom(Fill)
        .padding(10);

        let mut canvas_stack: Vec<Element<'_, Message>> = vec![canvas, zoom_controls.into()];

        if let Some((ref tip_text, tx, ty)) = win.tooltip {
            let tooltip_widget = container(
                container(Text::new(tip_text.clone()).size(20).color(Color::WHITE))
                    .padding(8)
                    .style(|_theme: &Theme| container::Style {
                        background: Some(iced::Background::Color(Color::from_rgb(
                            0.12, 0.12, 0.12,
                        ))),
                        border: iced::Border {
                            color: Color::WHITE,
                            width: 1.0,
                            radius: 6.0.into(),
                        },
                        ..Default::default()
                    }),
            )
            .padding(iced::Padding {
                top: ty + 6.0,
                right: 0.0,
                bottom: 0.0,
                left: (tx - 80.0).max(0.0),
            });
            canvas_stack.push(tooltip_widget.into());
        }

        // Initializer editor dialog overlay
        if let Some(ref editor) = win.initializer_editor {
            let port_label = if let Some(node) = win.nodes.get(editor.node_index) {
                format!("{}/{}", node.alias, editor.port_name)
            } else {
                editor.port_name.clone()
            };

            let init_types = vec!["none", "once", "always"];
            let selected: Option<&str> =
                init_types.iter().find(|&&t| t == editor.init_type).copied();

            let mut dialog_col = Column::new()
                .spacing(8)
                .padding(12)
                .push(Text::new(format!("Initializer: {port_label}")).size(14))
                .push(
                    pick_list(init_types, selected, move |s: &str| {
                        Message::InitializerTypeChanged(window_id, s.to_string())
                    })
                    .text_size(12),
                );

            if editor.init_type != "none" {
                dialog_col = dialog_col.push(
                    text_input("JSON value (e.g. 42, \"hello\", true)", &editor.value_text)
                        .on_input(move |v| Message::InitializerValueChanged(window_id, v))
                        .size(12)
                        .padding(6),
                );
            }

            dialog_col = dialog_col.push(
                Row::new()
                    .spacing(8)
                    .push(
                        button(Text::new("Apply").size(12).center())
                            .on_press(Message::InitializerApply(window_id))
                            .style(button::primary)
                            .padding(6),
                    )
                    .push(
                        button(Text::new("Cancel").size(12).center())
                            .on_press(Message::InitializerCancel(window_id))
                            .style(button::secondary)
                            .padding(6),
                    ),
            );

            let dialog = container(container(dialog_col).width(280).style(|_theme: &Theme| {
                container::Style {
                    background: Some(iced::Background::Color(Color::from_rgb(0.15, 0.15, 0.15))),
                    border: iced::Border {
                        color: Color::from_rgb(0.4, 0.4, 0.4),
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                }
            }))
            .center(Fill);

            canvas_stack.push(dialog.into());
        }

        // Context menu overlay (right-click on empty canvas)
        if let Some((cx, cy)) = win.context_menu {
            let menu = container(
                Column::new()
                    .spacing(2)
                    .push(
                        button(Text::new("+ New Sub-flow").size(13))
                            .on_press(Message::NewSubFlow)
                            .style(button::text)
                            .padding([6, 16])
                            .width(Fill),
                    )
                    .push(
                        button(Text::new("+ New Function").size(13))
                            .on_press(Message::NewFunction)
                            .style(button::text)
                            .padding([6, 16])
                            .width(Fill),
                    ),
            )
            .style(|_theme: &Theme| container::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.18, 0.18, 0.22))),
                border: iced::Border {
                    color: Color::from_rgb(0.4, 0.4, 0.4),
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            })
            .width(160)
            .padding(4);

            let positioned = container(menu).padding(iced::Padding {
                top: cy,
                left: cx,
                right: 0.0,
                bottom: 0.0,
            });
            canvas_stack.push(positioned.into());
        }

        let canvas_with_controls = stack(canvas_stack);

        let hierarchy_panel = win
            .flow_hierarchy
            .view()
            .map(move |msg| Message::Hierarchy(window_id, msg));

        let library_panel = self
            .library_tree
            .view()
            .map(move |msg| Message::Library(window_id, msg));

        let left_panel = Column::new()
            .push(hierarchy_panel)
            .push(library_panel)
            .height(Fill);

        let mut right_col: Column<'_, Message> =
            Column::new().push(container(canvas_with_controls).width(Fill).height(Fill));

        let edit_indicator = if win.unsaved_edits > 0 {
            format!("  |  {} unsaved edit(s)", win.unsaved_edits)
        } else {
            String::from("  |  saved")
        };

        // Build status bar — action buttons only for root windows
        let btn_pad = [6, 14];
        let btn_size = 13;
        let toolbar_btn = |_theme: &Theme, status: button::Status| -> button::Style {
            let is_hovered = matches!(status, button::Status::Hovered);
            button::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.25, 0.25, 0.30))),
                text_color: Color::WHITE,
                border: iced::Border {
                    color: if is_hovered {
                        Color::WHITE
                    } else {
                        Color::from_rgb(0.4, 0.4, 0.45)
                    },
                    width: 2.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            }
        };
        let toolbar_btn_active = |_theme: &Theme, status: button::Status| -> button::Style {
            let is_hovered = matches!(status, button::Status::Hovered);
            button::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.3, 0.35, 0.5))),
                text_color: Color::WHITE,
                border: iced::Border {
                    color: if is_hovered {
                        Color::WHITE
                    } else {
                        Color::from_rgb(0.4, 0.5, 0.7)
                    },
                    width: 2.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            }
        };
        let status_bar: Row<'_, Message> = if win.is_root {
            let mut compile_btn = button(Text::new("\u{1F528} Build").size(btn_size).center())
                .padding(btn_pad)
                .style(toolbar_btn);
            if !win.nodes.is_empty() {
                compile_btn = compile_btn.on_press(Message::Compile);
            }

            let new_subflow_btn = button(Text::new("+ Sub-flow").size(btn_size).center())
                .on_press(Message::NewSubFlow)
                .style(toolbar_btn)
                .padding(btn_pad);

            let new_func_btn = button(Text::new("+ Function").size(btn_size).center())
                .on_press(Message::NewFunction)
                .style(toolbar_btn)
                .padding(btn_pad);

            let info_btn = button(Text::new("\u{2139} Info").size(btn_size).center())
                .on_press(Message::ToggleMetadataEditor(window_id))
                .style(if win.show_metadata {
                    toolbar_btn_active
                } else {
                    toolbar_btn
                })
                .padding(btn_pad);

            Row::new()
                .spacing(8)
                .padding([4, 8])
                .push(Text::new(format!("{}{}", win.status, edit_indicator)).size(14))
                .push(iced::widget::Space::new().width(Fill))
                .push(info_btn)
                .push(
                    button(Text::new("\u{1F4C1} Libs").size(btn_size).center())
                        .on_press(Message::ToggleLibPaths)
                        .style(if self.show_lib_paths {
                            toolbar_btn_active
                        } else {
                            toolbar_btn
                        })
                        .padding(btn_pad),
                )
                .push(new_subflow_btn)
                .push(new_func_btn)
                .push(compile_btn)
        } else {
            Row::new()
                .spacing(8)
                .padding([4, 8])
                .push(Text::new(format!("{}{}", win.status, edit_indicator)).size(14))
                .push(iced::widget::Space::new().width(Fill))
        };

        // Flow I/O editor panel for sub-flow windows
        if !win.is_root && matches!(win.kind, WindowKind::FlowEditor) {
            right_col = right_col.push(self.view_flow_io_panel(window_id, win));
        }

        // Metadata editor panel (toggled by Info button)
        if win.show_metadata && matches!(win.kind, WindowKind::FlowEditor) {
            let authors_str = win.flow_definition.metadata.authors.join(", ");
            let meta_panel = container(
                Column::new()
                    .spacing(6)
                    .padding(12)
                    .push(
                        Row::new()
                            .spacing(8)
                            .align_y(iced::Alignment::Center)
                            .push(Text::new("Name:").size(12).width(70))
                            .push(
                                text_input("Flow name", &win.flow_name)
                                    .on_input(move |s| Message::FlowNameChanged(window_id, s))
                                    .size(13)
                                    .padding(4)
                                    .width(250),
                            ),
                    )
                    .push(
                        Row::new()
                            .spacing(8)
                            .align_y(iced::Alignment::Center)
                            .push(Text::new("Version:").size(12).width(70))
                            .push(
                                text_input("0.1.0", &win.flow_definition.metadata.version)
                                    .on_input(move |s| Message::FlowVersionChanged(window_id, s))
                                    .size(13)
                                    .padding(4)
                                    .width(120),
                            ),
                    )
                    .push(
                        Row::new()
                            .spacing(8)
                            .align_y(iced::Alignment::Center)
                            .push(Text::new("Description:").size(12).width(70))
                            .push(
                                text_input(
                                    "A short description",
                                    &win.flow_definition.metadata.description,
                                )
                                .on_input(move |s| Message::FlowDescriptionChanged(window_id, s))
                                .size(13)
                                .padding(4)
                                .width(Fill),
                            ),
                    )
                    .push(
                        Row::new()
                            .spacing(8)
                            .align_y(iced::Alignment::Center)
                            .push(Text::new("Authors:").size(12).width(70))
                            .push(
                                text_input("Name <email>, ...", &authors_str)
                                    .on_input(move |s| Message::FlowAuthorsChanged(window_id, s))
                                    .size(13)
                                    .padding(4)
                                    .width(Fill),
                            ),
                    ),
            )
            .style(|_theme: &Theme| container::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.14, 0.14, 0.18))),
                border: iced::Border {
                    color: Color::from_rgb(0.3, 0.3, 0.3),
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .width(Fill);

            right_col = right_col.push(meta_panel);
        }

        // Library paths panel (toggled by Libs button)
        if self.show_lib_paths {
            let mut paths_col = Column::new().spacing(4).padding(12);
            paths_col = paths_col.push(Text::new("Library Search Paths").size(14));

            for (i, p) in self.lib_paths.iter().enumerate() {
                let row = Row::new()
                    .spacing(6)
                    .align_y(iced::Alignment::Center)
                    .push(Text::new(p).size(12))
                    .push(iced::widget::Space::new().width(Fill))
                    .push(
                        button(Text::new("\u{2715}").size(10).center())
                            .on_press(Message::RemoveLibraryPath(i))
                            .style(button::danger)
                            .padding([2, 5]),
                    );
                paths_col = paths_col.push(row);
            }
            paths_col = paths_col.push(
                button(Text::new("+ Add Path...").size(12).center())
                    .on_press(Message::AddLibraryPath)
                    .style(button::secondary)
                    .padding([4, 10]),
            );

            let lib_panel = container(paths_col)
                .style(|_theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgb(0.14, 0.14, 0.18))),
                    border: iced::Border {
                        color: Color::from_rgb(0.3, 0.3, 0.3),
                        width: 1.0,
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                })
                .width(Fill);

            right_col = right_col.push(lib_panel);
        }

        right_col = right_col.push(container(status_bar).width(Fill).padding(5));

        let layout = Row::new().push(left_panel).push(right_col.width(Fill));
        layout.into()
    }

    fn view_flow_io_panel<'a>(
        &'a self,
        window_id: window::Id,
        win: &'a WindowState,
    ) -> Element<'a, Message> {
        let input_color = Color::from_rgb(0.4, 0.8, 1.0);
        let output_color = Color::from_rgb(1.0, 0.6, 0.3);

        let mut input_col = Column::new().spacing(4);
        for (i, port) in win.flow_inputs.iter().enumerate() {
            let dtype = port.datatypes.first().cloned().unwrap_or_default();
            let row = Row::new()
                .spacing(4)
                .align_y(iced::Alignment::Center)
                .push(Text::new("\u{25D7}").size(18).color(input_color))
                .push(
                    text_input("name", &port.name)
                        .on_input(move |s| Message::FlowInputNameChanged(window_id, i, s))
                        .size(12)
                        .padding(3)
                        .width(80),
                )
                .push(
                    text_input("type", &dtype)
                        .on_input(move |s| Message::FlowInputTypeChanged(window_id, i, s))
                        .size(11)
                        .padding(3)
                        .width(70),
                )
                .push(
                    button(Text::new("\u{2715}").size(10).center())
                        .on_press(Message::FlowDeleteInput(window_id, i))
                        .style(button::danger)
                        .padding([2, 5]),
                );
            input_col = input_col.push(row);
        }
        input_col = input_col.push(
            button(Text::new("+ Input").size(11).center())
                .on_press(Message::FlowAddInput(window_id))
                .style(button::secondary)
                .padding([2, 8]),
        );

        let mut output_col = Column::new().spacing(4).align_x(iced::Alignment::End);
        for (i, port) in win.flow_outputs.iter().enumerate() {
            let dtype = port.datatypes.first().cloned().unwrap_or_default();
            let row = Row::new()
                .spacing(4)
                .align_y(iced::Alignment::Center)
                .push(
                    button(Text::new("\u{2715}").size(10).center())
                        .on_press(Message::FlowDeleteOutput(window_id, i))
                        .style(button::danger)
                        .padding([2, 5]),
                )
                .push(
                    text_input("type", &dtype)
                        .on_input(move |s| Message::FlowOutputTypeChanged(window_id, i, s))
                        .size(11)
                        .padding(3)
                        .width(70),
                )
                .push(
                    text_input("name", &port.name)
                        .on_input(move |s| Message::FlowOutputNameChanged(window_id, i, s))
                        .size(12)
                        .padding(3)
                        .width(80),
                )
                .push(Text::new("\u{25D6}").size(18).color(output_color));
            output_col = output_col.push(row);
        }
        output_col = output_col.push(
            button(Text::new("+ Output").size(11).center())
                .on_press(Message::FlowAddOutput(window_id))
                .style(button::secondary)
                .padding([2, 8]),
        );

        let io_box = container(
            Column::new()
                .spacing(12)
                .padding(iced::Padding {
                    top: 8.0,
                    bottom: 8.0,
                    left: 0.0,
                    right: 0.0,
                })
                .push(
                    Row::new()
                        .push(input_col)
                        .push(iced::widget::Space::new().width(Fill))
                        .push(output_col),
                ),
        )
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.18, 0.18, 0.22))),
            border: iced::Border {
                color: Color::from_rgb(0.9, 0.6, 0.2),
                width: 2.0,
                radius: 12.0.into(),
            },
            ..Default::default()
        })
        .width(Fill)
        .padding([0, 8]);

        container(io_box).padding([6, 12]).width(Fill).into()
    }

    fn view_function<'a>(
        &'a self,
        window_id: window::Id,
        viewer: &'a FunctionViewer,
        status: &'a str,
    ) -> Element<'a, Message> {
        let content: Element<'_, Message> = match viewer.active_tab {
            0 => {
                let input_color = Color::from_rgb(0.4, 0.8, 1.0);
                let output_color = Color::from_rgb(1.0, 0.6, 0.3);

                // Input ports inside box: semicircle ◗ (flat left), name, type, delete
                let mut input_col = Column::new().spacing(6);
                for (i, port) in viewer.inputs.iter().enumerate() {
                    let dtype = port.datatypes.first().cloned().unwrap_or_default();
                    let row = Row::new()
                        .spacing(4)
                        .align_y(iced::Alignment::Center)
                        .push(Text::new("\u{25D7}").size(24).color(input_color))
                        .push(
                            text_input("name", &port.name)
                                .on_input(move |s| {
                                    Message::FunctionInputNameChanged(window_id, i, s)
                                })
                                .size(13)
                                .padding(3)
                                .width(90),
                        )
                        .push(
                            text_input("type", &dtype)
                                .on_input(move |s| {
                                    Message::FunctionInputTypeChanged(window_id, i, s)
                                })
                                .size(11)
                                .padding(3)
                                .width(75),
                        )
                        .push(
                            button(Text::new("\u{2715}").size(10).center())
                                .on_press(Message::FunctionDeleteInput(window_id, i))
                                .style(button::danger)
                                .padding([2, 5]),
                        );
                    input_col = input_col.push(row);
                }
                input_col = input_col.push(
                    button(Text::new("+ Input").size(11).center())
                        .on_press(Message::FunctionAddInput(window_id))
                        .style(button::secondary)
                        .padding([2, 8]),
                );

                // Output ports inside box: delete, type, name, semicircle ◖ (flat right)
                let mut output_col = Column::new().spacing(6).align_x(iced::Alignment::End);
                for (i, port) in viewer.outputs.iter().enumerate() {
                    let dtype = port.datatypes.first().cloned().unwrap_or_default();
                    let row = Row::new()
                        .spacing(4)
                        .align_y(iced::Alignment::Center)
                        .push(
                            button(Text::new("\u{2715}").size(10).center())
                                .on_press(Message::FunctionDeleteOutput(window_id, i))
                                .style(button::danger)
                                .padding([2, 5]),
                        )
                        .push(
                            text_input("type", &dtype)
                                .on_input(move |s| {
                                    Message::FunctionOutputTypeChanged(window_id, i, s)
                                })
                                .size(11)
                                .padding(3)
                                .width(75),
                        )
                        .push(
                            text_input("name", &port.name)
                                .on_input(move |s| {
                                    Message::FunctionOutputNameChanged(window_id, i, s)
                                })
                                .size(13)
                                .padding(3)
                                .width(90),
                        )
                        .push(Text::new("\u{25D6}").size(24).color(output_color));
                    output_col = output_col.push(row);
                }
                output_col = output_col.push(
                    button(Text::new("+ Output").size(11).center())
                        .on_press(Message::FunctionAddOutput(window_id))
                        .style(button::secondary)
                        .padding([2, 8]),
                );

                let name_input = container(
                    text_input("Function name", &viewer.name)
                        .on_input(move |s| Message::FunctionNameChanged(window_id, s))
                        .size(16)
                        .padding(6)
                        .width(250),
                )
                .center_x(Fill);

                let mut source_row = Row::new()
                    .spacing(6)
                    .align_y(iced::Alignment::Center)
                    .push(
                        button(
                            Text::new(&viewer.source_file)
                                .size(13)
                                .color(Color::from_rgb(0.6, 0.8, 1.0)),
                        )
                        .on_press(Message::FunctionTabSelected(window_id, 1))
                        .style(button::text)
                        .padding(0),
                    )
                    .push(
                        button(Text::new("...").size(12).center())
                            .on_press(Message::FunctionBrowseSource(window_id))
                            .style(button::secondary)
                            .padding([3, 8]),
                    );
                if viewer.docs_content.is_some() {
                    source_row = source_row.push(
                        button(Text::new("Docs").size(12).center())
                            .on_press(Message::FunctionTabSelected(window_id, 2))
                            .style(button::secondary)
                            .padding([3, 8]),
                    );
                }

                let func_box = container(
                    Column::new()
                        .spacing(20)
                        .padding(iced::Padding {
                            top: 24.0,
                            bottom: 24.0,
                            left: 0.0,
                            right: 0.0,
                        })
                        .push(name_input)
                        .push(
                            Row::new()
                                .push(input_col)
                                .push(iced::widget::Space::new().width(Fill))
                                .push(output_col),
                        )
                        .push(container(source_row).center_x(Fill)),
                )
                .style(|_theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgb(0.18, 0.18, 0.22))),
                    border: iced::Border {
                        color: Color::from_rgb(0.5, 0.3, 0.7),
                        width: 2.0,
                        radius: 12.0.into(),
                    },
                    ..Default::default()
                })
                .width(550);

                container(func_box).center(Fill).padding(40).into()
            }
            1 => {
                let back_btn = button(Text::new("\u{2190} Definition").size(13).center())
                    .on_press(Message::FunctionTabSelected(window_id, 0))
                    .style(button::secondary)
                    .padding([6, 14]);
                Column::new()
                    .push(container(back_btn).padding([8, 12]))
                    .push(
                        container(
                            iced::widget::scrollable(
                                Text::new(&viewer.rs_content)
                                    .size(14)
                                    .font(iced::Font::MONOSPACE),
                            )
                            .width(Fill)
                            .height(Fill),
                        )
                        .width(Fill)
                        .height(Fill)
                        .padding(12),
                    )
                    .into()
            }
            _ => {
                let back_btn = button(Text::new("\u{2190} Definition").size(13).center())
                    .on_press(Message::FunctionTabSelected(window_id, 0))
                    .style(button::secondary)
                    .padding([6, 14]);
                let docs = viewer.docs_content.as_deref().unwrap_or("");
                Column::new()
                    .push(container(back_btn).padding([8, 12]))
                    .push(
                        container(
                            iced::widget::scrollable(Text::new(docs).size(14))
                                .width(Fill)
                                .height(Fill),
                        )
                        .width(Fill)
                        .height(Fill)
                        .padding(12),
                    )
                    .into()
            }
        };

        let save_btn = button(Text::new("\u{1F4BE} Save").size(14).center())
            .on_press(Message::FunctionSave(window_id))
            .style(button::primary)
            .padding([6, 14]);

        let status_bar = Row::new()
            .spacing(8)
            .push(Text::new(status).size(14))
            .push(iced::widget::Space::new().width(Fill))
            .push(save_btn);

        Column::new()
            .push(container(content).width(Fill).height(Fill))
            .push(container(status_bar).width(Fill).padding(5))
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let keyboard_sub = keyboard::listen().filter_map(|event| match event {
            keyboard::Event::KeyPressed {
                key: keyboard::Key::Character(ref c),
                modifiers,
                ..
            } if modifiers.command() => match c.as_str() {
                "z" if modifiers.shift() => Some(Message::Redo),
                "z" => Some(Message::Undo),
                "s" if modifiers.shift() => Some(Message::SaveAs),
                "s" => Some(Message::Save),
                "o" => Some(Message::Open),
                "n" => Some(Message::New),
                "b" => Some(Message::Compile),
                "w" => Some(Message::CloseActiveWindow),
                "q" => Some(Message::QuitAll),
                _ => None,
            },
            _ => None,
        });

        let window_events = iced::event::listen_with(|event, _status, id| match event {
            iced::Event::Window(iced::window::Event::CloseRequested) => {
                Some(Message::CloseRequested(id))
            }
            iced::Event::Window(iced::window::Event::Closed) => Some(Message::WindowClosed(id)),
            iced::Event::Window(iced::window::Event::Focused) => Some(Message::WindowFocused(id)),
            iced::Event::Window(iced::window::Event::Resized(size)) => {
                Some(Message::WindowResized(id, size))
            }
            iced::Event::Window(iced::window::Event::Moved(pos)) => {
                Some(Message::WindowMoved(id, pos))
            }
            _ => None,
        });

        Subscription::batch(vec![keyboard_sub, window_events])
    }

    fn open_library_function(&mut self, source: &str) -> Task<Message> {
        use flowcore::provider::Provider;

        let provider = build_meta_provider();
        let source_url = match Url::parse(source) {
            Ok(u) => u,
            Err(_) => return Task::none(),
        };
        let (resolved_url, _) = match provider.resolve_url(&source_url, "default", &["toml"]) {
            Ok(r) => r,
            Err(_) => return Task::none(),
        };
        let path = match resolved_url.to_file_path() {
            Ok(p) => p,
            Err(()) => return Task::none(),
        };

        // Check if already open
        for (&win_id, win) in &self.windows {
            if win.file_path.as_ref() == Some(&path) {
                return window::gain_focus(win_id);
            }
        }

        // Read and parse
        let contents = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Task::none(),
        };
        let url = match Url::from_file_path(&path) {
            Ok(u) => u,
            Err(()) => return Task::none(),
        };
        let deserializer = match get::<Process>(&url) {
            Ok(d) => d,
            Err(_) => return Task::none(),
        };

        match deserializer.deserialize(&contents, Some(&url)) {
            Ok(Process::FunctionProcess(ref func)) => {
                let parent = match self.root_window {
                    Some(id) => id,
                    None => return Task::none(),
                };
                self.open_function_viewer(parent, &path, func)
            }
            Ok(Process::FlowProcess(_)) => match load_flow(&path) {
                Ok(loaded) => {
                    let (fi, fo) = extract_ports(&loaded.flow_def.inputs, &loaded.flow_def.outputs);
                    let has_nodes = !loaded.nodes.is_empty();
                    let nc = loaded.nodes.len();
                    let ec = loaded.edges.len();
                    let (new_id, open_task) =
                        window::open(self.child_window_settings(1024.0, 768.0));
                    let child = WindowState {
                        kind: WindowKind::FlowEditor,
                        flow_name: loaded.name,
                        nodes: loaded.nodes,
                        edges: loaded.edges,
                        canvas_state: FlowCanvasState::default(),
                        status: format!("Library flow - {nc} nodes, {ec} connections"),
                        selected_node: None,
                        selected_connection: None,
                        history: EditHistory::default(),
                        auto_fit_pending: has_nodes,
                        auto_fit_enabled: true,
                        unsaved_edits: 0,
                        compiled_manifest: None,
                        file_path: Some(path),
                        flow_definition: loaded.flow_def,
                        tooltip: None,
                        initializer_editor: None,
                        is_root: false,
                        flow_inputs: fi,
                        flow_outputs: fo,
                        context_menu: None,
                        show_metadata: false,
                        flow_hierarchy: self.build_hierarchy(),
                        last_size: None,
                        last_position: None,
                    };
                    self.windows.insert(new_id, child);
                    open_task.discard()
                }
                Err(_) => Task::none(),
            },
            Err(_) => Task::none(),
        }
    }

    fn update_lib_paths(&mut self) {
        let path_str = self.lib_paths.join(",");
        std::env::set_var("FLOW_LIB_PATH", &path_str);

        // Reload library catalogs with the updated search paths.
        // Gather lib_references and context_references from the root window's flow.
        let (lib_refs, ctx_refs) = self
            .root_window
            .and_then(|id| self.windows.get(&id))
            .map(|win| {
                (
                    win.flow_definition.lib_references.clone(),
                    win.flow_definition.context_references.clone(),
                )
            })
            .unwrap_or_default();
        let (lc, ld, cd) = load_library_catalogs(&lib_refs, &ctx_refs);
        self.library_cache = lc;
        self.lib_definitions = ld;
        self.context_definitions = cd;
        self.library_tree = LibraryTree::from_cache(
            &self.library_cache,
            &self.lib_definitions,
            &self.context_definitions,
        );
    }

    fn build_hierarchy(&self) -> FlowHierarchy {
        self.root_flow_path
            .as_ref()
            .map(|p| FlowHierarchy::build(p))
            .unwrap_or_else(FlowHierarchy::empty)
    }

    fn child_window_settings(&self, width: f32, height: f32) -> window::Settings {
        let n = self.windows.len() as f32;
        window::Settings {
            size: iced::Size::new(width, height),
            position: window::Position::Specific(iced::Point::new(
                200.0 + n * 30.0,
                150.0 + n * 30.0,
            )),
            ..Default::default()
        }
    }

    /// Open a sub-flow in a new in-process window, or show a status message
    /// if the node resolves to a function rather than a flow.
    fn open_node(&mut self, parent_win_id: window::Id, idx: usize) -> Task<Message> {
        // Extract source and resolved path from the parent window (immutable borrow)
        let (source, resolved_path) = {
            let Some(win) = self.windows.get(&parent_win_id) else {
                return Task::none();
            };
            let Some(node) = win.nodes.get(idx) else {
                return Task::none();
            };
            let source = node.source.clone();
            let path = resolve_node_source(win, &source);
            (source, path)
        };

        let Some(path) = resolved_path else {
            if let Some(win) = self.windows.get_mut(&parent_win_id) {
                win.status = format!("Could not resolve source: {source}");
            }
            return Task::none();
        };

        // If a window already has this file open, focus it instead of opening a duplicate
        for (&win_id, win) in &self.windows {
            if win.file_path.as_ref() == Some(&path) && win_id != parent_win_id {
                return window::gain_focus(win_id);
            }
        }

        // Check whether the resolved file is a flow or a function
        let abs_path = match std::fs::canonicalize(&path) {
            Ok(p) => p,
            Err(_) => path.clone(),
        };
        if let Ok(contents) = std::fs::read_to_string(&abs_path) {
            if let Ok(url) = Url::from_file_path(&abs_path) {
                if let Ok(deserializer) = get::<Process>(&url) {
                    if let Ok(Process::FunctionProcess(ref func)) =
                        deserializer.deserialize(&contents, Some(&url))
                    {
                        return self.open_function_viewer(parent_win_id, &path, func);
                    }
                }
            }
        }

        // Load the sub-flow and open it in a new window
        match load_flow(&path) {
            Ok(loaded) => {
                let has_nodes = !loaded.nodes.is_empty();
                let (new_id, open_task) = window::open(self.child_window_settings(1024.0, 768.0));
                let nc = loaded.nodes.len();
                let ec = loaded.edges.len();
                let (fi, fo) = extract_ports(&loaded.flow_def.inputs, &loaded.flow_def.outputs);
                let child = WindowState {
                    kind: WindowKind::FlowEditor,
                    flow_name: loaded.name,
                    nodes: loaded.nodes,
                    edges: loaded.edges,
                    canvas_state: FlowCanvasState::default(),
                    status: format!("Ready - {nc} nodes, {ec} connections"),
                    selected_node: None,
                    selected_connection: None,
                    history: EditHistory::default(),
                    auto_fit_pending: has_nodes,
                    auto_fit_enabled: true,
                    unsaved_edits: 0,
                    compiled_manifest: None,
                    file_path: Some(path.clone()),
                    flow_definition: loaded.flow_def,
                    tooltip: None,
                    initializer_editor: None,
                    is_root: false,
                    flow_inputs: fi,
                    flow_outputs: fo,
                    context_menu: None,
                    show_metadata: false,
                    flow_hierarchy: self.build_hierarchy(),
                    last_size: None,
                    last_position: None,
                };
                self.windows.insert(new_id, child);
                if let Some(win) = self.windows.get_mut(&parent_win_id) {
                    win.status = format!("Opened: {}", path.display());
                }
                open_task.discard()
            }
            Err(e) => {
                if let Some(win) = self.windows.get_mut(&parent_win_id) {
                    win.status = format!("Could not open '{}': {e}", source);
                }
                Task::none()
            }
        }
    }

    fn open_function_viewer(
        &mut self,
        parent_win_id: window::Id,
        toml_path: &Path,
        func: &flowcore::model::function_definition::FunctionDefinition,
    ) -> Task<Message> {
        let dir = toml_path.parent().unwrap_or(Path::new("."));
        let func_name = &func.name;

        let rs_path = dir.join(&func.source);
        let rs_content = std::fs::read_to_string(&rs_path)
            .unwrap_or_else(|_| String::from("// Source file not found"));
        let docs_content = std::fs::read_to_string(dir.join(format!("{func_name}.md"))).ok();

        let (inputs, outputs) = extract_ports(&func.inputs, &func.outputs);

        let (new_id, open_task) = window::open(self.child_window_settings(700.0, 500.0));

        let viewer = FunctionViewer {
            name: func_name.clone(),
            source_file: func.source.clone(),
            inputs,
            outputs,
            rs_content,
            docs_content,
            active_tab: 0,
            toml_path: toml_path.to_path_buf(),
        };

        let child = WindowState {
            kind: WindowKind::FunctionViewer(viewer),
            flow_name: func_name.clone(),
            nodes: Vec::new(),
            edges: Vec::new(),
            canvas_state: FlowCanvasState::default(),
            status: format!("Function: {func_name}"),
            selected_node: None,
            selected_connection: None,
            history: EditHistory::default(),
            auto_fit_pending: false,
            auto_fit_enabled: false,
            unsaved_edits: 0,
            compiled_manifest: None,
            file_path: Some(toml_path.to_path_buf()),
            flow_definition: FlowDefinition::default(),
            tooltip: None,
            initializer_editor: None,
            is_root: false,
            flow_inputs: Vec::new(),
            flow_outputs: Vec::new(),
            context_menu: None,
            show_metadata: false,
            flow_hierarchy: self.build_hierarchy(),
            last_size: None,
            last_position: None,
        };

        self.windows.insert(new_id, child);
        if let Some(win) = self.windows.get_mut(&parent_win_id) {
            win.status = format!("Opened function: {func_name}");
        }
        open_task.discard()
    }

    fn create_new_subflow(&mut self) -> Task<Message> {
        let Some(root_id) = self.root_window else {
            return Task::none();
        };

        // Get the parent flow's directory for relative path resolution
        let base_dir = self
            .windows
            .get(&root_id)
            .and_then(|w| w.file_path.as_ref())
            .and_then(|p| p.parent())
            .map(Path::to_path_buf);

        let Some(base) = base_dir else {
            if let Some(win) = self.windows.get_mut(&root_id) {
                win.status = String::from("Save the flow first before creating a sub-flow");
            }
            return Task::none();
        };

        // Prompt user to choose where to save the new sub-flow
        let dialog = rfd::FileDialog::new()
            .add_filter("Flow", &["toml"])
            .set_directory(&base)
            .set_file_name("new_subflow.toml");
        let Some(path) = dialog.save_file() else {
            return Task::none();
        };

        // Derive flow name from filename
        let flow_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("subflow")
            .to_string();

        // Create the sub-flow definition with empty content
        let flow_def = FlowDefinition {
            name: flow_name.clone(),
            ..FlowDefinition::default()
        };

        // Write the initial TOML file
        let toml = format!("flow = \"{flow_name}\"\n");
        if let Err(e) = std::fs::write(&path, &toml) {
            if let Some(win) = self.windows.get_mut(&root_id) {
                win.status = format!("Could not create sub-flow: {e}");
            }
            return Task::none();
        }

        // Compute relative source path from parent flow to new sub-flow
        let source = path
            .strip_prefix(&base)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.to_string_lossy().to_string());
        // Strip .toml extension for the source reference
        let source = source.strip_suffix(".toml").unwrap_or(&source).to_string();

        // Add a process reference in the parent flow
        if let Some(win) = self.windows.get_mut(&root_id) {
            let alias = generate_unique_alias(&flow_name, &win.nodes);
            let (x, y) = next_node_position(&win.nodes);

            let node = NodeLayout {
                alias: alias.clone(),
                source: source.clone(),
                x,
                y,
                width: 180.0,
                height: 120.0,
                inputs: Vec::new(),
                outputs: Vec::new(),
                initializers: HashMap::new(),
            };
            win.nodes.push(node);
            win.flow_definition.process_refs.push(ProcessReference {
                alias: alias.clone(),
                source,
                initializations: std::collections::BTreeMap::new(),
                x: Some(x),
                y: Some(y),
                width: Some(180.0),
                height: Some(120.0),
            });
            win.unsaved_edits += 1;
            win.canvas_state.request_redraw();
            win.status = format!("Created sub-flow: {alias}");
        }

        // Open the new sub-flow in a child window
        let (new_id, open_task) = window::open(self.child_window_settings(1024.0, 768.0));

        let child = WindowState {
            kind: WindowKind::FlowEditor,
            flow_name: flow_name.clone(),
            nodes: Vec::new(),
            edges: Vec::new(),
            canvas_state: FlowCanvasState::default(),
            status: format!("New sub-flow: {flow_name}"),
            selected_node: None,
            selected_connection: None,
            history: EditHistory::default(),
            auto_fit_pending: false,
            auto_fit_enabled: true,
            unsaved_edits: 0,
            compiled_manifest: None,
            file_path: Some(path),
            flow_definition: flow_def,
            tooltip: None,
            initializer_editor: None,
            is_root: false,
            flow_inputs: Vec::new(),
            flow_outputs: Vec::new(),
            context_menu: None,
            show_metadata: false,
            flow_hierarchy: self.build_hierarchy(),
            last_size: None,
            last_position: None,
        };

        self.windows.insert(new_id, child);
        open_task.discard()
    }

    fn create_new_function(&mut self) -> Task<Message> {
        let Some(root_id) = self.root_window else {
            return Task::none();
        };

        let base_dir = self
            .windows
            .get(&root_id)
            .and_then(|w| w.file_path.as_ref())
            .and_then(|p| p.parent())
            .map(Path::to_path_buf);

        let Some(base) = base_dir else {
            if let Some(win) = self.windows.get_mut(&root_id) {
                win.status = String::from("Save the flow first before creating a function");
            }
            return Task::none();
        };

        // Prompt user to choose where to save the new function definition
        let dialog = rfd::FileDialog::new()
            .add_filter("Flow Function", &["toml"])
            .set_directory(&base)
            .set_file_name("new_function.toml");
        let Some(path) = dialog.save_file() else {
            return Task::none();
        };

        let func_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("function")
            .to_string();

        let rs_filename = format!("{func_name}.rs");

        // Compute relative source from parent flow
        let source = path
            .strip_prefix(&base)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.to_string_lossy().to_string());
        let source = source.strip_suffix(".toml").unwrap_or(&source).to_string();

        // Add process reference in the parent flow
        if let Some(win) = self.windows.get_mut(&root_id) {
            let alias = generate_unique_alias(&func_name, &win.nodes);
            let (x, y) = next_node_position(&win.nodes);

            let node = NodeLayout {
                alias: alias.clone(),
                source: source.clone(),
                x,
                y,
                width: 180.0,
                height: 120.0,
                inputs: Vec::new(),
                outputs: Vec::new(),
                initializers: HashMap::new(),
            };
            win.nodes.push(node);
            win.flow_definition.process_refs.push(ProcessReference {
                alias: alias.clone(),
                source,
                initializations: std::collections::BTreeMap::new(),
                x: Some(x),
                y: Some(y),
                width: Some(180.0),
                height: Some(120.0),
            });
            win.unsaved_edits += 1;
            win.canvas_state.request_redraw();
            win.status = format!("Created function: {alias}");
        }

        // Open the function viewer window
        let (new_id, open_task) = window::open(self.child_window_settings(700.0, 500.0));

        let viewer = FunctionViewer {
            name: func_name.clone(),
            source_file: rs_filename,
            inputs: Vec::new(),
            outputs: Vec::new(),
            rs_content: String::from("// Save to generate skeleton source"),
            docs_content: None,
            active_tab: 0,
            toml_path: path.clone(),
        };

        let child = WindowState {
            kind: WindowKind::FunctionViewer(viewer),
            flow_name: func_name,
            nodes: Vec::new(),
            edges: Vec::new(),
            canvas_state: FlowCanvasState::default(),
            status: String::from("New function — add ports and Save"),
            selected_node: None,
            selected_connection: None,
            history: EditHistory::default(),
            auto_fit_pending: false,
            auto_fit_enabled: false,
            unsaved_edits: 1,
            compiled_manifest: None,
            file_path: Some(path),
            flow_definition: FlowDefinition::default(),
            tooltip: None,
            initializer_editor: None,
            is_root: false,
            flow_inputs: Vec::new(),
            flow_outputs: Vec::new(),
            context_menu: None,
            show_metadata: false,
            flow_hierarchy: self.build_hierarchy(),
            last_size: None,
            last_position: None,
        };

        self.windows.insert(new_id, child);
        open_task.discard()
    }
}

/// Record an edit action in the history and increment the unsaved edit count.
fn record_edit(win: &mut WindowState, action: EditAction) {
    win.history.record(action);
    win.unsaved_edits += 1;
    win.compiled_manifest = None; // Invalidate compilation on any edit
}

/// Apply an undo action -- reverse the last edit.
fn apply_undo(win: &mut WindowState) {
    if let Some(action) = win.history.undo() {
        match action {
            EditAction::MoveNode {
                index,
                old_x,
                old_y,
                ..
            } => {
                if let Some(node) = win.nodes.get_mut(index) {
                    node.x = old_x;
                    node.y = old_y;
                }
                win.status = String::from("Undo: move");
            }
            EditAction::ResizeNode {
                index,
                old_x,
                old_y,
                old_w,
                old_h,
                ..
            } => {
                if let Some(node) = win.nodes.get_mut(index) {
                    node.x = old_x;
                    node.y = old_y;
                    node.width = old_w;
                    node.height = old_h;
                }
                win.status = String::from("Undo: resize");
            }
            EditAction::DeleteNode {
                index,
                node,
                removed_edges,
            } => {
                win.nodes.insert(index, node);
                win.edges.extend(removed_edges);
                win.status = String::from("Undo: delete node");
            }
            EditAction::CreateConnection { edge } => {
                win.edges.retain(|e| {
                    e.from_node != edge.from_node
                        || e.from_port != edge.from_port
                        || e.to_node != edge.to_node
                        || e.to_port != edge.to_port
                });
                win.status = String::from("Undo: create connection");
            }
            EditAction::DeleteConnection { index, edge } => {
                win.edges.insert(index, edge);
                win.status = String::from("Undo: delete connection");
            }
            EditAction::EditInitializer {
                node_index,
                ref port_name,
                ref old_init,
                ref old_display,
                ..
            } => {
                apply_initializer_state(
                    win,
                    node_index,
                    port_name,
                    old_init.as_ref(),
                    old_display.as_ref(),
                );
                win.status = String::from("Undo: initializer");
            }
        }
        win.canvas_state.request_redraw();
    }
}

/// Save the current flow to the given path.
fn perform_save(win: &mut WindowState, path: &PathBuf) {
    sync_flow_definition(win);
    match save_flow_toml(&win.flow_definition, &win.edges, path) {
        Ok(()) => {
            win.unsaved_edits = 0;
            win.file_path = Some(path.clone());
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
fn perform_save_as(win: &mut WindowState) {
    let dialog = rfd::FileDialog::new()
        .add_filter("Flow", &["toml"])
        .set_file_name(format!("{}.toml", win.flow_name));
    if let Some(path) = dialog.save_file() {
        perform_save(win, &path);
    }
}

/// Prompt the user with an open dialog and load the selected flow file.
/// Open a flow file and update the window state.
/// Returns the lib and context references if successful, for rebuilding the library cache.
fn perform_open(win: &mut WindowState) -> Option<(BTreeSet<Url>, BTreeSet<Url>)> {
    let dialog = rfd::FileDialog::new().add_filter("Flow", &["toml"]);
    if let Some(path) = dialog.pick_file() {
        match load_flow(&path) {
            Ok(loaded) => {
                let nc = loaded.nodes.len();
                let ec = loaded.edges.len();
                let (fi, fo) = extract_ports(&loaded.flow_def.inputs, &loaded.flow_def.outputs);
                win.flow_name = loaded.name;
                win.nodes = loaded.nodes;
                win.edges = loaded.edges;
                win.flow_definition = loaded.flow_def;
                win.file_path = Some(path);
                win.flow_inputs = fi;
                win.flow_outputs = fo;
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
fn perform_new(win: &mut WindowState) {
    win.flow_name = String::from("(new flow)");
    win.nodes = Vec::new();
    win.edges = Vec::new();
    win.flow_definition = FlowDefinition::default();
    win.file_path = None;
    win.flow_inputs = Vec::new();
    win.flow_outputs = Vec::new();
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
fn perform_compile(win: &mut WindowState) -> Result<PathBuf, String> {
    // New flows must be saved first so the compiler has a real file path
    if win.file_path.is_none() {
        perform_save_as(win);
    }
    let Some(flow_path) = win.file_path.clone() else {
        return Err("Flow must be saved before compiling".to_string());
    };

    // Save any unsaved edits so the file on disk matches the editor state
    if win.unsaved_edits > 0 {
        perform_save(win, &flow_path);
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

/// Add a function from the library panel as a new node on the canvas.
///
/// Creates a `NodeLayout` at a default position and a `ProcessReference`
/// in the flow definition, and records the action in the edit history.
fn add_library_function(win: &mut WindowState, source: &str, func_name: &str) {
    // Generate a unique alias: if the name already exists, append a number
    let alias = generate_unique_alias(func_name, &win.nodes);

    // Place the new node at a default position offset from existing nodes
    let (x, y) = next_node_position(&win.nodes);

    // Resolve port info by parsing the function/flow definition
    let (inputs, outputs) = match Url::parse(source) {
        Ok(url) => {
            let provider = build_meta_provider();
            match flowrclib::compiler::parser::parse(&url, &provider) {
                Ok(Process::FunctionProcess(func)) => extract_ports(&func.inputs, &func.outputs),
                Ok(Process::FlowProcess(flow)) => extract_ports(&flow.inputs, &flow.outputs),
                Err(e) => {
                    info!("add_library_function: could not parse '{source}': {e}");
                    (Vec::new(), Vec::new())
                }
            }
        }
        Err(e) => {
            info!("add_library_function: could not parse URL '{source}': {e}");
            (Vec::new(), Vec::new())
        }
    };

    let node = NodeLayout {
        alias: alias.clone(),
        source: source.to_string(),
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

    record_edit(
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
fn resolve_node_source(win: &WindowState, source: &str) -> Option<PathBuf> {
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

/// Apply an initializer edit to the flow definition and update the node display.
fn apply_initializer_edit(win: &mut WindowState, editor: &InitializerEditor) {
    let alias = win
        .nodes
        .get(editor.node_index)
        .map(|n| n.alias.clone())
        .unwrap_or_default();

    // Capture old state for undo
    let old_init = win
        .flow_definition
        .process_refs
        .iter()
        .find(|pr| {
            let pr_alias = if pr.alias.is_empty() {
                derive_short_name(&pr.source)
            } else {
                pr.alias.to_string()
            };
            pr_alias == alias
        })
        .and_then(|pr| pr.initializations.get(&editor.port_name).cloned());
    let old_display = win
        .nodes
        .get(editor.node_index)
        .and_then(|n| n.initializers.get(&editor.port_name).cloned());

    // Compute new initializer and display
    let (new_init, new_display) = match editor.init_type.as_str() {
        "none" => (None, None),
        "once" | "always" => {
            let value = serde_json::from_str(&editor.value_text)
                .unwrap_or_else(|_| serde_json::Value::String(editor.value_text.clone()));
            let init = if editor.init_type == "once" {
                InputInitializer::Once(value)
            } else {
                InputInitializer::Always(value)
            };
            let display = format!("{}: {}", editor.init_type, editor.value_text);
            (Some(init), Some(display))
        }
        _ => return,
    };

    // Apply to model
    if let Some(pref) = win.flow_definition.process_refs.iter_mut().find(|pr| {
        let pr_alias = if pr.alias.is_empty() {
            derive_short_name(&pr.source)
        } else {
            pr.alias.to_string()
        };
        pr_alias == alias
    }) {
        match &new_init {
            Some(init) => {
                pref.initializations
                    .insert(editor.port_name.clone(), init.clone());
            }
            None => {
                pref.initializations.remove(&editor.port_name);
            }
        }
    }

    // Apply to display
    if let Some(node) = win.nodes.get_mut(editor.node_index) {
        match &new_display {
            Some(display) => {
                node.initializers
                    .insert(editor.port_name.clone(), display.clone());
            }
            None => {
                node.initializers.remove(&editor.port_name);
            }
        }
    }

    win.history.record(EditAction::EditInitializer {
        node_index: editor.node_index,
        port_name: editor.port_name.clone(),
        old_init,
        old_display,
        new_init,
        new_display,
    });
    win.unsaved_edits += 1;
    win.compiled_manifest = None;
    win.canvas_state.request_redraw();
    win.status = format!("Initializer updated on {}/{}", alias, editor.port_name);
}

/// Synchronize the in-memory `FlowDefinition` with the current editor state
/// so that process references and the flow name are up to date.
/// Connections are handled separately via `EdgeLayout` during save.
fn sync_flow_definition(win: &mut WindowState) {
    // Update or rebuild process_refs from current NodeLayout data
    let mut new_refs: Vec<ProcessReference> = Vec::with_capacity(win.nodes.len());
    for node in &win.nodes {
        // Try to find the original ProcessReference by alias to preserve initializations
        let original = win
            .flow_definition
            .process_refs
            .iter()
            .find(|pr| {
                let alias = if pr.alias.is_empty() {
                    derive_short_name(&pr.source)
                } else {
                    pr.alias.to_string()
                };
                alias == node.alias
            })
            .cloned();

        let pref = if let Some(mut orig) = original {
            orig.x = Some(node.x);
            orig.y = Some(node.y);
            orig.width = Some(node.width);
            orig.height = Some(node.height);
            orig
        } else {
            // New node without an original -- build from scratch
            ProcessReference {
                alias: node.alias.clone(),
                source: node.source.clone(),
                initializations: std::collections::BTreeMap::new(),
                x: Some(node.x),
                y: Some(node.y),
                width: Some(node.width),
                height: Some(node.height),
            }
        };
        new_refs.push(pref);
    }
    win.flow_definition.process_refs = new_refs;

    // Update the flow name
    win.flow_definition.name = win.flow_name.clone();
}

/// Apply a redo action -- re-apply the last undone edit.
fn apply_redo(win: &mut WindowState) {
    if let Some(action) = win.history.redo() {
        match action {
            EditAction::MoveNode {
                index,
                new_x,
                new_y,
                ..
            } => {
                if let Some(node) = win.nodes.get_mut(index) {
                    node.x = new_x;
                    node.y = new_y;
                }
                win.status = String::from("Redo: move");
            }
            EditAction::ResizeNode {
                index,
                new_x,
                new_y,
                new_w,
                new_h,
                ..
            } => {
                if let Some(node) = win.nodes.get_mut(index) {
                    node.x = new_x;
                    node.y = new_y;
                    node.width = new_w;
                    node.height = new_h;
                }
                win.status = String::from("Redo: resize");
            }
            EditAction::DeleteNode {
                index,
                removed_edges,
                node,
                ..
            } => {
                let alias = node.alias.clone();
                if index <= win.nodes.len() {
                    win.nodes.remove(index);
                }
                for edge in &removed_edges {
                    win.edges.retain(|e| {
                        e.from_node != edge.from_node
                            || e.from_port != edge.from_port
                            || e.to_node != edge.to_node
                            || e.to_port != edge.to_port
                    });
                }
                let _ = alias; // used for edge cleanup above
                win.status = String::from("Redo: delete node");
            }
            EditAction::CreateConnection { edge } => {
                win.edges.push(edge);
                win.status = String::from("Redo: create connection");
            }
            EditAction::DeleteConnection { index, .. } => {
                if index < win.edges.len() {
                    win.edges.remove(index);
                }
                win.status = String::from("Redo: delete connection");
            }
            EditAction::EditInitializer {
                node_index,
                ref port_name,
                ref new_init,
                ref new_display,
                ..
            } => {
                apply_initializer_state(
                    win,
                    node_index,
                    port_name,
                    new_init.as_ref(),
                    new_display.as_ref(),
                );
                win.status = String::from("Redo: initializer");
            }
        }
        win.canvas_state.request_redraw();
    }
}

/// Apply an initializer state to both the model and display.
fn apply_initializer_state(
    win: &mut WindowState,
    node_index: usize,
    port_name: &str,
    init: Option<&InputInitializer>,
    display: Option<&String>,
) {
    let alias = win
        .nodes
        .get(node_index)
        .map(|n| n.alias.clone())
        .unwrap_or_default();

    if let Some(pref) = win.flow_definition.process_refs.iter_mut().find(|pr| {
        let pr_alias = if pr.alias.is_empty() {
            derive_short_name(&pr.source)
        } else {
            pr.alias.to_string()
        };
        pr_alias == alias
    }) {
        match init {
            Some(i) => {
                pref.initializations
                    .insert(port_name.to_string(), i.clone());
            }
            None => {
                pref.initializations.remove(port_name);
            }
        }
    }

    if let Some(node) = win.nodes.get_mut(node_index) {
        match display {
            Some(d) => {
                node.initializers.insert(port_name.to_string(), d.clone());
            }
            None => {
                node.initializers.remove(port_name);
            }
        }
    }
}

/// Build a `MetaProvider` with `FLOW_LIB_PATH` (plus `~/.flow/lib` default)
/// and the default flowrcli context root.
fn build_meta_provider() -> MetaProvider {
    let mut lib_search_path = Simpath::new_with_separator("FLOW_LIB_PATH", ',');
    if let Ok(home) = std::env::var("HOME") {
        let default_lib = PathBuf::from(&home).join(".flow").join("lib");
        if default_lib.exists() {
            if let Some(path_str) = default_lib.to_str() {
                lib_search_path.add_directory(path_str);
            }
        }
    }
    let context_root = std::env::var("HOME")
        .map(|h| {
            PathBuf::from(h)
                .join(".flow")
                .join("runner")
                .join("flowrcli")
        })
        .unwrap_or_else(|_| PathBuf::from("/"));
    MetaProvider::new(lib_search_path, context_root)
}

/// Resolve the library search paths from the `FLOW_LIB_PATH` environment variable
/// and the default `~/.flow/lib` directory.
fn resolve_lib_paths() -> Vec<String> {
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

fn extract_ports(
    inputs: &[flowcore::model::io::IO],
    outputs: &[flowcore::model::io::IO],
) -> (Vec<PortInfo>, Vec<PortInfo>) {
    let input_ports = inputs
        .iter()
        .map(|io| PortInfo {
            name: io.name().to_string(),
            datatypes: io.datatypes().iter().map(|dt| dt.to_string()).collect(),
        })
        .collect();
    let output_ports = outputs
        .iter()
        .map(|io| PortInfo {
            name: io.name().to_string(),
            datatypes: io.datatypes().iter().map(|dt| dt.to_string()).collect(),
        })
        .collect();
    (input_ports, output_ports)
}

/// Result of loading a flow definition file.
struct LoadedFlow {
    name: String,
    nodes: Vec<NodeLayout>,
    edges: Vec<EdgeLayout>,
    flow_def: FlowDefinition,
    lib_references: BTreeSet<Url>,
    context_references: BTreeSet<Url>,
}

/// Load a flow definition file and return the flow name, node layouts, edge layouts,
/// the original `FlowDefinition`, and the library/context references for catalog loading.
fn load_flow(path: &PathBuf) -> Result<LoadedFlow, String> {
    let abs_path = if path.is_absolute() {
        path.clone()
    } else {
        std::env::current_dir()
            .map_err(|e| format!("Could not get current directory: {e}"))?
            .join(path)
    };

    let url =
        Url::from_file_path(&abs_path).map_err(|()| format!("Invalid file path: {abs_path:?}"))?;

    let provider = build_meta_provider();
    let process = flowrclib::compiler::parser::parse(&url, &provider)
        .map_err(|e| format!("Could not parse flow definition: {e}"))?;

    match process {
        Process::FlowProcess(flow) => {
            // Extract port definitions from the fully-resolved subprocesses
            let mut resolved_ports = HashMap::new();
            for (alias, subprocess) in &flow.subprocesses {
                let (inputs, outputs) = match subprocess {
                    Process::FunctionProcess(func) => {
                        extract_ports(&func.inputs, &func.outputs)
                    }
                    Process::FlowProcess(sub_flow) => {
                        extract_ports(&sub_flow.inputs, &sub_flow.outputs)
                    }
                };
                info!(
                    "Resolved '{}': {} inputs, {} outputs",
                    alias,
                    inputs.len(),
                    outputs.len()
                );
                resolved_ports.insert(alias.to_string(), (inputs, outputs));
            }

            let edges = build_edge_layouts(&flow.connections);
            let nodes =
                build_node_layouts(&flow.process_refs, &flow.connections, &resolved_ports);
            let name = flow.name.clone();
            let lib_references = flow.lib_references.clone();
            let context_references = flow.context_references.clone();
            Ok(LoadedFlow {
                name,
                nodes,
                edges,
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

/// Load full library catalogs and cache all definitions.
///
/// For each unique library root URL found in `lib_references`, loads the library
/// manifest and parses every function/flow definition in that library. For each
/// URL in `context_references`, parses the context function definition.
fn load_library_catalogs(
    lib_references: &BTreeSet<Url>,
    context_references: &BTreeSet<Url>,
) -> (
    HashMap<Url, LibraryManifest>,
    HashMap<Url, Process>,
    HashMap<Url, Process>,
) {
    let provider = build_meta_provider();
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
                let meta_provider = build_meta_provider();
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

    // Parse each context function definition
    let ctx_provider = build_meta_provider();
    for context_ref in context_references {
        match flowrclib::compiler::parser::parse(context_ref, &ctx_provider) {
            Ok(process) => {
                context_definitions.insert(context_ref.clone(), process);
            }
            Err(e) => {
                warn!(
                    "Could not parse context definition '{}': {}",
                    context_ref, e
                );
            }
        }
    }

    (library_cache, lib_definitions, context_definitions)
}

/// Serialize a `serde_json::Value` into a TOML-compatible inline value string.
fn value_to_toml(v: &serde_json::Value) -> String {
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
/// Connections are written from `edges` to preserve names that would be lost
/// when roundtripping through `Connection::new`.
fn save_flow_toml(
    flow: &FlowDefinition,
    edges: &[EdgeLayout],
    path: &PathBuf,
) -> Result<(), String> {
    let mut out = String::new();

    // Flow name
    out.push_str(&format!("flow = \"{}\"\n", flow.name));

    // Docs
    if !flow.docs.is_empty() {
        out.push_str(&format!("docs = \"{}\"\n", flow.docs));
    }

    // Metadata (only if any field is non-empty)
    let md = &flow.metadata;
    if !md.version.is_empty() || !md.description.is_empty() || !md.authors.is_empty() {
        out.push_str("\n[metadata]\n");
        if !md.version.is_empty() {
            out.push_str(&format!("version = \"{}\"\n", md.version));
        }
        if !md.description.is_empty() {
            out.push_str(&format!("description = \"{}\"\n", md.description));
        }
        if !md.authors.is_empty() {
            let authors: Vec<String> = md.authors.iter().map(|a| format!("\"{a}\"")).collect();
            out.push_str(&format!("authors = [{}]\n", authors.join(", ")));
        }
    }

    // Flow-level inputs
    for input in &flow.inputs {
        out.push_str("\n[[input]]\n");
        let name = input.name();
        if !name.is_empty() {
            out.push_str(&format!("name = \"{name}\"\n"));
        }
        let types = input.datatypes();
        if types.len() == 1 {
            if let Some(t) = types.first() {
                out.push_str(&format!("type = \"{t}\"\n"));
            }
        } else if types.len() > 1 {
            let ts: Vec<String> = types.iter().map(|t| format!("\"{t}\"")).collect();
            out.push_str(&format!("type = [{}]\n", ts.join(", ")));
        }
    }

    // Flow-level outputs
    for output in &flow.outputs {
        out.push_str("\n[[output]]\n");
        let name = output.name();
        if !name.is_empty() {
            out.push_str(&format!("name = \"{name}\"\n"));
        }
        let types = output.datatypes();
        if types.len() == 1 {
            if let Some(t) = types.first() {
                out.push_str(&format!("type = \"{t}\"\n"));
            }
        } else if types.len() > 1 {
            let ts: Vec<String> = types.iter().map(|t| format!("\"{t}\"")).collect();
            out.push_str(&format!("type = [{}]\n", ts.join(", ")));
        }
    }

    // Processes
    for pref in &flow.process_refs {
        out.push_str("\n[[process]]\n");
        if !pref.alias.is_empty() {
            out.push_str(&format!("alias = \"{}\"\n", pref.alias));
        }
        out.push_str(&format!("source = \"{}\"\n", pref.source));

        // Layout positions
        if let Some(x) = pref.x {
            out.push_str(&format!("x = {x}\n"));
        }
        if let Some(y) = pref.y {
            out.push_str(&format!("y = {y}\n"));
        }
        if let Some(w) = pref.width {
            out.push_str(&format!("width = {w}\n"));
        }
        if let Some(h) = pref.height {
            out.push_str(&format!("height = {h}\n"));
        }

        // Initializations
        for (port_name, init) in &pref.initializations {
            out.push_str(&format!(
                "input.{port_name} = {}\n",
                initializer_to_toml(init)
            ));
        }
    }

    // Connections (from EdgeLayout to preserve names)
    for edge in edges {
        out.push_str("\n[[connection]]\n");
        if !edge.name.is_empty() {
            out.push_str(&format!("name = \"{}\"\n", edge.name));
        }
        let from = if edge.from_port.is_empty() {
            edge.from_node.clone()
        } else {
            format!("{}/{}", edge.from_node, edge.from_port)
        };
        out.push_str(&format!("from = \"{from}\"\n"));
        let to = if edge.to_port.is_empty() {
            edge.to_node.clone()
        } else {
            format!("{}/{}", edge.to_node, edge.to_port)
        };
        out.push_str(&format!("to = \"{to}\"\n"));
    }

    std::fs::write(path, out).map_err(|e| format!("Could not write file: {e}"))
}

/// Generate a unique alias for a new node, appending a numeric suffix if needed.
fn generate_unique_alias(base_name: &str, nodes: &[NodeLayout]) -> String {
    let existing: Vec<&str> = nodes.iter().map(|n| n.alias.as_str()).collect();
    if !existing.contains(&base_name) {
        return base_name.to_string();
    }
    let mut counter = 2u32;
    loop {
        let candidate = format!("{base_name}_{counter}");
        if !existing.iter().any(|a| *a == candidate) {
            return candidate;
        }
        counter = counter.saturating_add(1);
    }
}

/// Compute a default position for a new node, offset from the last node or at a default origin.
fn next_node_position(nodes: &[NodeLayout]) -> (f32, f32) {
    if nodes.is_empty() {
        return (100.0, 100.0);
    }
    // Find the rightmost node and place the new one to its right
    let max_right = nodes.iter().map(|n| n.x + n.width).fold(0.0_f32, f32::max);
    (max_right + 50.0, 100.0)
}

/// Format a connection endpoint for display, omitting "default" or empty port names.
fn format_endpoint(node: &str, port: &str) -> String {
    if port.is_empty() || port == "default" || port == "output" {
        node.to_string()
    } else {
        format!("{node}/{port}")
    }
}

fn save_function_definition(viewer: &FunctionViewer) -> Result<(), String> {
    let dir = viewer
        .toml_path
        .parent()
        .ok_or_else(|| "Invalid path".to_string())?;
    std::fs::create_dir_all(dir).map_err(|e| format!("Could not create directory: {e}"))?;

    // 1. Write the function definition TOML
    let mut toml = format!(
        "function = \"{}\"\nsource = \"{}\"\ntype = \"rust\"\n",
        viewer.name, viewer.source_file
    );
    for input in &viewer.inputs {
        let dtype = input.datatypes.first().map_or("", String::as_str);
        if input.name.is_empty() || input.name == "input" || input.name == "name" {
            toml.push_str(&format!("\n[[input]]\ntype = \"{dtype}\"\n"));
        } else {
            toml.push_str(&format!(
                "\n[[input]]\nname = \"{}\"\ntype = \"{dtype}\"\n",
                input.name
            ));
        }
    }
    for output in &viewer.outputs {
        let dtype = output.datatypes.first().map_or("", String::as_str);
        if output.name.is_empty() || output.name == "output" || output.name == "name" {
            toml.push_str(&format!("\n[[output]]\ntype = \"{dtype}\"\n"));
        } else {
            toml.push_str(&format!(
                "\n[[output]]\nname = \"{}\"\ntype = \"{dtype}\"\n",
                output.name
            ));
        }
    }
    std::fs::write(&viewer.toml_path, &toml)
        .map_err(|e| format!("Could not write {}: {e}", viewer.toml_path.display()))?;

    // 2. Generate skeleton .rs if it doesn't exist
    let rs_path = dir.join(&viewer.source_file);
    if !rs_path.exists() {
        let input_count = viewer.inputs.len();
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
            input_bindings = (0..input_count)
                .map(|i| format!("    let _input{i} = &inputs[{i}];\n"))
                .collect::<String>(),
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
            name = viewer.name,
            source = stem,
        );
        std::fs::write(&cargo_path, &cargo)
            .map_err(|e| format!("Could not write {}: {e}", cargo_path.display()))?;
    }

    Ok(())
}

fn editor_prefs_path(flow_path: &Path) -> PathBuf {
    let mut p = flow_path.to_path_buf();
    let name = p
        .file_name()
        .map(|n| format!(".{}.flowedit", n.to_string_lossy()))
        .unwrap_or_else(|| ".flowedit".to_string());
    p.set_file_name(name);
    p
}

fn save_editor_prefs(flow_path: &Path, size: Option<iced::Size>, position: Option<iced::Point>) {
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

struct EditorPrefs {
    width: f32,
    height: f32,
    x: Option<f32>,
    y: Option<f32>,
}

fn load_editor_prefs(flow_path: &Path) -> Option<EditorPrefs> {
    let prefs_path = editor_prefs_path(flow_path);
    let content = std::fs::read_to_string(prefs_path).ok()?;
    let val: serde_json::Value = serde_json::from_str(&content).ok()?;
    let w = val.get("width")?.as_f64()? as f32;
    let h = val.get("height")?.as_f64()? as f32;
    let x = val.get("x").and_then(|v| v.as_f64()).map(|v| v as f32);
    let y = val.get("y").and_then(|v| v.as_f64()).map(|v| v as f32);
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
    use std::collections::HashMap;

    fn test_node(alias: &str, source: &str) -> NodeLayout {
        NodeLayout {
            alias: alias.into(),
            source: source.into(),
            x: 100.0,
            y: 100.0,
            width: 180.0,
            height: 120.0,
            inputs: Vec::new(),
            outputs: Vec::new(),
            initializers: HashMap::new(),
        }
    }

    #[test]
    fn unique_alias_no_conflict() {
        let nodes = vec![test_node("add", "lib://test")];
        assert_eq!(generate_unique_alias("subtract", &nodes), "subtract");
    }

    #[test]
    fn unique_alias_with_conflict() {
        let nodes = vec![test_node("add", "lib://test")];
        assert_eq!(generate_unique_alias("add", &nodes), "add_2");
    }

    #[test]
    fn unique_alias_multiple_conflicts() {
        let nodes = vec![
            test_node("add", "lib://test"),
            test_node("add_2", "lib://test"),
        ];
        assert_eq!(generate_unique_alias("add", &nodes), "add_3");
    }

    #[test]
    fn next_position_empty() {
        let (x, y) = next_node_position(&[]);
        assert!((x - 100.0).abs() < 0.01);
        assert!((y - 100.0).abs() < 0.01);
    }

    #[test]
    fn next_position_after_nodes() {
        let nodes = vec![test_node("a", "lib://test")];
        let (x, _y) = next_node_position(&nodes);
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
    fn sync_flow_definition_preserves_nodes() {
        let mut win = WindowState {
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
            flow_definition: FlowDefinition::default(),
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
        };

        sync_flow_definition(&mut win);
        assert_eq!(win.flow_definition.process_refs.len(), 2);
        assert_eq!(win.flow_definition.name, "test");
    }

    #[test]
    fn record_and_undo_edit() {
        let mut win = WindowState {
            kind: WindowKind::FlowEditor,
            flow_name: String::from("test"),
            nodes: vec![test_node("a", "lib://test")],
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
            flow_definition: FlowDefinition::default(),
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
        };

        // Move node
        win.nodes[0].x = 200.0;
        win.nodes[0].y = 300.0;
        record_edit(
            &mut win,
            EditAction::MoveNode {
                index: 0,
                old_x: 100.0,
                old_y: 100.0,
                new_x: 200.0,
                new_y: 300.0,
            },
        );
        assert_eq!(win.unsaved_edits, 1);

        // Undo
        apply_undo(&mut win);
        assert!((win.nodes[0].x - 100.0).abs() < 0.01);
        assert!((win.nodes[0].y - 100.0).abs() < 0.01);

        // Redo
        apply_redo(&mut win);
        assert!((win.nodes[0].x - 200.0).abs() < 0.01);
        assert!((win.nodes[0].y - 300.0).abs() < 0.01);
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
            flow_definition: FlowDefinition::default(),
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

    fn test_app() -> (FlowEdit, window::Id) {
        let win_id = window::Id::unique();
        let app = FlowEdit {
            windows: HashMap::from([(win_id, test_win_state())]),
            root_window: Some(win_id),
            focused_window: Some(win_id),
            library_tree: LibraryTree {
                libraries: Vec::new(),
            },
            root_flow_path: None,
            show_lib_paths: false,
            lib_paths: Vec::new(),
            library_cache: HashMap::new(),
            lib_definitions: HashMap::new(),
            context_definitions: HashMap::new(),
        };
        (app, win_id)
    }

    #[test]
    fn update_zoom_in() {
        let (mut app, win_id) = test_app();
        let old_zoom = app
            .windows
            .get(&win_id)
            .map(|w| w.canvas_state.zoom)
            .unwrap_or(1.0);
        app.update(Message::ZoomIn(win_id));
        let new_zoom = app
            .windows
            .get(&win_id)
            .map(|w| w.canvas_state.zoom)
            .unwrap_or(1.0);
        assert!(new_zoom > old_zoom);
    }

    #[test]
    fn update_zoom_out() {
        let (mut app, win_id) = test_app();
        let old_zoom = app
            .windows
            .get(&win_id)
            .map(|w| w.canvas_state.zoom)
            .unwrap_or(1.0);
        app.update(Message::ZoomOut(win_id));
        let new_zoom = app
            .windows
            .get(&win_id)
            .map(|w| w.canvas_state.zoom)
            .unwrap_or(1.0);
        assert!(new_zoom < old_zoom);
    }

    #[test]
    fn update_toggle_auto_fit() {
        let (mut app, win_id) = test_app();
        app.windows
            .get_mut(&win_id)
            .map(|w| w.auto_fit_enabled = false);
        app.update(Message::ToggleAutoFit(win_id));
        assert!(app
            .windows
            .get(&win_id)
            .map_or(false, |w| w.auto_fit_enabled));
    }

    #[test]
    fn update_canvas_select_node() {
        let (mut app, win_id) = test_app();
        app.update(Message::WindowCanvas(
            win_id,
            CanvasMessage::Selected(Some(0)),
        ));
        assert_eq!(
            app.windows.get(&win_id).and_then(|w| w.selected_node),
            Some(0)
        );
    }

    #[test]
    fn update_canvas_deselect() {
        let (mut app, win_id) = test_app();
        app.windows
            .get_mut(&win_id)
            .map(|w| w.selected_node = Some(0));
        app.update(Message::WindowCanvas(win_id, CanvasMessage::Selected(None)));
        assert_eq!(app.windows.get(&win_id).and_then(|w| w.selected_node), None);
    }

    #[test]
    fn update_canvas_move_node() {
        let (mut app, win_id) = test_app();
        app.update(Message::WindowCanvas(
            win_id,
            CanvasMessage::Moved(0, 200.0, 300.0),
        ));
        let node = app.windows.get(&win_id).and_then(|w| w.nodes.first());
        assert!((node.map(|n| n.x).unwrap_or(0.0) - 200.0).abs() < 0.01);
        assert!((node.map(|n| n.y).unwrap_or(0.0) - 300.0).abs() < 0.01);
    }

    #[test]
    fn update_canvas_move_completed_records_history() {
        let (mut app, win_id) = test_app();
        app.update(Message::WindowCanvas(
            win_id,
            CanvasMessage::MoveCompleted(0, 100.0, 100.0, 200.0, 300.0),
        ));
        assert_eq!(app.windows.get(&win_id).map(|w| w.unsaved_edits), Some(1));
    }

    #[test]
    fn update_canvas_delete_node() {
        let (mut app, win_id) = test_app();
        assert_eq!(app.windows.get(&win_id).map(|w| w.nodes.len()), Some(2));
        app.update(Message::WindowCanvas(win_id, CanvasMessage::Deleted(0)));
        assert_eq!(app.windows.get(&win_id).map(|w| w.nodes.len()), Some(1));
        assert_eq!(app.windows.get(&win_id).map(|w| w.unsaved_edits), Some(1));
    }

    #[test]
    fn update_canvas_create_connection() {
        let (mut app, win_id) = test_app();
        app.update(Message::WindowCanvas(
            win_id,
            CanvasMessage::ConnectionCreated {
                from_node: "add".into(),
                from_port: "".into(),
                to_node: "stdout".into(),
                to_port: "".into(),
            },
        ));
        assert_eq!(app.windows.get(&win_id).map(|w| w.edges.len()), Some(1));
        assert_eq!(app.windows.get(&win_id).map(|w| w.unsaved_edits), Some(1));
    }

    #[test]
    fn update_canvas_select_connection() {
        let (mut app, win_id) = test_app();
        // Add a connection first
        if let Some(win) = app.windows.get_mut(&win_id) {
            win.edges.push(EdgeLayout::new(
                "add".into(),
                "".into(),
                "stdout".into(),
                "".into(),
            ));
        }
        app.update(Message::WindowCanvas(
            win_id,
            CanvasMessage::ConnectionSelected(Some(0)),
        ));
        assert_eq!(
            app.windows.get(&win_id).and_then(|w| w.selected_connection),
            Some(0)
        );
    }

    #[test]
    fn update_canvas_delete_connection() {
        let (mut app, win_id) = test_app();
        if let Some(win) = app.windows.get_mut(&win_id) {
            win.edges.push(EdgeLayout::new(
                "add".into(),
                "".into(),
                "stdout".into(),
                "".into(),
            ));
        }
        app.update(Message::WindowCanvas(
            win_id,
            CanvasMessage::ConnectionDeleted(0),
        ));
        assert_eq!(app.windows.get(&win_id).map(|w| w.edges.len()), Some(0));
    }

    #[test]
    fn update_undo_redo_cycle() {
        let (mut app, win_id) = test_app();
        // Move node and record
        app.update(Message::WindowCanvas(
            win_id,
            CanvasMessage::Moved(0, 200.0, 300.0),
        ));
        app.update(Message::WindowCanvas(
            win_id,
            CanvasMessage::MoveCompleted(0, 100.0, 100.0, 200.0, 300.0),
        ));
        assert_eq!(app.windows.get(&win_id).map(|w| w.unsaved_edits), Some(1));

        // Undo
        app.update(Message::Undo);
        let node = app.windows.get(&win_id).and_then(|w| w.nodes.first());
        assert!((node.map(|n| n.x).unwrap_or(0.0) - 100.0).abs() < 0.01);

        // Redo
        app.update(Message::Redo);
        let node = app.windows.get(&win_id).and_then(|w| w.nodes.first());
        assert!((node.map(|n| n.x).unwrap_or(0.0) - 200.0).abs() < 0.01);
    }

    #[test]
    fn update_toggle_metadata() {
        let (mut app, win_id) = test_app();
        assert!(!app.windows.get(&win_id).map_or(true, |w| w.show_metadata));
        app.update(Message::ToggleMetadataEditor(win_id));
        assert!(app.windows.get(&win_id).map_or(false, |w| w.show_metadata));
        app.update(Message::ToggleMetadataEditor(win_id));
        assert!(!app.windows.get(&win_id).map_or(true, |w| w.show_metadata));
    }

    #[test]
    fn update_flow_name_changed() {
        let (mut app, win_id) = test_app();
        app.update(Message::FlowNameChanged(win_id, "new_name".into()));
        assert_eq!(
            app.windows.get(&win_id).map(|w| w.flow_name.as_str()),
            Some("new_name")
        );
        assert_eq!(app.windows.get(&win_id).map(|w| w.unsaved_edits), Some(1));
    }

    #[test]
    fn update_flow_version_changed() {
        let (mut app, win_id) = test_app();
        app.update(Message::FlowVersionChanged(win_id, "2.0.0".into()));
        assert_eq!(
            app.windows
                .get(&win_id)
                .map(|w| w.flow_definition.metadata.version.as_str()),
            Some("2.0.0")
        );
    }

    #[test]
    fn update_flow_description_changed() {
        let (mut app, win_id) = test_app();
        app.update(Message::FlowDescriptionChanged(
            win_id,
            "A test flow".into(),
        ));
        assert_eq!(
            app.windows
                .get(&win_id)
                .map(|w| w.flow_definition.metadata.description.as_str()),
            Some("A test flow")
        );
    }

    #[test]
    fn update_flow_authors_changed() {
        let (mut app, win_id) = test_app();
        app.update(Message::FlowAuthorsChanged(win_id, "Alice, Bob".into()));
        let authors = app
            .windows
            .get(&win_id)
            .map(|w| w.flow_definition.metadata.authors.clone())
            .unwrap_or_default();
        assert_eq!(authors, vec!["Alice", "Bob"]);
    }

    #[test]
    fn update_flow_add_input() {
        let (mut app, win_id) = test_app();
        app.update(Message::FlowAddInput(win_id));
        assert_eq!(
            app.windows.get(&win_id).map(|w| w.flow_inputs.len()),
            Some(1)
        );
        assert_eq!(app.windows.get(&win_id).map(|w| w.unsaved_edits), Some(1));
    }

    #[test]
    fn update_flow_add_output() {
        let (mut app, win_id) = test_app();
        app.update(Message::FlowAddOutput(win_id));
        assert_eq!(
            app.windows.get(&win_id).map(|w| w.flow_outputs.len()),
            Some(1)
        );
    }

    #[test]
    fn update_flow_delete_input() {
        let (mut app, win_id) = test_app();
        app.update(Message::FlowAddInput(win_id));
        app.update(Message::FlowDeleteInput(win_id, 0));
        assert_eq!(
            app.windows.get(&win_id).map(|w| w.flow_inputs.len()),
            Some(0)
        );
    }

    #[test]
    fn update_flow_input_name_changed() {
        let (mut app, win_id) = test_app();
        app.update(Message::FlowAddInput(win_id));
        app.update(Message::FlowInputNameChanged(win_id, 0, "data".into()));
        assert_eq!(
            app.windows
                .get(&win_id)
                .and_then(|w| w.flow_inputs.first().map(|p| p.name.as_str())),
            Some("data")
        );
    }

    #[test]
    fn update_window_focused() {
        let (mut app, win_id) = test_app();
        let other_id = window::Id::unique();
        app.update(Message::WindowFocused(other_id));
        assert_eq!(app.focused_window, Some(other_id));
    }

    #[test]
    fn update_window_resized() {
        let (mut app, win_id) = test_app();
        app.update(Message::WindowResized(
            win_id,
            iced::Size::new(800.0, 600.0),
        ));
        let size = app.windows.get(&win_id).and_then(|w| w.last_size);
        assert!(size.is_some());
        assert!((size.map(|s| s.width).unwrap_or(0.0) - 800.0).abs() < 0.01);
    }

    #[test]
    fn update_window_moved() {
        let (mut app, win_id) = test_app();
        app.update(Message::WindowMoved(win_id, iced::Point::new(100.0, 200.0)));
        let pos = app.windows.get(&win_id).and_then(|w| w.last_position);
        assert!(pos.is_some());
    }

    #[test]
    fn update_toggle_lib_paths() {
        let (mut app, _win_id) = test_app();
        assert!(!app.show_lib_paths);
        app.update(Message::ToggleLibPaths);
        assert!(app.show_lib_paths);
        app.update(Message::ToggleLibPaths);
        assert!(!app.show_lib_paths);
    }

    #[test]
    fn update_context_menu() {
        let (mut app, win_id) = test_app();
        app.update(Message::WindowCanvas(
            win_id,
            CanvasMessage::ContextMenu(100.0, 200.0),
        ));
        assert!(app
            .windows
            .get(&win_id)
            .and_then(|w| w.context_menu)
            .is_some());
        // Clicking deselects context menu
        app.update(Message::WindowCanvas(win_id, CanvasMessage::Selected(None)));
        assert!(app
            .windows
            .get(&win_id)
            .and_then(|w| w.context_menu)
            .is_none());
    }

    #[test]
    fn update_canvas_resize_node() {
        let (mut app, win_id) = test_app();
        app.update(Message::WindowCanvas(
            win_id,
            CanvasMessage::Resized(0, 50.0, 50.0, 200.0, 150.0),
        ));
        let node = app.windows.get(&win_id).and_then(|w| w.nodes.first());
        assert!((node.map(|n| n.width).unwrap_or(0.0) - 200.0).abs() < 0.01);
        assert!((node.map(|n| n.height).unwrap_or(0.0) - 150.0).abs() < 0.01);
    }

    #[test]
    fn update_initializer_type_changed() {
        let (mut app, win_id) = test_app();
        // Open initializer editor
        if let Some(win) = app.windows.get_mut(&win_id) {
            win.initializer_editor = Some(InitializerEditor {
                node_index: 0,
                port_name: "i1".into(),
                init_type: "none".into(),
                value_text: String::new(),
            });
        }
        app.update(Message::InitializerTypeChanged(win_id, "once".into()));
        let init_type = app
            .windows
            .get(&win_id)
            .and_then(|w| w.initializer_editor.as_ref())
            .map(|e| e.init_type.as_str());
        assert_eq!(init_type, Some("once"));
    }

    #[test]
    fn update_initializer_cancel() {
        let (mut app, win_id) = test_app();
        if let Some(win) = app.windows.get_mut(&win_id) {
            win.initializer_editor = Some(InitializerEditor {
                node_index: 0,
                port_name: "i1".into(),
                init_type: "once".into(),
                value_text: "42".into(),
            });
        }
        app.update(Message::InitializerCancel(win_id));
        assert!(app
            .windows
            .get(&win_id)
            .and_then(|w| w.initializer_editor.as_ref())
            .is_none());
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("flowedit_tests").join(name);
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn save_and_load_flow_roundtrip() {
        let dir = temp_dir("save_load");
        let path = dir.join("test.toml");

        let mut flow = FlowDefinition::default();
        flow.name = "roundtrip_test".into();
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

        let edges = vec![EdgeLayout::new(
            "add1".into(),
            "".into(),
            "add1".into(),
            "i1".into(),
        )];

        save_flow_toml(&flow, &edges, &path).expect("save failed");

        let contents = std::fs::read_to_string(&path).expect("read failed");
        assert!(contents.contains("flow = \"roundtrip_test\""));
        assert!(contents.contains("version = \"1.0.0\""));
        assert!(contents.contains("Test Author"));
        assert!(contents.contains("lib://flowstdlib/math/add"));

        let loaded = load_flow(&path).expect("load failed");
        assert_eq!(loaded.name, "roundtrip_test");
        assert_eq!(loaded.nodes.len(), 1);
        assert_eq!(loaded.edges.len(), 1);
        assert_eq!(loaded.flow_def.metadata.version, "1.0.0");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_flow_with_metadata() {
        let dir = temp_dir("metadata");
        let path = dir.join("meta.toml");

        let mut flow = FlowDefinition::default();
        flow.name = "meta_flow".into();
        flow.metadata.description = "A test description".into();

        save_flow_toml(&flow, &[], &path).expect("save failed");
        let contents = std::fs::read_to_string(&path).expect("read failed");
        assert!(contents.contains("description = \"A test description\""));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_flow_with_initializers() {
        let dir = temp_dir("initializers");
        let path = dir.join("init.toml");

        let mut flow = FlowDefinition::default();
        flow.name = "init_flow".into();
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

        save_flow_toml(&flow, &[], &path).expect("save failed");
        let contents = std::fs::read_to_string(&path).expect("read failed");
        assert!(contents.contains("input.start"));
        assert!(contents.contains("once"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_flow_with_connections() {
        let dir = temp_dir("connections");
        let path = dir.join("conn.toml");

        let mut flow = FlowDefinition::default();
        flow.name = "conn_flow".into();
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

        let edges = vec![EdgeLayout::new(
            "a".into(),
            "out".into(),
            "b".into(),
            "in".into(),
        )];

        save_flow_toml(&flow, &edges, &path).expect("save failed");
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
    fn perform_save_updates_state() {
        let dir = temp_dir("perform_save");
        let path = dir.join("saved.toml");

        let mut win = test_win_state();
        win.unsaved_edits = 5;
        win.flow_name = "saved_flow".into();

        perform_save(&mut win, &path);
        assert_eq!(win.unsaved_edits, 0);
        assert_eq!(win.file_path, Some(path.clone()));

        let contents = std::fs::read_to_string(&path).expect("read failed");
        assert!(contents.contains("flow = \"saved_flow\""));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_function_definition_creates_files() {
        let dir = temp_dir("func_def");
        let toml_path = dir.join("myfunc.toml");

        let viewer = FunctionViewer {
            name: "myfunc".into(),
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
            source_file: "existing.rs".into(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            rs_content: String::new(),
            docs_content: None,
            active_tab: 0,
            toml_path,
        };

        save_function_definition(&viewer).expect("save failed");

        // Existing .rs should NOT be overwritten
        let rs = std::fs::read_to_string(&rs_path).expect("read rs");
        assert_eq!(rs, "// existing code");

        let _ = std::fs::remove_dir_all(&dir);
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
