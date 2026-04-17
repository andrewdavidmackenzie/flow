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

use std::collections::HashMap;
use std::path::PathBuf;

use clap::{Arg, Command as ClapCommand};
use iced::keyboard;
use iced::widget::{button, container, stack, Column, Row, Text};
use iced::{Element, Fill, Subscription, Task};
use log::{info, warn};
use simpath::Simpath;
use url::Url;

use flowcore::deserializers::deserializer::get;
use flowcore::meta_provider::MetaProvider;
use flowcore::model::name::HasName;
use flowcore::model::process::Process;
mod canvas_view;
mod history;
use canvas_view::{
    build_edge_layouts, build_node_layouts, CanvasMessage, EdgeLayout, FlowCanvasState, NodeLayout,
    PortInfo,
};
use history::{EditAction, EditHistory};

/// Messages handled by the flowedit application
#[derive(Debug, Clone)]
enum Message {
    /// A message from the interactive canvas (select, move, delete)
    Canvas(CanvasMessage),
    /// Zoom in by one step
    ZoomIn,
    /// Zoom out by one step
    ZoomOut,
    /// Toggle auto-fit mode
    ToggleAutoFit,
    /// Undo the last edit action
    Undo,
    /// Redo the last undone action
    Redo,
}

/// Top-level application state
struct FlowEdit {
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
}

