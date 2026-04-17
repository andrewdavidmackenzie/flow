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
use iced::{Color, Element, Fill, Subscription, Task, Theme};
use log::{info, warn};
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
    /// A message from the interactive canvas (select, move, delete)
    Canvas(CanvasMessage),
    /// A message from the library side panel
    Library(LibraryMessage),
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
    /// Save the flow to the current file (or prompt if none)
    Save,
    /// Save the flow to a new file (always prompts)
    SaveAs,
    /// Open a flow file
    Open,
    /// Create a new empty flow
    New,
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
    /// Path to the currently loaded flow file, if any
    file_path: Option<PathBuf>,
    /// The original flow definition, used to preserve metadata when saving
    flow_definition: FlowDefinition,
    /// Tooltip text and screen position to display (full source path on hover)
    tooltip: Option<(String, f32, f32)>,
    /// Library panel tree for process discovery
    library_tree: LibraryTree,
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
            file_path,
            flow_definition,
            tooltip: None,
            library_tree,
        };

        (app, Task::none())
    }

    /// Return the window title, showing the file name and unsaved indicator.
    fn title(&self) -> String {
        let modified = if self.unsaved_edits > 0 { " *" } else { "" };
        let name = self
            .file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or(&self.flow_name);
        format!("flowedit - {name}{modified}")
    }

    /// Handle messages from canvas interactions.
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Canvas(canvas_msg) => match canvas_msg {
                CanvasMessage::Selected(idx) => {
                    self.selected_node = idx;
                    if self.selected_connection.is_some() {
                        self.selected_connection = None;
                        self.canvas_state.request_redraw();
                    }
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
                    self.canvas_state.request_redraw();
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
                CanvasMessage::HoverChanged(data) => {
                    self.tooltip = data;
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
            Message::Library(ref lib_msg) => {
                if let Some((source, func_name)) = self.library_tree.update(lib_msg) {
                    self.add_library_function(&source, &func_name);
                }
            }
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
            Message::Save => {
                if let Some(ref path) = self.file_path.clone() {
                    self.perform_save(path);
                } else {
                    self.perform_save_as();
                }
            }
            Message::SaveAs => {
                self.perform_save_as();
            }
            Message::Open => {
                self.perform_open();
            }
            Message::New => {
                self.perform_new();
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

        let canvas_with_controls = if let Some((ref tip_text, tx, ty)) = self.tooltip {
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
            stack![canvas, zoom_controls, tooltip_widget]
        } else {
            stack![canvas, zoom_controls]
        };

        let library_panel = self.library_tree.view().map(Message::Library);

        let main_content = Row::new()
            .push(library_panel)
            .push(container(canvas_with_controls).width(Fill).height(Fill));

        let edit_indicator = if self.unsaved_edits > 0 {
            format!("  |  {} unsaved edit(s)", self.unsaved_edits)
        } else {
            String::from("  |  saved")
        };
        let status_bar: Row<'_, Message> =
            Row::new().push(Text::new(format!("{}{}", self.status, edit_indicator)).size(14));

        Column::new()
            .push(container(main_content).width(Fill).height(Fill))
            .push(container(status_bar).width(Fill).padding(5))
            .into()
    }

    /// Listen for keyboard shortcuts: Cmd+Z undo, Cmd+Shift+Z redo,
    /// Cmd+S save, Cmd+Shift+S save-as, Cmd+O open, Cmd+N new.
    fn subscription(&self) -> Subscription<Message> {
        keyboard::listen().filter_map(|event| match event {
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

    /// Save the current flow to the given path.
    fn perform_save(&mut self, path: &PathBuf) {
        self.sync_flow_definition();
        match save_flow_toml(&self.flow_definition, &self.edges, path) {
            Ok(()) => {
                self.unsaved_edits = 0;
                self.file_path = Some(path.clone());
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    self.status = format!("Saved to {name}");
                } else {
                    self.status = String::from("Saved");
                }
            }
            Err(e) => {
                self.status = format!("Save failed: {e}");
            }
        }
    }

    /// Prompt the user with a save dialog and save to the chosen path.
    fn perform_save_as(&mut self) {
        let dialog = rfd::FileDialog::new()
            .add_filter("Flow", &["toml"])
            .set_file_name(format!("{}.toml", self.flow_name));
        if let Some(path) = dialog.save_file() {
            self.perform_save(&path);
        }
    }

    /// Prompt the user with an open dialog and load the selected flow file.
    fn perform_open(&mut self) {
        let dialog = rfd::FileDialog::new().add_filter("Flow", &["toml"]);
        if let Some(path) = dialog.pick_file() {
            match load_flow(&path) {
                Ok((name, node_list, edge_list, flow_def)) => {
                    let nc = node_list.len();
                    let ec = edge_list.len();
                    self.flow_name = name;
                    self.nodes = node_list;
                    self.edges = edge_list;
                    self.flow_definition = flow_def;
                    self.file_path = Some(path);
                    self.selected_node = None;
                    self.selected_connection = None;
                    self.history = EditHistory::default();
                    self.unsaved_edits = 0;
                    self.auto_fit_pending = true;
                    self.auto_fit_enabled = true;
                    self.canvas_state = FlowCanvasState::default();
                    self.status = format!("Loaded - {nc} nodes, {ec} connections");
                }
                Err(e) => {
                    self.status = format!("Open failed: {e}");
                }
            }
        }
    }

    /// Clear the canvas and reset to an empty flow state.
    fn perform_new(&mut self) {
        self.flow_name = String::from("(new flow)");
        self.nodes = Vec::new();
        self.edges = Vec::new();
        self.flow_definition = FlowDefinition::default();
        self.file_path = None;
        self.selected_node = None;
        self.selected_connection = None;
        self.history = EditHistory::default();
        self.unsaved_edits = 0;
        self.auto_fit_pending = false;
        self.auto_fit_enabled = true;
        self.canvas_state = FlowCanvasState::default();
        self.status = String::from("New flow");
    }

    /// Add a function from the library panel as a new node on the canvas.
    ///
    /// Creates a `NodeLayout` at a default position and a `ProcessReference`
    /// in the flow definition, and records the action in the edit history.
    fn add_library_function(&mut self, source: &str, func_name: &str) {
        // Generate a unique alias: if the name already exists, append a number
        let alias = generate_unique_alias(func_name, &self.nodes);

        // Place the new node at a default position offset from existing nodes
        let (x, y) = next_node_position(&self.nodes);

        let node = NodeLayout {
            alias: alias.clone(),
            source: source.to_string(),
            x,
            y,
            width: 180.0,
            height: 120.0,
            inputs: Vec::new(),
            outputs: Vec::new(),
            initializers: HashMap::new(),
        };

        let index = self.nodes.len();
        self.nodes.push(node.clone());

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
        self.flow_definition.process_refs.push(pref);

        self.record_edit(EditAction::DeleteNode {
            index,
            node,
            removed_edges: Vec::new(),
        });
        // Note: We record a DeleteNode so that *undo* removes the added node.
        // This is intentional: undoing an "add" means deleting what was added.

        self.selected_node = Some(index);
        self.canvas_state.request_redraw();
        let nc = self.nodes.len();
        self.status = format!("Added {alias} from library - {nc} nodes");
    }

    /// Synchronize the in-memory `FlowDefinition` with the current editor state
    /// so that process references and the flow name are up to date.
    /// Connections are handled separately via `EdgeLayout` during save.
    fn sync_flow_definition(&mut self) {
        // Update or rebuild process_refs from current NodeLayout data
        let mut new_refs: Vec<ProcessReference> = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            // Try to find the original ProcessReference by alias to preserve initializations
            let original = self
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
        self.flow_definition.process_refs = new_refs;

        // Update the flow name
        self.flow_definition.name = self.flow_name.clone();
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
