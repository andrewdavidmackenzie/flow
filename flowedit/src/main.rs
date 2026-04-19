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

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, Command as ClapCommand};
use iced::keyboard;
use iced::widget::{button, container, pick_list, stack, text_input, Column, Row, Text};
use iced::window;
use iced::{Color, Element, Fill, Subscription, Task, Theme};
use log::info;
use simpath::Simpath;
use url::Url;

use flowcore::deserializers::deserializer::get;
use flowcore::meta_provider::MetaProvider;
use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::input::InputInitializer;
use flowcore::model::name::HasName;
use flowcore::model::process::Process;
use flowcore::model::process_reference::ProcessReference;
mod canvas_view;
mod history;
mod library_panel;
use canvas_view::{
    build_edge_layouts, build_node_layouts, derive_short_name, CanvasMessage, EdgeLayout,
    FlowCanvasState, NodeLayout, PortInfo,
};
use history::{EditAction, EditHistory};
use library_panel::{LibraryMessage, LibraryTree};

/// Messages handled by the flowedit application
#[derive(Debug, Clone)]
enum Message {
    /// A message from the interactive canvas, tagged with its window ID
    WindowCanvas(window::Id, CanvasMessage),
    /// A message from the library side panel, tagged with the originating window ID
    Library(window::Id, LibraryMessage),
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
    /// A window close was requested
    CloseRequested(window::Id),
    /// Close the currently focused window (Cmd+W)
    CloseActiveWindow,
    /// A window received focus
    WindowFocused(window::Id),
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

/// Per-window state for the flow editor.
struct WindowState {
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