/// Main entry point for the flowedit binary.
///
/// Parses CLI arguments, loads the flow definition, and launches the iced GUI.
fn main() -> iced::Result {
    env_logger::init();
    iced::application(FlowEdit::new, FlowEdit::update, FlowEdit::view)
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
            .get_matches();

        let (flow_name, nodes, edges, status) =
            if let Some(flow_path_str) = matches.get_one::<String>("flow-file") {
                let flow_path = PathBuf::from(flow_path_str);
                match load_flow(&flow_path) {
                    Ok((name, node_list, edge_list)) => {
                        let nc = node_list.len();
                        let ec = edge_list.len();
                        (
                            name,
                            node_list,
                            edge_list,
                            format!("Ready - {nc} nodes, {ec} connections"),
                        )
                    }
                    Err(e) => (
                        String::from("(error)"),
                        Vec::new(),
                        Vec::new(),
                        format!("Error loading flow: {e}"),
                    ),
                }
            } else {
                (
                    String::from("(new flow)"),
                    Vec::new(),
                    Vec::new(),
                    String::from("Ready"),
                )
            };

        let has_nodes = !nodes.is_empty();
        let app = FlowEdit {
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
        };

        (app, Task::none())
    }

    /// Return the window title.
    fn title(&self) -> String {
        format!("flowedit - {}", self.flow_name)
    }

    /// Handle messages from canvas interactions.
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Canvas(canvas_msg) => match canvas_msg {
                CanvasMessage::Selected(idx) => {
                    self.selected_node = idx;
                    self.selected_connection = None;
                    if let Some(i) = idx {
                        if let Some(node) = self.nodes.get(i) {
                            self.status = format!("Selected: {}", node.alias);
                        }
                    } else {
                        self.status = String::from("Ready");
                    }
                }
                CanvasMessage::Moved(idx, x, y) => {
                    if let Some(node) = self.nodes.get_mut(idx) {
                        node.x = x;
                        node.y = y;
                        self.canvas_state.request_redraw();
                    }
                }
                CanvasMessage::Resized(idx, x, y, w, h) => {
                    if let Some(node) = self.nodes.get_mut(idx) {
                        node.x = x;
                        node.y = y;
                        node.width = w;
                        node.height = h;
                        self.canvas_state.request_redraw();
                    }
                }
                CanvasMessage::MoveCompleted(idx, old_x, old_y, new_x, new_y) => {
                    info!("MoveCompleted: idx={idx}, ({old_x},{old_y}) -> ({new_x},{new_y})");
                    if (old_x - new_x).abs() > 0.5 || (old_y - new_y).abs() > 0.5 {
                        self.record_edit(EditAction::MoveNode {
                            index: idx,
                            old_x,
                            old_y,
                            new_x,
                            new_y,
                        });
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
                    self.record_edit(EditAction::ResizeNode {
                        index: idx,
                        old_x,
                        old_y,
                        old_w,
                        old_h,
                        new_x,
                        new_y,
                        new_w,
                        new_h,
                    });
                }
                CanvasMessage::Deleted(idx) => {
                    if idx < self.nodes.len() {
                        let node = if let Some(node) = self.nodes.get(idx) {
                            node.clone()
                        } else {
                            return Task::none();
                        };
                        let alias = node.alias.clone();
                        let removed_edges: Vec<EdgeLayout> = self
                            .edges
                            .iter()
                            .filter(|e| e.references_node(&alias))
                            .cloned()
                            .collect();
                        self.nodes.remove(idx);
                        self.edges.retain(|e| !e.references_node(&alias));
                        self.record_edit(EditAction::DeleteNode {
                            index: idx,
                            node,
                            removed_edges,
                        });
                        self.selected_node = None;
                        self.selected_connection = None;
                        self.canvas_state.request_redraw();
                        let nc = self.nodes.len();
                        let ec = self.edges.len();
                        self.status = format!("Node deleted - {nc} nodes, {ec} connections");
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
                    self.record_edit(EditAction::CreateConnection { edge: edge.clone() });
                    self.edges.push(edge);
                    self.canvas_state.request_redraw();
                    let nc = self.nodes.len();
                    let ec = self.edges.len();
                    self.status = format!(
                        "Connection created: {from_node}/{from_port} -> {to_node}/{to_port} - {nc} nodes, {ec} connections"
                    );
                }
                CanvasMessage::ConnectionSelected(idx) => {
                    self.selected_connection = idx;
                    self.selected_node = None;
                    if let Some(i) = idx {
                        if let Some(edge) = self.edges.get(i) {
                            self.status = format!(
                                "Connection: {} -> {}",
                                format_endpoint(&edge.from_node, &edge.from_port),
                                format_endpoint(&edge.to_node, &edge.to_port),
                            );
                        }
                    } else {
                        self.status = String::from("Ready");
                    }
                }
                CanvasMessage::ConnectionDeleted(idx) => {
                    if idx < self.edges.len() {
                        let edge = self.edges.remove(idx);
                        self.record_edit(EditAction::DeleteConnection { index: idx, edge });
                        self.selected_connection = None;
                        self.canvas_state.request_redraw();
                        let nc = self.nodes.len();
                        let ec = self.edges.len();
                        self.status = format!("Connection deleted - {nc} nodes, {ec} connections");
                    }
                }
                CanvasMessage::AutoFitViewport(viewport) => {
                    if self.auto_fit_enabled || self.auto_fit_pending {
                        self.canvas_state.auto_fit(&self.nodes, viewport);
                        self.auto_fit_pending = false;
                    }
                }
                CanvasMessage::Pan(dx, dy) => {
                    self.auto_fit_enabled = false; // Manual pan disables auto-fit
                    self.canvas_state.scroll_offset.x += dx;
                    self.canvas_state.scroll_offset.y += dy;
                    self.canvas_state.request_redraw();
                }
                CanvasMessage::ZoomBy(factor) => {
                    self.auto_fit_enabled = false; // Manual zoom disables auto-fit
                    self.canvas_state.zoom = (self.canvas_state.zoom * factor).clamp(0.1, 5.0);
                    self.canvas_state.request_redraw();
                    let pct = (self.canvas_state.zoom * 100.0) as u32;
                    self.status = format!("Zoom: {pct}%");
                }
            },
            Message::ZoomIn => {
                self.auto_fit_enabled = false;
                self.canvas_state.zoom_in();
                let pct = (self.canvas_state.zoom * 100.0) as u32;
                self.status = format!("Zoom: {pct}%");
            }
            Message::ZoomOut => {
                self.auto_fit_enabled = false;
                self.canvas_state.zoom_out();
                let pct = (self.canvas_state.zoom * 100.0) as u32;
                self.status = format!("Zoom: {pct}%");
            }
            Message::ToggleAutoFit => {
                self.auto_fit_enabled = !self.auto_fit_enabled;
                if self.auto_fit_enabled {
                    self.auto_fit_pending = true;
                    self.canvas_state.request_redraw();
                    self.status = String::from("Auto-fit enabled");
                } else {
                    self.status = String::from("Auto-fit disabled");
                }
            }
            Message::Undo => {
                self.apply_undo();
                self.unsaved_edits = (self.unsaved_edits - 1).max(0);
            }
            Message::Redo => {
                self.apply_redo();
                self.unsaved_edits += 1;
            }
        }
        Task::none()
    }

    /// Build the view: a canvas area with zoom controls overlaid, and a status bar at the bottom.
    fn view(&self) -> Element<'_, Message> {
        let canvas = self
            .canvas_state
            .view(
                &self.nodes,
                &self.edges,
                self.auto_fit_pending,
                self.auto_fit_enabled,
            )
            .map(Message::Canvas);

        let btn_width = 40;
        let zoom_controls = container(
            container(
                Column::new()
                    .spacing(4)
                    .push(
                        button(Text::new("+").center())
                            .on_press(Message::ZoomIn)
                            .width(btn_width)
                            .style(button::secondary),
                    )
                    .push(
                        button(Text::new("\u{2212}").center())
                            .on_press(Message::ZoomOut)
                            .width(btn_width)
                            .style(button::secondary),
                    )
                    .push(
                        button(Text::new("Fit").center())
                            .on_press(Message::ToggleAutoFit)
                            .width(btn_width)
                            .style(if self.auto_fit_enabled {
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

        let canvas_with_controls = stack![canvas, zoom_controls];

        let edit_indicator = if self.unsaved_edits > 0 {
            format!("  [{} unsaved]", self.unsaved_edits)
        } else {
            String::new()
        };
        let status_bar: Row<'_, Message> =
            Row::new().push(Text::new(format!("{}{}", self.status, edit_indicator)).size(14));

        Column::new()
            .push(container(canvas_with_controls).width(Fill).height(Fill))
            .push(container(status_bar).width(Fill).padding(5))
            .into()
    }

    /// Listen for Cmd+Z (undo) and Cmd+Shift+Z (redo) keyboard shortcuts.
    fn subscription(&self) -> Subscription<Message> {
        keyboard::listen().filter_map(|event| match event {
            keyboard::Event::KeyPressed {
                key: keyboard::Key::Character(ref c),
                modifiers,
                ..
            } if modifiers.command() => match c.as_str() {
                "z" if modifiers.shift() => Some(Message::Redo),
                "z" => Some(Message::Undo),
                "=" | "+" => Some(Message::ZoomIn),
                "-" => Some(Message::ZoomOut),
                _ => None,
            },
            _ => None,
        })
    }

    /// Record an edit action in the history and increment the unsaved edit count.
    fn record_edit(&mut self, action: EditAction) {
        self.history.record(action);
        self.unsaved_edits += 1;
    }

    /// Apply an undo action — reverse the last edit.
    fn apply_undo(&mut self) {
        if let Some(action) = self.history.undo() {
            match action {
                EditAction::MoveNode {
                    index,
                    old_x,
                    old_y,
                    ..
                } => {
                    if let Some(node) = self.nodes.get_mut(index) {
                        node.x = old_x;
                        node.y = old_y;
                    }
                    self.status = String::from("Undo: move");
                }
                EditAction::ResizeNode {
                    index,
                    old_x,
                    old_y,
                    old_w,
                    old_h,
                    ..
                } => {
                    if let Some(node) = self.nodes.get_mut(index) {
                        node.x = old_x;
                        node.y = old_y;
                        node.width = old_w;
                        node.height = old_h;
                    }
                    self.status = String::from("Undo: resize");
                }
                EditAction::DeleteNode {
                    index,
                    node,
                    removed_edges,
                } => {
                    self.nodes.insert(index, node);
                    self.edges.extend(removed_edges);
                    self.status = String::from("Undo: delete node");
                }
                EditAction::CreateConnection { edge } => {
                    self.edges.retain(|e| {
                        e.from_node != edge.from_node
                            || e.from_port != edge.from_port
                            || e.to_node != edge.to_node
                            || e.to_port != edge.to_port
                    });
                    self.status = String::from("Undo: create connection");
                }
                EditAction::DeleteConnection { index, edge } => {
                    self.edges.insert(index, edge);
                    self.status = String::from("Undo: delete connection");
                }
            }
            self.canvas_state.request_redraw();
        }
    }

    /// Apply a redo action — re-apply the last undone edit.
    fn apply_redo(&mut self) {
        if let Some(action) = self.history.redo() {
            match action {
                EditAction::MoveNode {
                    index,
                    new_x,
                    new_y,
                    ..
                } => {
                    if let Some(node) = self.nodes.get_mut(index) {
                        node.x = new_x;
                        node.y = new_y;
                    }
                    self.status = String::from("Redo: move");
                }
                EditAction::ResizeNode {
                    index,
                    new_x,
                    new_y,
                    new_w,
                    new_h,
                    ..
                } => {
                    if let Some(node) = self.nodes.get_mut(index) {
                        node.x = new_x;
                        node.y = new_y;
                        node.width = new_w;
                        node.height = new_h;
                    }
                    self.status = String::from("Redo: resize");
                }
                EditAction::DeleteNode {
                    index,
                    removed_edges,
                    node,
                    ..
                } => {
                    let alias = node.alias.clone();
                    if index <= self.nodes.len() {
                        self.nodes.remove(index);
                    }
                    for edge in &removed_edges {
                        self.edges.retain(|e| {
                            e.from_node != edge.from_node
                                || e.from_port != edge.from_port
                                || e.to_node != edge.to_node
                                || e.to_port != edge.to_port
                        });
                    }
                    let _ = alias; // used for edge cleanup above
                    self.status = String::from("Redo: delete node");
                }
                EditAction::CreateConnection { edge } => {
                    self.edges.push(edge);
                    self.status = String::from("Redo: create connection");
                }
                EditAction::DeleteConnection { index, .. } => {
                    if index < self.edges.len() {
                        self.edges.remove(index);
                    }
                    self.status = String::from("Redo: delete connection");
                }
            }
            self.canvas_state.request_redraw();
        }
    }
}

/// Resolve port information for subprocesses by parsing the flow with `flowrclib`.
///
/// Returns a map from subprocess alias to (inputs, outputs) port info.
/// If parsing fails, returns an empty map so the caller can fall back to guessing.
fn resolve_subprocess_ports(url: &Url) -> HashMap<String, (Vec<PortInfo>, Vec<PortInfo>)> {
    let mut resolved = HashMap::new();

    // Set up the library search path from FLOW_LIB_PATH, with ~/.flow/lib as default
    let mut lib_search_path = Simpath::new_with_separator("FLOW_LIB_PATH", ',');
    if let Ok(home) = std::env::var("HOME") {
        let default_lib = PathBuf::from(home).join(".flow").join("lib");
        if default_lib.exists() {
            if let Some(path_str) = default_lib.to_str() {
                lib_search_path.add_directory(path_str);
                info!("Added default library path: {path_str}");
            }
        }
    }

    let provider = MetaProvider::new(lib_search_path, PathBuf::from("/"));

    match flowrclib::compiler::parser::parse(url, &provider) {
        Ok(Process::FlowProcess(flow)) => {
            info!(
                "Parsed flow '{}' with {} subprocesses",
                flow.name,
                flow.subprocesses.len()
            );
            for (alias, subprocess) in &flow.subprocesses {
                match subprocess {
                    Process::FunctionProcess(func) => {
                        let inputs: Vec<PortInfo> = func
                            .inputs
                            .iter()
                            .map(|io| PortInfo {
                                name: io.name().to_string(),
                                datatypes: io.datatypes().iter().map(|dt| dt.to_string()).collect(),
                            })
                            .collect();
                        let outputs: Vec<PortInfo> = func
                            .outputs
                            .iter()
                            .map(|io| PortInfo {
                                name: io.name().to_string(),
                                datatypes: io.datatypes().iter().map(|dt| dt.to_string()).collect(),
                            })
                            .collect();
                        info!(
                            "Resolved function '{}': {} inputs, {} outputs",
                            alias,
                            inputs.len(),
                            outputs.len()
                        );
                        resolved.insert(alias.clone(), (inputs, outputs));
                    }
                    Process::FlowProcess(sub_flow) => {
                        let inputs: Vec<PortInfo> = sub_flow
                            .inputs
                            .iter()
                            .map(|io| PortInfo {
                                name: io.name().to_string(),
                                datatypes: io.datatypes().iter().map(|dt| dt.to_string()).collect(),
                            })
                            .collect();
                        let outputs: Vec<PortInfo> = sub_flow
                            .outputs
                            .iter()
                            .map(|io| PortInfo {
                                name: io.name().to_string(),
                                datatypes: io.datatypes().iter().map(|dt| dt.to_string()).collect(),
                            })
                            .collect();
                        info!(
                            "Resolved sub-flow '{}': {} inputs, {} outputs",
                            alias,
                            inputs.len(),
                            outputs.len()
                        );
                        resolved.insert(alias.clone(), (inputs, outputs));
                    }
                }
            }
        }
        Ok(Process::FunctionProcess(_)) => {
            warn!("Parser returned a FunctionProcess instead of FlowProcess");
        }
        Err(e) => {
            warn!("Could not fully parse flow for port resolution, falling back to guessing: {e}");
        }
    }

    resolved
}

/// Load a flow definition file and return the flow name, node layouts, and edge layouts.
fn load_flow(path: &PathBuf) -> Result<(String, Vec<NodeLayout>, Vec<EdgeLayout>), String> {
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
            // Attempt to resolve real port definitions via the full parser
            let resolved_ports = resolve_subprocess_ports(&url);
            info!(
                "Resolved ports for {} of {} subprocesses",
                resolved_ports.len(),
                flow.process_refs.len()
            );

            let edges = build_edge_layouts(&flow.connections);
            let nodes =
                build_node_layouts(&flow.process_refs, &flow.connections, &resolved_ports);
            Ok((flow.name, nodes, edges))
        }
        Process::FunctionProcess(_) => Err(
            "The specified file defines a Function, not a Flow. flowedit requires a flow definition."
                .to_string(),
        ),
    }
}

/// Format a connection endpoint for display, omitting "default" or empty port names.
fn format_endpoint(node: &str, port: &str) -> String {
    if port.is_empty() || port == "default" || port == "output" {
        node.to_string()
    } else {
        format!("{node}/{port}")
    }
}