        let (flow_name, nodes, edges, status, file_path, flow_definition) =
            if let Some(flow_path_str) = matches.get_one::<String>("flow-file") {
                let flow_path = PathBuf::from(flow_path_str);
                match load_flow(&flow_path) {
                    Ok((name, node_list, edge_list, flow_def)) => {
                        let nc = node_list.len();
                        let ec = edge_list.len();
                        (
                            name,
                            node_list,
                            edge_list,
                            format!("Ready - {nc} nodes, {ec} connections"),
                            Some(flow_path),
                            flow_def,
                        )
                    }
                    Err(e) => (
                        String::from("(error)"),
                        Vec::new(),
                        Vec::new(),
                        format!("Error loading flow: {e}"),
                        None,
                        FlowDefinition::default(),
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
                )
            };

        let has_nodes = !nodes.is_empty();
        let library_tree = LibraryTree::scan();

        // Open the root window via daemon API
        let (root_id, open_task) = window::open(window::Settings {
            size: iced::Size::new(1024.0, 768.0),
            ..Default::default()
        });

        let (fi, fo) = extract_ports(&flow_definition.inputs, &flow_definition.outputs);
        let win_state = WindowState {
            flow_name,
            nodes,
            edges,
            canvas_state: FlowCanvasState::default(),
            status,
            selected_node: None,
            selected_connection: None,
            auto_fit_pending: has_nodes,
            auto_fit_enabled: true, // Start in auto-fit mode
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
        };

        let mut windows = HashMap::new();
        windows.insert(root_id, win_state);

        let app = FlowEdit {
            windows,
            root_window: Some(root_id),
            focused_window: Some(root_id),
            library_tree,
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
                }
            }
            Message::Library(win_id, ref lib_msg) => {
                if let Some((source, func_name)) = self.library_tree.update(lib_msg) {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        add_library_function(win, &source, &func_name);
                    }
                }
            }
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
                if let Some(win) = self.root_window.and_then(|id| self.windows.get_mut(&id)) {
                    perform_open(win);
                }
            }
            Message::New => {
                if let Some(win) = self.root_window.and_then(|id| self.windows.get_mut(&id)) {
                    perform_new(win);
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
            Message::CloseRequested(id) => {
                self.windows.remove(&id);
                if self.root_window == Some(id) || self.windows.is_empty() {
                    return iced::exit();
                }
                return window::close(id);
            }
            Message::CloseActiveWindow => {
                if let Some(id) = self.focused_window.or(self.root_window) {
                    self.windows.remove(&id);
                    if self.root_window == Some(id) || self.windows.is_empty() {
                        return iced::exit();
                    }
                    return window::close(id);
                }
            }
        }
        Task::none()
    }

    /// Build the view: a canvas area with zoom controls overlaid, and a status bar at the bottom.
    fn view(&self, window_id: window::Id) -> Element<'_, Message> {
        let Some(win) = self.windows.get(&window_id) else {
            return Text::new("Window not found").into();
        };

        let canvas = win
            .canvas_state
            .view(
                &win.nodes,
                &win.edges,
                &win.flow_inputs,
                &win.flow_outputs,
                win.auto_fit_pending,
                win.auto_fit_enabled,
            )
            .map(move |msg| Message::WindowCanvas(window_id, msg));

        let btn_width = 40;
        let zoom_controls = container(
            container(
                Column::new()
                    .spacing(4)
                    .push(
                        button(Text::new("+").center())
                            .on_press(Message::ZoomIn(window_id))
                            .width(btn_width)
                            .style(button::secondary),
                    )
                    .push(
                        button(Text::new("\u{2212}").center())
                            .on_press(Message::ZoomOut(window_id))
                            .width(btn_width)
                            .style(button::secondary),
                    )
                    .push(
                        button(Text::new("Fit").center())
                            .on_press(Message::ToggleAutoFit(window_id))
                            .width(btn_width)
                            .style(if win.auto_fit_enabled {
                                button::primary
                            } else {
                                button::secondary
                            }),
                    ),
            )
            .padding(6)
            .style(container::rounded_box),
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
                top: ty + 20.0,
                right: 0.0,
                bottom: 0.0,
                left: tx + 16.0,
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

        let canvas_with_controls = stack(canvas_stack);

        let library_panel = self
            .library_tree
            .view()
            .map(move |msg| Message::Library(window_id, msg));

        let main_content = Row::new()
            .push(library_panel)
            .push(container(canvas_with_controls).width(Fill).height(Fill));

        let edit_indicator = if win.unsaved_edits > 0 {
            format!("  |  {} unsaved edit(s)", win.unsaved_edits)
        } else {
            String::from("  |  saved")
        };

        // Build status bar — compile button only for root windows
        let status_bar: Row<'_, Message> = if win.is_root {
            let mut compile_btn = button(Text::new("\u{1F528} Build").size(14).center())
                .padding([6, 14])
                .style(if win.nodes.is_empty() {
                    button::secondary
                } else {
                    button::primary
                });
            if !win.nodes.is_empty() {
                compile_btn = compile_btn.on_press(Message::Compile);
            }

            Row::new()
                .spacing(8)
                .push(Text::new(format!("{}{}", win.status, edit_indicator)).size(14))
                .push(iced::widget::Space::new().width(Fill))
                .push(compile_btn)
        } else {
            Row::new()
                .spacing(8)
                .push(Text::new(format!("{}{}", win.status, edit_indicator)).size(14))
                .push(iced::widget::Space::new().width(Fill))
        };

        let mut layout = Column::new().push(container(main_content).width(Fill).height(Fill));

        layout = layout.push(container(status_bar).width(Fill).padding(5));
        layout.into()
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
                _ => None,
            },
            _ => None,
        });

        let close_sub = window::close_requests().map(Message::CloseRequested);

        let focus_sub = iced::event::listen_with(|event, _status, id| {
            if let iced::Event::Window(iced::window::Event::Focused) = event {
                Some(Message::WindowFocused(id))
            } else {
                None
            }
        });

        Subscription::batch(vec![keyboard_sub, close_sub, focus_sub])
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
        if let Ok(contents) = std::fs::read_to_string(&path) {
            if let Ok(url) = Url::from_file_path(&path) {
                if let Ok(deserializer) = get::<Process>(&url) {
                    if let Ok(Process::FunctionProcess(_)) =
                        deserializer.deserialize(&contents, Some(&url))
                    {
                        if let Some(win) = self.windows.get_mut(&parent_win_id) {
                            win.status =
                                format!("'{}' is a function -- viewer not yet implemented", source);
                        }
                        return Task::none();
                    }
                }
            }
        }

        // Load the sub-flow and open it in a new window
        match load_flow(&path) {
            Ok((name, nodes, edges, flow_def)) => {
                let has_nodes = !nodes.is_empty();
                let cascade = self.windows.len() as f32;
                let (new_id, open_task) = window::open(window::Settings {
                    size: iced::Size::new(1024.0, 768.0),
                    position: window::Position::Specific(iced::Point::new(
                        80.0 + cascade * 30.0,
                        60.0 + cascade * 30.0,
                    )),
                    ..Default::default()
                });
                let nc = nodes.len();
                let ec = edges.len();
                let (fi, fo) = extract_ports(&flow_def.inputs, &flow_def.outputs);
                let child = WindowState {
                    flow_name: name,
                    nodes,
                    edges,
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
                    flow_definition: flow_def,
                    tooltip: None,
                    initializer_editor: None,
                    is_root: false,
                    flow_inputs: fi,
                    flow_outputs: fo,
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
fn perform_open(win: &mut WindowState) {
    let dialog = rfd::FileDialog::new().add_filter("Flow", &["toml"]);
    if let Some(path) = dialog.pick_file() {
        match load_flow(&path) {
            Ok((name, node_list, edge_list, flow_def)) => {
                let nc = node_list.len();
                let ec = edge_list.len();
                let (fi, fo) = extract_ports(&flow_def.inputs, &flow_def.outputs);
                win.flow_name = name;
                win.nodes = node_list;
                win.edges = edge_list;
                win.flow_definition = flow_def;
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
            }
            Err(e) => {
                win.status = format!("Open failed: {e}");
            }
        }
    }
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

    // Resolve port info from the function definition
    let (inputs, outputs) = resolve_single_function_ports(source, None);

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
    let candidate = base_dir.join(source);
    if candidate.exists() {
        return Some(candidate);
    }
    let with_ext = base_dir.join(format!("{source}.toml"));
    if with_ext.exists() {
        return Some(with_ext);
    }
    // Try as directory/default.toml
    let dir_default = base_dir.join(source).join("default.toml");
    if dir_default.exists() {
        return Some(dir_default);
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

/// Resolve port info for a single function/flow from its source string.
///
/// If `base_url` is provided, relative source paths are resolved against it.
/// For `lib://` and `context://` sources, the base URL is not needed.
fn resolve_single_function_ports(
    source: &str,
    base_url: Option<&Url>,
) -> (Vec<PortInfo>, Vec<PortInfo>) {
    use flowcore::provider::Provider;

    let provider = build_meta_provider();

    // Parse the source as a URL; for relative paths, resolve against the base URL
    let source_url = match Url::parse(source) {
        Ok(u) => u,
        Err(_) => {
            match base_url.and_then(|base| base.join(source).ok()) {
                Some(u) => u,
                None => {
                    info!("resolve_single_function_ports: could not resolve relative source '{source}'");
                    return (Vec::new(), Vec::new());
                }
            }
        }
    };

    let (resolved_url, _) = match provider.resolve_url(&source_url, "default", &["toml"]) {
        Ok(r) => r,
        Err(e) => {
            info!("resolve_single_function_ports: could not resolve '{source_url}': {e}");
            return (Vec::new(), Vec::new());
        }
    };

    let content_bytes = match provider.get_contents(&resolved_url) {
        Ok(bytes) => bytes,
        Err(e) => {
            info!(
                "resolve_single_function_ports: could not get contents from '{resolved_url}': {e}"
            );
            return (Vec::new(), Vec::new());
        }
    };
    let content = String::from_utf8_lossy(&content_bytes);

    let deserializer = match get::<Process>(&resolved_url) {
        Ok(d) => d,
        Err(_) => return (Vec::new(), Vec::new()),
    };

    match deserializer.deserialize(&content, Some(&resolved_url)) {
        Ok(Process::FunctionProcess(func)) => extract_ports(&func.inputs, &func.outputs),
        Ok(Process::FlowProcess(flow)) => extract_ports(&flow.inputs, &flow.outputs),
        Err(_) => (Vec::new(), Vec::new()),
    }
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

/// Load a flow definition file and return the flow name, node layouts, edge layouts,
/// and the original `FlowDefinition` for use when saving.
fn load_flow(
    path: &PathBuf,
) -> Result<(String, Vec<NodeLayout>, Vec<EdgeLayout>, FlowDefinition), String> {
    let abs_path = if path.is_absolute() {
        path.clone()
    } else {
        std::env::current_dir()
            .map_err(|e| format!("Could not get current directory: {e}"))?
            .join(path)
    };

    let url =
        Url::from_file_path(&abs_path).map_err(|()| format!("Invalid file path: {abs_path:?}"))?;

    let contents =
        std::fs::read_to_string(&abs_path).map_err(|e| format!("Could not read file: {e}"))?;

    let deserializer =
        get::<Process>(&url).map_err(|e| format!("Could not get deserializer: {e}"))?;

    let process = deserializer
        .deserialize(&contents, Some(&url))
        .map_err(|e| format!("Could not parse flow definition: {e}"))?;

    match process {
        Process::FlowProcess(flow) => {
            // Resolve port definitions for each subprocess by loading its definition
            let mut resolved_ports = HashMap::new();
            for pref in &flow.process_refs {
                let alias = if pref.alias.is_empty() {
                    derive_short_name(&pref.source)
                } else {
                    pref.alias.to_string()
                };
                let (inputs, outputs) =
                    resolve_single_function_ports(&pref.source, Some(&url));
                info!(
                    "Resolved '{}' ({}): {} inputs, {} outputs",
                    alias,
                    pref.source,
                    inputs.len(),
                    outputs.len()
                );
                resolved_ports.insert(alias, (inputs, outputs));
            }

            let edges = build_edge_layouts(&flow.connections);
            let nodes =
                build_node_layouts(&flow.process_refs, &flow.connections, &resolved_ports);
            let name = flow.name.clone();
            Ok((name, nodes, edges, flow))
        }
        Process::FunctionProcess(_) => Err(
            "The specified file defines a Function, not a Flow. flowedit requires a flow definition."
                .to_string(),
        ),
    }
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
