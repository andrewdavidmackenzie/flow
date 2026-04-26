//! Per-window state and related types for the flow editor.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use iced::widget::canvas::{self, Canvas};
use iced::widget::{button, container, pick_list, stack, text_input, Column, Row, Text};
use iced::{window, Color, Element, Fill, Point, Size, Theme};
use log::info;
use url::Url;

use flowcore::model::connection::Connection;
use flowcore::model::datatype::DataType;
use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::function_definition::FunctionDefinition;
use flowcore::model::input::InputInitializer;
use flowcore::model::io::{IOType, IO};
use flowcore::model::name::HasName;
use flowcore::model::process::Process;
use flowcore::model::process_reference::ProcessReference;
use flowcore::model::route::Route;

use crate::file_ops;
use crate::flow_canvas::{
    flow_io_bounding_box, transform_point, CanvasAction, CanvasMessage, FlowCanvas,
};
use crate::hierarchy_panel::FlowHierarchy;
use crate::history::{EditAction, EditHistory};
use crate::node_layout::NodeLayout;
use crate::node_layout::PORT_FONT_SIZE;
use crate::utils::{
    connection_references_node, derive_short_name, next_unique_io_name, split_route,
};
use crate::{FlowEditMessage, Message, ViewMessage};

/// Tooltip text and screen position for hover display.
#[derive(Debug, Clone)]
pub(crate) struct Tooltip {
    pub(crate) text: String,
    pub(crate) x: f32,
    pub(crate) y: f32,
}

/// Screen position for a right-click context menu.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MenuPosition {
    pub(crate) x: f32,
    pub(crate) y: f32,
}

/// State for the initializer editing dialog.
pub(crate) struct InitializerEditor {
    /// Index of the node being edited
    pub(crate) node_index: usize,
    /// Name of the input port being edited
    pub(crate) port_name: String,
    /// Selected type: "none", "once", or "always"
    pub(crate) init_type: String,
    /// The value as a string (JSON)
    pub(crate) value_text: String,
}

/// State for a function definition viewer/editor window.
pub(crate) struct FunctionViewer {
    /// The canonical function definition (owns name, description, source, inputs, outputs, `source_url`)
    pub(crate) func_def: FunctionDefinition,
    pub(crate) rs_content: String,
    pub(crate) docs_content: Option<String>,
    pub(crate) active_tab: usize,
    /// Parent window that opened this viewer (for propagating edits back to canvas)
    pub(crate) parent_window: Option<window::Id>,
    /// Source string of the node this viewer is editing (to find the `NodeLayout`)
    pub(crate) node_source: String,
    /// Whether this viewer is read-only (library/context functions cannot be edited)
    pub(crate) read_only: bool,
}

impl FunctionViewer {
    /// Derive the TOML file path from the function definition's source URL.
    pub(crate) fn toml_path(&self) -> Option<PathBuf> {
        self.func_def.get_source_url().to_file_path().ok()
    }
}

/// Minimum allowed zoom level
const MIN_ZOOM: f32 = 0.1;
/// Maximum allowed zoom level
pub(crate) const MAX_ZOOM: f32 = 5.0;
/// Zoom factor applied per step (zoom-in multiplies, zoom-out divides)
pub(crate) const ZOOM_STEP: f32 = 1.1;
/// Padding in world units around canvas content for auto-fit
const CANVAS_PADDING: f32 = 20.0;

/// Persistent canvas state that caches the rendered geometry.
pub(crate) struct FlowCanvasState {
    /// The geometry cache — cleared when the flow data changes
    pub(crate) cache: canvas::Cache,
    /// Current zoom level (1.0 = 100%)
    pub(crate) zoom: f32,
    /// Scroll offset in world coordinates
    pub(crate) scroll_offset: Point,
}

impl Default for FlowCanvasState {
    fn default() -> Self {
        Self {
            cache: canvas::Cache::new(),
            zoom: 1.0,
            scroll_offset: Point::new(0.0, 0.0),
        }
    }
}

impl FlowCanvasState {
    /// Create the canvas [`Element`] for displaying the given flow definition.
    ///
    /// Builds render nodes from the flow definition's process references and
    /// subprocess definitions. The `FlowCanvas` owns these render nodes for the
    /// duration of the frame.
    pub(crate) fn view<'a>(
        &'a self,
        flow_def: &'a FlowDefinition,
        is_subflow: bool,
        auto_fit_pending: bool,
        auto_fit_enabled: bool,
    ) -> Element<'a, CanvasMessage> {
        let nodes = NodeLayout::build_from_flow(flow_def);
        Canvas::new(FlowCanvas {
            state: self,
            flow_def,
            nodes,
            is_subflow,
            auto_fit_pending,
            auto_fit_enabled,
        })
        .width(Fill)
        .height(Fill)
        .into()
    }

    /// Invalidate the cached geometry so the canvas redraws on the next frame.
    pub(crate) fn request_redraw(&mut self) {
        self.cache.clear();
    }

    /// Zoom in by one step (multiply zoom by [`ZOOM_STEP`]).
    pub(crate) fn zoom_in(&mut self) {
        self.zoom = (self.zoom * ZOOM_STEP).min(MAX_ZOOM);
        self.cache.clear();
    }

    /// Zoom out by one step (divide zoom by [`ZOOM_STEP`]).
    pub(crate) fn zoom_out(&mut self) {
        self.zoom = (self.zoom / ZOOM_STEP).max(MIN_ZOOM);
        self.cache.clear();
    }

    /// Compute zoom and offset so that all nodes fit within the given viewport with padding.
    ///
    /// If `nodes` is empty, resets to default zoom and offset.
    fn content_extents(
        nodes: &[NodeLayout],
        flow_inputs: &[IO],
        flow_outputs: &[IO],
        has_flow_io: bool,
    ) -> (f32, f32, f32, f32) {
        if has_flow_io {
            let (box_x, box_y, box_w, box_h, _, _) =
                flow_io_bounding_box(nodes, flow_inputs, flow_outputs);
            let max_input_label = flow_inputs
                .iter()
                .map(|io| io.name().len())
                .max()
                .unwrap_or(0);
            let max_output_label = flow_outputs
                .iter()
                .map(|io| io.name().len())
                .max()
                .unwrap_or(0);
            let label_margin = max_input_label.max(max_output_label) as f32 * PORT_FONT_SIZE + 20.0;
            (
                box_x - label_margin,
                box_y,
                box_x + box_w + label_margin,
                box_y + box_h,
            )
        } else {
            let mut min_x = f32::MAX;
            let mut min_y = f32::MAX;
            let mut max_x = f32::MIN;
            let mut max_y = f32::MIN;
            for node in nodes {
                let init_margin = if node.has_initializers() {
                    node.max_initializer_display_len() as f32 * 8.0
                } else {
                    0.0
                };
                min_x = min_x.min(node.x() - init_margin);
                min_y = min_y.min(node.y());
                max_x = max_x.max(node.x() + node.width());
                max_y = max_y.max(node.y() + node.height());
            }
            (min_x, min_y, max_x, max_y)
        }
    }

    pub(crate) fn auto_fit(
        &mut self,
        nodes: &[NodeLayout],
        flow_inputs: &[IO],
        flow_outputs: &[IO],
        is_subflow: bool,
        viewport: Size,
    ) {
        let has_flow_io = is_subflow && (!flow_inputs.is_empty() || !flow_outputs.is_empty());
        if nodes.is_empty() && !has_flow_io {
            self.zoom = 1.0;
            self.scroll_offset = Point::new(0.0, 0.0);
            self.cache.clear();
            return;
        }

        let (min_x, min_y, max_x, max_y) =
            Self::content_extents(nodes, flow_inputs, flow_outputs, has_flow_io);

        let content_width = max_x - min_x + CANVAS_PADDING * 2.0;
        let content_height = max_y - min_y + CANVAS_PADDING * 2.0;

        // Avoid division by zero
        if content_width <= 0.0 || content_height <= 0.0 {
            self.zoom = 1.0;
            self.scroll_offset = Point::new(0.0, 0.0);
            self.cache.clear();
            return;
        }

        let zoom_x = viewport.width / content_width;
        let zoom_y = viewport.height / content_height;
        self.zoom = zoom_x.min(zoom_y).clamp(MIN_ZOOM, MAX_ZOOM);

        // Set offset so that the content is centered
        // screen_x = (world_x + offset_x) * zoom
        // We want the center of the content to map to the center of the viewport
        let content_center_x = min_x.midpoint(max_x);
        let content_center_y = min_y.midpoint(max_y);
        let viewport_center_x = viewport.width / 2.0 / self.zoom;
        let viewport_center_y = viewport.height / 2.0 / self.zoom;

        self.scroll_offset = Point::new(
            viewport_center_x - content_center_x,
            viewport_center_y - content_center_y,
        );
        self.cache.clear();
    }
}

/// What kind of content a window displays.
pub(crate) enum WindowKind {
    FlowEditor,
    FunctionViewer(Box<FunctionViewer>),
}

/// Per-window state for the flow editor.
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct WindowState {
    /// Route in the flow hierarchy that this window is editing
    #[allow(dead_code)]
    pub(crate) route: Route,
    /// What this window displays
    pub(crate) kind: WindowKind,
    /// Canvas state for caching rendered geometry
    pub(crate) canvas_state: FlowCanvasState,
    /// Status message displayed in the bottom bar
    pub(crate) status: String,
    /// Index of the currently selected node, if any
    pub(crate) selected_node: Option<usize>,
    /// Index of the currently selected connection, if any
    pub(crate) selected_connection: Option<usize>,
    /// Edit history for undo/redo
    pub(crate) history: EditHistory,
    /// Whether auto-fit should be performed on the next opportunity
    pub(crate) auto_fit_pending: bool,
    /// Whether auto-fit mode is active (continuously fits to window)
    pub(crate) auto_fit_enabled: bool,
    /// Tooltip text and screen position to display (full source path on hover)
    pub(crate) tooltip: Option<Tooltip>,
    /// Active initializer editor dialog, if any
    pub(crate) initializer_editor: Option<InitializerEditor>,
    /// Whether this is the root (main) window
    pub(crate) is_root: bool,
    /// Context menu position (screen coords), if showing
    pub(crate) context_menu: Option<MenuPosition>,
    /// Whether the metadata editor is visible
    pub(crate) show_metadata: bool,
    /// Flow hierarchy tree for this window's navigation panel
    pub(crate) flow_hierarchy: FlowHierarchy,
    /// Last known window size (tracked via resize events)
    pub(crate) last_size: Option<iced::Size>,
    /// Last known window position (tracked via move events)
    pub(crate) last_position: Option<iced::Point>,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            route: Route::default(),
            kind: WindowKind::FlowEditor,
            canvas_state: FlowCanvasState::default(),
            status: String::new(),
            selected_node: None,
            selected_connection: None,
            history: EditHistory::default(),
            auto_fit_pending: false,
            auto_fit_enabled: false,
            tooltip: None,
            initializer_editor: None,
            is_root: false,
            context_menu: None,
            show_metadata: false,
            flow_hierarchy: FlowHierarchy::empty(),
            last_size: None,
            last_position: None,
        }
    }
}

impl WindowState {
    /// Get the file path from a given flow definition's source URL.
    pub(crate) fn file_path_of(flow_def: &FlowDefinition) -> Option<PathBuf> {
        flow_def.source_url.to_file_path().ok()
    }

    /// Set the file path on a given flow definition.
    pub(crate) fn set_file_path_on(flow_def: &mut FlowDefinition, path: &Path) {
        let abs = path.canonicalize().unwrap_or_else(|_| {
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                std::env::current_dir().map_or_else(|_| path.to_path_buf(), |cwd| cwd.join(path))
            }
        });
        if let Ok(url) = Url::from_file_path(&abs) {
            flow_def.source_url = url;
        }
    }

    /// Clear the file path on a given flow definition.
    pub(crate) fn clear_file_path_on(flow_def: &mut FlowDefinition) {
        flow_def.source_url = FlowDefinition::default_url();
    }

    /// Handle a [`CanvasMessage`] by mutating canvas/selection state.
    ///
    /// Returns a [`CanvasAction`] when the caller needs to perform cross-window
    /// operations (e.g. opening a sub-flow in a new editor window).
    #[allow(clippy::too_many_lines)]
    pub(crate) fn handle_canvas_message(
        &mut self,
        flow_def: &mut FlowDefinition,
        msg: CanvasMessage,
    ) -> CanvasAction {
        match msg {
            CanvasMessage::Selected(idx) => self.handle_selected(flow_def, idx),
            CanvasMessage::Moved(idx, x, y) => {
                if let Some(pref) = flow_def.process_refs.get_mut(idx) {
                    pref.x = Some(x);
                    pref.y = Some(y);
                    self.canvas_state.request_redraw();
                }
            }
            CanvasMessage::Resized(idx, x, y, w, h) => {
                if let Some(pref) = flow_def.process_refs.get_mut(idx) {
                    pref.x = Some(x);
                    pref.y = Some(y);
                    pref.width = Some(w);
                    pref.height = Some(h);
                    self.canvas_state.request_redraw();
                }
            }
            CanvasMessage::MoveCompleted(idx, old_x, old_y, new_x, new_y) => {
                info!("MoveCompleted: idx={idx}, ({old_x},{old_y}) -> ({new_x},{new_y})");
                if (old_x - new_x).abs() > 0.5 || (old_y - new_y).abs() > 0.5 {
                    self.history.record(EditAction::MoveNode {
                        index: idx,
                        old_x,
                        old_y,
                        new_x,
                        new_y,
                    });
                    self.trigger_auto_fit_if_enabled();
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
                self.handle_resize_completed(
                    idx, old_x, old_y, old_w, old_h, new_x, new_y, new_w, new_h,
                );
            }
            CanvasMessage::Deleted(idx) => self.handle_deleted(flow_def, idx),
            CanvasMessage::ConnectionCreated {
                from_node,
                from_port,
                to_node,
                to_port,
            } => {
                self.handle_connection_created(
                    flow_def, &from_node, &from_port, &to_node, &to_port,
                );
            }
            CanvasMessage::ConnectionSelected(idx) => {
                self.handle_connection_selected(flow_def, idx);
            }
            CanvasMessage::ConnectionDeleted(idx) => {
                self.handle_connection_deleted(flow_def, idx);
            }
            CanvasMessage::HoverChanged(data) => {
                self.tooltip = data;
            }
            CanvasMessage::AutoFitViewport(viewport) => {
                if self.auto_fit_enabled || self.auto_fit_pending {
                    let render_nodes = NodeLayout::build_from_flow(flow_def);
                    let is_subflow = !self.is_root;
                    self.canvas_state.auto_fit(
                        &render_nodes,
                        &flow_def.inputs,
                        &flow_def.outputs,
                        is_subflow,
                        viewport,
                    );
                    self.auto_fit_pending = false;
                }
            }
            CanvasMessage::Pan(dx, dy) => {
                self.auto_fit_enabled = false;
                self.auto_fit_pending = false;
                self.canvas_state.scroll_offset.x += dx;
                self.canvas_state.scroll_offset.y += dy;
                self.canvas_state.request_redraw();
            }
            CanvasMessage::ZoomBy(factor) => {
                self.auto_fit_enabled = false;
                self.auto_fit_pending = false;
                self.canvas_state.zoom = (self.canvas_state.zoom * factor).clamp(0.1, 5.0);
                self.canvas_state.request_redraw();
                let pct = (self.canvas_state.zoom * 100.0) as u32;
                self.status = format!("Zoom: {pct}%");
            }
            CanvasMessage::InitializerEdit(node_idx, port_name) => {
                self.handle_initializer_edit(flow_def, node_idx, port_name);
            }
            CanvasMessage::OpenNode(idx) => {
                return CanvasAction::OpenNode(idx);
            }
            CanvasMessage::ContextMenu(x, y) => {
                self.context_menu = Some(crate::window_state::MenuPosition { x, y });
            }
        }
        CanvasAction::None
    }

    pub(crate) fn trigger_auto_fit_if_enabled(&mut self) {
        if self.auto_fit_enabled {
            self.auto_fit_pending = true;
            self.canvas_state.request_redraw();
        }
    }

    fn handle_selected(&mut self, flow_def: &FlowDefinition, idx: Option<usize>) {
        self.selected_node = idx;
        self.context_menu = None;
        if self.selected_connection.is_some() {
            self.selected_connection = None;
            self.canvas_state.request_redraw();
        }
        if let Some(i) = idx {
            if let Some(pref) = flow_def.process_refs.get(i) {
                let alias = if pref.alias.is_empty() {
                    derive_short_name(&pref.source)
                } else {
                    pref.alias.clone()
                };
                self.status = format!("Selected: {alias}");
            }
        } else {
            self.status = String::from("Ready");
        }
    }

    #[allow(clippy::similar_names, clippy::too_many_arguments)]
    fn handle_resize_completed(
        &mut self,
        idx: usize,
        old_x: f32,
        old_y: f32,
        old_w: f32,
        old_h: f32,
        new_x: f32,
        new_y: f32,
        new_w: f32,
        new_h: f32,
    ) {
        if (old_x - new_x).abs() > 0.5
            || (old_y - new_y).abs() > 0.5
            || (old_w - new_w).abs() > 0.5
            || (old_h - new_h).abs() > 0.5
        {
            self.history.record(EditAction::ResizeNode {
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
            self.trigger_auto_fit_if_enabled();
        }
    }

    fn handle_deleted(&mut self, flow_def: &mut FlowDefinition, idx: usize) {
        if idx < flow_def.process_refs.len() {
            let Some(pref) = flow_def.process_refs.get(idx).cloned() else {
                return;
            };
            let alias = if pref.alias.is_empty() {
                derive_short_name(&pref.source)
            } else {
                pref.alias.clone()
            };
            let removed_connections: Vec<Connection> = flow_def
                .connections
                .iter()
                .filter(|c| connection_references_node(c, &alias))
                .cloned()
                .collect();
            let removed_pref = flow_def.process_refs.remove(idx);
            let removed_subprocess = flow_def.subprocesses.remove(&alias);
            flow_def
                .connections
                .retain(|c| !connection_references_node(c, &alias));
            self.history.record(EditAction::DeleteNode {
                index: idx,
                process_ref: removed_pref,
                subprocess: removed_subprocess.map(|p| (alias, p)),
                removed_connections,
            });
            self.selected_node = None;
            self.selected_connection = None;
            self.canvas_state.request_redraw();
            let nc = flow_def.process_refs.len();
            let ec = flow_def.connections.len();
            self.status = format!("Node deleted - {nc} nodes, {ec} connections");
            self.trigger_auto_fit_if_enabled();
        }
    }

    fn handle_connection_created(
        &mut self,
        flow_def: &mut FlowDefinition,
        from_node: &str,
        from_port: &str,
        to_node: &str,
        to_port: &str,
    ) {
        let from_route = if from_port.is_empty() {
            from_node.to_string()
        } else {
            format!("{from_node}/{from_port}")
        };
        let to_route = if to_port.is_empty() {
            to_node.to_string()
        } else {
            format!("{to_node}/{to_port}")
        };
        let connection = Connection::new(from_route, to_route);
        self.history.record(EditAction::CreateConnection {
            connection: connection.clone(),
        });
        flow_def.connections.push(connection);
        self.canvas_state.request_redraw();
        let nc = flow_def.process_refs.len();
        let ec = flow_def.connections.len();
        self.status = format!(
            "Connection created: {from_node}/{from_port} -> {to_node}/{to_port} - {nc} nodes, {ec} connections"
        );
        self.trigger_auto_fit_if_enabled();
    }

    fn handle_connection_selected(&mut self, flow_def: &FlowDefinition, idx: Option<usize>) {
        self.context_menu = None;
        self.selected_connection = idx;
        self.selected_node = None;
        self.canvas_state.request_redraw();
        if let Some(i) = idx {
            if let Some(conn) = flow_def.connections.get(i) {
                let (from_node, from_port) = split_route(conn.from().as_ref());
                let to_str = conn
                    .to()
                    .first()
                    .map_or_else(String::new, ToString::to_string);
                let (to_node, to_port) = split_route(&to_str);
                self.status = format!(
                    "Connection: {} -> {}",
                    file_ops::format_endpoint(&from_node, &from_port),
                    file_ops::format_endpoint(&to_node, &to_port),
                );
            }
        } else {
            self.status = String::from("Ready");
        }
    }

    fn handle_connection_deleted(&mut self, flow_def: &mut FlowDefinition, idx: usize) {
        if idx < flow_def.connections.len() {
            let connection = flow_def.connections.remove(idx);
            self.history.record(EditAction::DeleteConnection {
                index: idx,
                connection,
            });
            self.selected_connection = None;
            self.canvas_state.request_redraw();
            let nc = flow_def.process_refs.len();
            let ec = flow_def.connections.len();
            self.status = format!("Connection deleted - {nc} nodes, {ec} connections");
            self.trigger_auto_fit_if_enabled();
        }
    }

    fn handle_initializer_edit(
        &mut self,
        flow_def: &FlowDefinition,
        node_idx: usize,
        port_name: String,
    ) {
        self.context_menu = None;
        let (init_type, value_text) = flow_def
            .process_refs
            .get(node_idx)
            .and_then(|pr| pr.initializations.get(&port_name))
            .map_or_else(
                || ("none".to_string(), String::new()),
                |init| match init {
                    InputInitializer::Once(v) => (
                        "once".to_string(),
                        serde_json::to_string(v).unwrap_or_default(),
                    ),
                    InputInitializer::Always(v) => (
                        "always".to_string(),
                        serde_json::to_string(v).unwrap_or_default(),
                    ),
                },
            );

        self.initializer_editor = Some(InitializerEditor {
            node_index: node_idx,
            port_name,
            init_type,
            value_text,
        });
    }

    pub(crate) fn handle_view_message(&mut self, msg: &ViewMessage) {
        match msg {
            ViewMessage::ZoomIn => {
                self.auto_fit_enabled = false;
                self.auto_fit_pending = false;
                self.canvas_state.zoom_in();
                let pct = (self.canvas_state.zoom * 100.0) as u32;
                self.status = format!("Zoom: {pct}%");
            }
            ViewMessage::ZoomOut => {
                self.auto_fit_enabled = false;
                self.auto_fit_pending = false;
                self.canvas_state.zoom_out();
                let pct = (self.canvas_state.zoom * 100.0) as u32;
                self.status = format!("Zoom: {pct}%");
            }
            ViewMessage::ToggleAutoFit => {
                self.auto_fit_enabled = !self.auto_fit_enabled;
                if self.auto_fit_enabled {
                    self.auto_fit_pending = true;
                    self.canvas_state.request_redraw();
                    self.status = String::from("Auto-fit enabled");
                } else {
                    self.status = String::from("Auto-fit disabled");
                }
            }
        }
    }

    pub(crate) fn view_canvas_area<'a>(
        &'a self,
        flow_def: &'a FlowDefinition,
        window_id: window::Id,
    ) -> Element<'a, Message> {
        let canvas = self
            .canvas_state
            .view(
                flow_def,
                !self.is_root,
                self.auto_fit_pending,
                self.auto_fit_enabled,
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
                        .on_press(Message::View(window_id, ViewMessage::ZoomIn))
                        .width(btn_width)
                        .style(zoom_btn),
                )
                .push(
                    button(Text::new("\u{2212}").center())
                        .on_press(Message::View(window_id, ViewMessage::ZoomOut))
                        .width(btn_width)
                        .style(zoom_btn),
                )
                .push(
                    button(Text::new("Fit").center())
                        .on_press(Message::View(window_id, ViewMessage::ToggleAutoFit))
                        .width(btn_width)
                        .style(if self.auto_fit_enabled {
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

        // Inline I/O editing overlays for sub-flow windows
        if !self.is_root {
            canvas_stack.push(Self::build_flow_io_overlays(
                flow_def,
                &self.canvas_state,
                window_id,
            ));
        }

        if let Some(ref tip) = self.tooltip {
            canvas_stack.push(Self::build_tooltip_overlay(tip));
        }

        if let Some(ref editor) = self.initializer_editor {
            canvas_stack.push(self.build_initializer_dialog(flow_def, window_id, editor));
        }

        if let Some(menu_pos) = self.context_menu {
            canvas_stack.push(Self::build_context_menu(window_id, menu_pos));
        }

        stack(canvas_stack).into()
    }

    fn build_flow_io_overlays<'a>(
        flow_def: &'a FlowDefinition,
        canvas_state: &FlowCanvasState,
        window_id: window::Id,
    ) -> Element<'a, Message> {
        let nodes = NodeLayout::build_from_flow(flow_def);
        let (box_x, _, box_w, _, center_y, spacing) =
            flow_io_bounding_box(&nodes, &flow_def.inputs, &flow_def.outputs);
        let zoom = canvas_state.zoom;
        let offset = canvas_state.scroll_offset;
        let route = flow_def.route.clone();
        let right_x = box_x + box_w;

        let mut layers: Vec<Element<'_, Message>> = Vec::new();

        Self::build_io_layers(
            &mut layers,
            &flow_def.inputs,
            box_x,
            center_y,
            spacing,
            zoom,
            offset,
            &route,
            window_id,
            true,
        );
        Self::build_io_layers(
            &mut layers,
            &flow_def.outputs,
            right_x,
            center_y,
            spacing,
            zoom,
            offset,
            &route,
            window_id,
            false,
        );

        stack(layers).into()
    }

    #[allow(clippy::too_many_arguments)]
    fn build_io_layers<'a>(
        layers: &mut Vec<Element<'a, Message>>,
        ports: &'a [IO],
        port_x: f32,
        center_y: f32,
        spacing: f32,
        zoom: f32,
        offset: Point,
        route: &Route,
        window_id: window::Id,
        is_input: bool,
    ) {
        let row_height = 24.0;
        let (row_x_offset, add_x_offset) = if is_input {
            (-100.0, -60.0)
        } else {
            (8.0, 8.0)
        };
        let start_y = center_y - (ports.len() as f32 - 1.0) * spacing / 2.0;

        for (i, port) in ports.iter().enumerate() {
            let world_y = start_y + i as f32 * spacing;
            let screen_pos = transform_point(Point::new(port_x, world_y), zoom, offset);
            let route_clone = route.clone();

            let name_input = text_input("name", port.name())
                .on_input(move |s| {
                    let msg = if is_input {
                        FlowEditMessage::InputNameChanged(i, s)
                    } else {
                        FlowEditMessage::OutputNameChanged(i, s)
                    };
                    Message::FlowEdit(window_id, route_clone.clone(), msg)
                })
                .size(PORT_FONT_SIZE)
                .padding(2)
                .width(80);

            let route_del = route.clone();
            let del_msg = if is_input {
                FlowEditMessage::DeleteInput(i)
            } else {
                FlowEditMessage::DeleteOutput(i)
            };
            let del_btn = button(Text::new("\u{2715}").size(9))
                .on_press(Message::FlowEdit(window_id, route_del, del_msg))
                .style(button::text)
                .padding([1, 3]);

            let row = if is_input {
                Row::new()
                    .spacing(2)
                    .align_y(iced::Alignment::Center)
                    .push(del_btn)
                    .push(name_input)
            } else {
                Row::new()
                    .spacing(2)
                    .align_y(iced::Alignment::Center)
                    .push(name_input)
                    .push(del_btn)
            };

            layers.push(
                container(row)
                    .width(Fill)
                    .height(Fill)
                    .padding(iced::Padding {
                        top: (screen_pos.y - row_height / 2.0).max(0.0),
                        left: (screen_pos.x + row_x_offset).max(0.0),
                        right: 0.0,
                        bottom: 0.0,
                    })
                    .into(),
            );
        }

        let add_y = start_y + ports.len() as f32 * spacing;
        let add_screen = transform_point(Point::new(port_x, add_y), zoom, offset);
        let route_add = route.clone();
        let (add_label, add_msg) = if is_input {
            ("+ Input", FlowEditMessage::AddInput)
        } else {
            ("+ Output", FlowEditMessage::AddOutput)
        };
        layers.push(
            container(
                button(Text::new(add_label).size(10))
                    .on_press(Message::FlowEdit(window_id, route_add, add_msg))
                    .style(button::text)
                    .padding([2, 4]),
            )
            .width(Fill)
            .height(Fill)
            .padding(iced::Padding {
                top: (add_screen.y - row_height / 2.0).max(0.0),
                left: (add_screen.x + add_x_offset).max(0.0),
                right: 0.0,
                bottom: 0.0,
            })
            .into(),
        );
    }

    fn build_tooltip_overlay<'a>(tip: &crate::window_state::Tooltip) -> Element<'a, Message> {
        container(
            container(Text::new(tip.text.clone()).size(20).color(Color::WHITE))
                .padding(8)
                .style(|_theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgb(0.12, 0.12, 0.12))),
                    border: iced::Border {
                        color: Color::WHITE,
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                }),
        )
        .padding(iced::Padding {
            top: tip.y + 26.0,
            right: 0.0,
            bottom: 0.0,
            left: (tip.x - 80.0).max(0.0),
        })
        .into()
    }

    #[allow(clippy::unused_self)]
    fn build_initializer_dialog<'a>(
        &self,
        flow_def: &FlowDefinition,
        window_id: window::Id,
        editor: &InitializerEditor,
    ) -> Element<'a, Message> {
        let port_label = if let Some(pref) = flow_def.process_refs.get(editor.node_index) {
            let alias = if pref.alias.is_empty() {
                derive_short_name(&pref.source)
            } else {
                pref.alias.clone()
            };
            format!("{}/{}", alias, editor.port_name)
        } else {
            editor.port_name.clone()
        };

        let init_types = vec!["none", "once", "always"];
        let selected: Option<&str> = init_types.iter().find(|&&t| t == editor.init_type).copied();

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

        container(
            container(dialog_col)
                .width(280)
                .style(|_theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgb(0.15, 0.15, 0.15))),
                    border: iced::Border {
                        color: Color::from_rgb(0.4, 0.4, 0.4),
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                }),
        )
        .center(Fill)
        .into()
    }

    fn build_context_menu(
        window_id: window::Id,
        menu_pos: crate::window_state::MenuPosition,
    ) -> Element<'static, Message> {
        let menu = container(
            Column::new()
                .spacing(2)
                .push(
                    button(Text::new("+ New Sub-flow").size(13))
                        .on_press(Message::NewSubFlow(window_id))
                        .style(button::text)
                        .padding([6, 16])
                        .width(Fill),
                )
                .push(
                    button(Text::new("+ New Function").size(13))
                        .on_press(Message::NewFunction(window_id))
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

        container(menu)
            .padding(iced::Padding {
                top: menu_pos.y,
                left: menu_pos.x,
                right: 0.0,
                bottom: 0.0,
            })
            .into()
    }
    // --- File operations (from file_ops.rs) ---

    fn flow_directory(flow_def: &FlowDefinition) -> Option<PathBuf> {
        Self::file_path_of(flow_def).and_then(|p| p.parent().map(Path::to_path_buf))
    }

    pub(crate) fn perform_save(&mut self, flow_def: &mut FlowDefinition, path: &PathBuf) -> bool {
        match file_ops::save_flow_toml(flow_def, path) {
            Ok(()) => {
                Self::set_file_path_on(flow_def, path);
                file_ops::save_editor_prefs(path, self.last_size, self.last_position);
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    self.status = format!("Saved to {name}");
                } else {
                    self.status = String::from("Saved");
                }
                true
            }
            Err(e) => {
                self.status = format!("Save failed: {e}");
                false
            }
        }
    }

    pub(crate) fn perform_save_as(&mut self, flow_def: &mut FlowDefinition) -> bool {
        let mut dialog = rfd::FileDialog::new()
            .add_filter("Flow", &["toml"])
            .set_file_name(format!("{}.toml", flow_def.name));
        if let Some(dir) = Self::flow_directory(flow_def) {
            dialog = dialog.set_directory(dir);
        }
        if let Some(path) = dialog.save_file() {
            return self.perform_save(flow_def, &path);
        }
        false
    }

    pub(crate) fn handle_save(&mut self, flow_def: &mut FlowDefinition) -> bool {
        if let Some(path) = Self::file_path_of(flow_def) {
            self.perform_save(flow_def, &path)
        } else {
            self.perform_save_as(flow_def)
        }
    }

    pub(crate) fn handle_save_as(&mut self, flow_def: &mut FlowDefinition) -> bool {
        self.perform_save_as(flow_def)
    }

    pub(crate) fn perform_open(
        &mut self,
        flow_def: &mut FlowDefinition,
    ) -> Option<(BTreeSet<Url>, BTreeSet<Url>)> {
        let mut dialog = rfd::FileDialog::new().add_filter("Flow", &["toml"]);
        if let Some(dir) = Self::flow_directory(flow_def) {
            dialog = dialog.set_directory(dir);
        }
        if let Some(path) = dialog.pick_file() {
            match file_ops::load_flow(&path) {
                Ok(loaded_flow) => {
                    let lib_refs = loaded_flow.lib_references.clone();
                    let ctx_refs = loaded_flow.context_references.clone();
                    let nc = loaded_flow.process_refs.len();
                    let ec = loaded_flow.connections.len();
                    *flow_def = loaded_flow;
                    Self::set_file_path_on(flow_def, &path);
                    self.selected_node = None;
                    self.selected_connection = None;
                    self.tooltip = None;
                    self.context_menu = None;
                    self.initializer_editor = None;
                    self.show_metadata = false;
                    self.history = EditHistory::default();
                    self.auto_fit_pending = true;
                    self.auto_fit_enabled = true;
                    self.canvas_state = FlowCanvasState::default();
                    self.status = format!("Loaded - {nc} nodes, {ec} connections");
                    return Some((lib_refs, ctx_refs));
                }
                Err(e) => {
                    self.status = format!("Open failed: {e}");
                }
            }
        }
        None
    }

    /// Clear the canvas and reset to an empty flow state.
    pub(crate) fn perform_new(&mut self, flow_def: &mut FlowDefinition) {
        *flow_def = FlowDefinition::default();
        flow_def.name = String::from("(new flow)");
        Self::clear_file_path_on(flow_def);
        self.selected_node = None;
        self.selected_connection = None;
        self.tooltip = None;
        self.context_menu = None;
        self.initializer_editor = None;
        self.show_metadata = false;
        self.history = EditHistory::default();
        self.auto_fit_pending = false;
        self.auto_fit_enabled = true;
        self.canvas_state = FlowCanvasState::default();
        self.status = String::from("New flow");
    }

    /// Compile the current flow to a manifest.
    pub(crate) fn perform_compile(
        &mut self,
        flow_def: &mut FlowDefinition,
    ) -> Result<PathBuf, String> {
        if Self::file_path_of(flow_def).is_none() {
            self.perform_save_as(flow_def);
        }
        let Some(flow_path) = Self::file_path_of(flow_def) else {
            return Err("Flow must be saved before compiling".to_string());
        };

        if !self.history.is_empty() {
            self.perform_save(flow_def, &flow_path);
            if !self.history.is_empty() {
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

        let provider = file_ops::build_meta_provider();

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
        let tables = flowrclib::compiler::compile::compile(
            &flow,
            &output_dir,
            false,
            false,
            &mut source_urls,
        )
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

    // --- Undo/redo (from history.rs) ---

    fn apply_undo(&mut self, flow_def: &mut FlowDefinition) {
        if let Some(action) = self.history.undo() {
            match action {
                EditAction::MoveNode {
                    index,
                    old_x,
                    old_y,
                    ..
                } => {
                    if let Some(pref) = flow_def.process_refs.get_mut(index) {
                        pref.x = Some(old_x);
                        pref.y = Some(old_y);
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
                    if let Some(pref) = flow_def.process_refs.get_mut(index) {
                        pref.x = Some(old_x);
                        pref.y = Some(old_y);
                        pref.width = Some(old_w);
                        pref.height = Some(old_h);
                    }
                    self.status = String::from("Undo: resize");
                }
                EditAction::CreateNode {
                    index,
                    ref process_ref,
                    ..
                } => {
                    let alias = if process_ref.alias.is_empty() {
                        derive_short_name(&process_ref.source)
                    } else {
                        process_ref.alias.clone()
                    };
                    if index < flow_def.process_refs.len() {
                        flow_def.process_refs.remove(index);
                        flow_def.subprocesses.remove(&alias);
                        flow_def
                            .connections
                            .retain(|c| !connection_references_node(c, &alias));
                    }
                    self.status = String::from("Undo: create node");
                }
                EditAction::DeleteNode {
                    index,
                    process_ref,
                    subprocess,
                    removed_connections,
                } => {
                    flow_def.process_refs.insert(index, process_ref);
                    if let Some((name, proc)) = subprocess {
                        flow_def.subprocesses.insert(name, proc);
                    }
                    flow_def.connections.extend(removed_connections);
                    self.status = String::from("Undo: delete node");
                }
                EditAction::CreateConnection { connection } => {
                    let from_str = connection.from().to_string();
                    let to_strs: Vec<String> =
                        connection.to().iter().map(ToString::to_string).collect();
                    flow_def.connections.retain(|c| {
                        c.from().to_string() != from_str
                            || c.to().iter().map(ToString::to_string).collect::<Vec<_>>() != to_strs
                    });
                    self.status = String::from("Undo: create connection");
                }
                EditAction::DeleteConnection { index, connection } => {
                    flow_def.connections.insert(index, connection);
                    self.status = String::from("Undo: delete connection");
                }
                EditAction::EditInitializer {
                    node_index,
                    ref port_name,
                    ref old_init,
                    ..
                } => {
                    self.apply_initializer_state(
                        flow_def,
                        node_index,
                        port_name,
                        old_init.as_ref(),
                    );
                    self.status = String::from("Undo: initializer");
                }
            }
            self.canvas_state.request_redraw();
            self.trigger_auto_fit_if_enabled();
        }
    }

    fn apply_redo(&mut self, flow_def: &mut FlowDefinition) {
        if let Some(action) = self.history.redo() {
            match action {
                EditAction::MoveNode {
                    index,
                    new_x,
                    new_y,
                    ..
                } => {
                    if let Some(pref) = flow_def.process_refs.get_mut(index) {
                        pref.x = Some(new_x);
                        pref.y = Some(new_y);
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
                    if let Some(pref) = flow_def.process_refs.get_mut(index) {
                        pref.x = Some(new_x);
                        pref.y = Some(new_y);
                        pref.width = Some(new_w);
                        pref.height = Some(new_h);
                    }
                    self.status = String::from("Redo: resize");
                }
                EditAction::CreateNode {
                    index,
                    process_ref,
                    subprocess,
                } => {
                    let idx = index.min(flow_def.process_refs.len());
                    flow_def.process_refs.insert(idx, process_ref);
                    if let Some((name, proc)) = subprocess {
                        flow_def.subprocesses.insert(name, proc);
                    }
                    self.status = String::from("Redo: create node");
                }
                EditAction::DeleteNode {
                    index,
                    subprocess,
                    removed_connections,
                    ..
                } => {
                    if index < flow_def.process_refs.len() {
                        let removed = flow_def.process_refs.remove(index);
                        let alias = if removed.alias.is_empty() {
                            derive_short_name(&removed.source)
                        } else {
                            removed.alias.clone()
                        };
                        flow_def.subprocesses.remove(&alias);
                    }
                    if let Some((ref name, _)) = subprocess {
                        flow_def.subprocesses.remove(name);
                    }
                    for conn in &removed_connections {
                        let from_str = conn.from().to_string();
                        let to_strs: Vec<String> =
                            conn.to().iter().map(ToString::to_string).collect();
                        flow_def.connections.retain(|c| {
                            c.from().to_string() != from_str
                                || c.to().iter().map(ToString::to_string).collect::<Vec<_>>()
                                    != to_strs
                        });
                    }
                    self.status = String::from("Redo: delete node");
                }
                EditAction::CreateConnection { connection } => {
                    flow_def.connections.push(connection);
                    self.status = String::from("Redo: create connection");
                }
                EditAction::DeleteConnection { index, .. } => {
                    if index < flow_def.connections.len() {
                        flow_def.connections.remove(index);
                    }
                    self.status = String::from("Redo: delete connection");
                }
                EditAction::EditInitializer {
                    node_index,
                    ref port_name,
                    ref new_init,
                    ..
                } => {
                    self.apply_initializer_state(
                        flow_def,
                        node_index,
                        port_name,
                        new_init.as_ref(),
                    );
                    self.status = String::from("Redo: initializer");
                }
            }
            self.canvas_state.request_redraw();
            self.trigger_auto_fit_if_enabled();
        }
    }

    pub(crate) fn handle_undo(&mut self, flow_def: &mut FlowDefinition) {
        self.apply_undo(flow_def);
    }

    pub(crate) fn handle_redo(&mut self, flow_def: &mut FlowDefinition) {
        self.apply_redo(flow_def);
    }

    // --- Initializer editing (from initializer.rs) ---

    pub(crate) fn apply_initializer_edit(
        &mut self,
        flow_def: &mut FlowDefinition,
        editor: &InitializerEditor,
    ) {
        let alias = flow_def
            .process_refs
            .get(editor.node_index)
            .map(|pr| {
                if pr.alias.is_empty() {
                    derive_short_name(&pr.source)
                } else {
                    pr.alias.clone()
                }
            })
            .unwrap_or_default();

        let old_init = flow_def
            .process_refs
            .get(editor.node_index)
            .and_then(|pr| pr.initializations.get(&editor.port_name).cloned());

        let new_init = match editor.init_type.as_str() {
            "none" => None,
            "once" | "always" => {
                let value = serde_json::from_str(&editor.value_text)
                    .unwrap_or_else(|_| serde_json::Value::String(editor.value_text.clone()));
                let init = if editor.init_type == "once" {
                    InputInitializer::Once(value)
                } else {
                    InputInitializer::Always(value)
                };
                Some(init)
            }
            _ => return,
        };

        let Some(pref) = flow_def.process_refs.get_mut(editor.node_index) else {
            return;
        };
        match &new_init {
            Some(init) => {
                pref.initializations
                    .insert(editor.port_name.clone(), init.clone());
            }
            None => {
                pref.initializations.remove(&editor.port_name);
            }
        }

        self.history.record(EditAction::EditInitializer {
            node_index: editor.node_index,
            port_name: editor.port_name.clone(),
            old_init,
            new_init,
        });
        self.canvas_state.request_redraw();
        self.status = format!("Initializer updated on {}/{}", alias, editor.port_name);
    }

    #[allow(clippy::unused_self)]
    pub(crate) fn apply_initializer_state(
        &mut self,
        flow_def: &mut FlowDefinition,
        node_index: usize,
        port_name: &str,
        init: Option<&InputInitializer>,
    ) {
        if let Some(pref) = flow_def.process_refs.get_mut(node_index) {
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
    }

    pub(crate) fn handle_initializer_type_changed(&mut self, new_type: String) {
        if let Some(ref mut editor) = self.initializer_editor {
            editor.init_type = new_type;
        }
    }

    pub(crate) fn handle_initializer_value_changed(&mut self, new_value: String) {
        if let Some(ref mut editor) = self.initializer_editor {
            editor.value_text = new_value;
        }
    }

    pub(crate) fn handle_initializer_apply(&mut self, flow_def: &mut FlowDefinition) {
        if let Some(editor) = self.initializer_editor.take() {
            self.apply_initializer_edit(flow_def, &editor);
        }
    }

    pub(crate) fn handle_initializer_cancel(&mut self) {
        self.initializer_editor = None;
    }

    // --- Library management (from library_mgmt.rs) ---

    /// Add a library function as a new node on the canvas.
    pub(crate) fn add_library_function(
        &mut self,
        flow_def: &mut FlowDefinition,
        source: &str,
        func_name: &str,
    ) {
        let alias = file_ops::generate_unique_alias(func_name, &flow_def.process_refs);
        let (x, y) = file_ops::next_node_position(&flow_def.process_refs);

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

        let index = flow_def.process_refs.len();
        flow_def.process_refs.push(pref.clone());

        if let Some(proc) = resolved_process {
            flow_def.subprocesses.insert(alias.clone(), proc);
        }

        self.history.record(EditAction::CreateNode {
            index,
            process_ref: pref,
            subprocess: flow_def
                .subprocesses
                .get(&alias)
                .map(|p| (alias.clone(), p.clone())),
        });

        self.selected_node = Some(index);
        self.canvas_state.request_redraw();
        if self.auto_fit_enabled {
            self.auto_fit_pending = true;
        }
        let nc = flow_def.process_refs.len();
        self.status = format!("Added {alias} from library - {nc} nodes");
    }

    // --- Flow editing (from main.rs) ---

    fn rename_flow_input(&mut self, flow_def: &mut FlowDefinition, idx: usize, name: &str) {
        let duplicate = flow_def
            .inputs
            .iter()
            .enumerate()
            .any(|(i, io)| i != idx && io.name() == name);
        if duplicate {
            self.status = format!("Input name \"{name}\" already in use");
            return;
        }
        if let Some(io) = flow_def.inputs.get_mut(idx) {
            let old_name = io.name().clone();
            io.set_name(name.into());
            let old_route = format!("input/{old_name}");
            let new_route = format!("input/{name}");
            for conn in &mut flow_def.connections {
                if conn.from().to_string() == old_route {
                    conn.set_from(Route::from(new_route.as_str()));
                }
            }
        }
        self.status = String::new();
        self.history.mark_modified();
        self.canvas_state.request_redraw();
    }

    fn rename_flow_output(&mut self, flow_def: &mut FlowDefinition, idx: usize, name: &str) {
        let duplicate = flow_def
            .outputs
            .iter()
            .enumerate()
            .any(|(i, io)| i != idx && io.name() == name);
        if duplicate {
            self.status = format!("Output name \"{name}\" already in use");
            return;
        }
        if let Some(io) = flow_def.outputs.get_mut(idx) {
            let old_name = io.name().clone();
            io.set_name(name.into());
            let old_route_str = format!("output/{old_name}");
            let new_route_str = format!("output/{name}");
            for conn in &mut flow_def.connections {
                let new_to: Vec<Route> = conn
                    .to()
                    .iter()
                    .map(|r| {
                        if r.to_string() == old_route_str {
                            Route::from(new_route_str.as_str())
                        } else {
                            r.clone()
                        }
                    })
                    .collect();
                conn.set_to(new_to);
            }
        }
        self.status = String::new();
        self.history.mark_modified();
        self.canvas_state.request_redraw();
    }

    /// Handle flow metadata and I/O editing messages.
    pub(crate) fn handle_flow_edit_message(
        &mut self,
        flow_def: &mut FlowDefinition,
        msg: FlowEditMessage,
    ) {
        match msg {
            FlowEditMessage::NameChanged(new_name) => {
                flow_def.name = new_name;
                self.history.mark_modified();
            }
            FlowEditMessage::VersionChanged(version) => {
                flow_def.metadata.version = version;
                self.history.mark_modified();
            }
            FlowEditMessage::DescriptionChanged(desc) => {
                flow_def.metadata.description = desc;
                self.history.mark_modified();
            }
            FlowEditMessage::AuthorsChanged(authors_str) => {
                flow_def.metadata.authors = authors_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                self.history.mark_modified();
            }
            FlowEditMessage::ToggleMetadata => {
                self.show_metadata = !self.show_metadata;
            }
            FlowEditMessage::AddInput => {
                let name = next_unique_io_name("input", &flow_def.inputs);
                let mut io = IO::new_named(vec![DataType::from("string")], Route::default(), name);
                io.set_io_type(IOType::FlowInput);
                flow_def.inputs.push(io);
                self.history.mark_modified();
                self.canvas_state.request_redraw();
                self.trigger_auto_fit_if_enabled();
            }
            FlowEditMessage::AddOutput => {
                let name = next_unique_io_name("output", &flow_def.outputs);
                let mut io = IO::new_named(vec![DataType::from("string")], Route::default(), name);
                io.set_io_type(IOType::FlowOutput);
                flow_def.outputs.push(io);
                self.history.mark_modified();
                self.canvas_state.request_redraw();
                self.trigger_auto_fit_if_enabled();
            }
            FlowEditMessage::DeleteInput(idx) => {
                if let Some(io) = flow_def.inputs.get(idx) {
                    let name = io.name().clone();
                    flow_def.inputs.remove(idx);
                    flow_def.connections.retain(|c| {
                        let (from_node, from_port) = split_route(c.from().as_ref());
                        !(from_node == "input" && from_port == name)
                    });
                    self.history.mark_modified();
                    self.canvas_state.request_redraw();
                    self.trigger_auto_fit_if_enabled();
                }
            }
            FlowEditMessage::DeleteOutput(idx) => {
                if let Some(io) = flow_def.outputs.get(idx) {
                    let name = io.name().clone();
                    flow_def.outputs.remove(idx);
                    for conn in &mut flow_def.connections {
                        let new_to: Vec<Route> = conn
                            .to()
                            .iter()
                            .filter(|to_route| {
                                let (to_node, to_port) = split_route(to_route.as_ref());
                                !(to_node == "output" && to_port == name)
                            })
                            .cloned()
                            .collect();
                        conn.set_to(new_to);
                    }
                    flow_def.connections.retain(|c| !c.to().is_empty());
                    self.history.mark_modified();
                    self.canvas_state.request_redraw();
                    self.trigger_auto_fit_if_enabled();
                }
            }
            FlowEditMessage::InputNameChanged(idx, name) => {
                self.rename_flow_input(flow_def, idx, &name);
            }
            FlowEditMessage::OutputNameChanged(idx, name) => {
                self.rename_flow_output(flow_def, idx, &name);
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod test {
    use super::*;
    use crate::node_layout::NodeLayout;
    use flowcore::model::datatype::DataType;
    use flowcore::model::process_reference::ProcessReference;
    use flowcore::model::route::Route;

    fn test_node_data(
        alias: &str,
        source: &str,
        process: Option<flowcore::model::process::Process>,
    ) -> (ProcessReference, Option<flowcore::model::process::Process>) {
        (
            ProcessReference {
                alias: alias.into(),
                source: source.into(),
                initializations: std::collections::BTreeMap::new(),
                x: Some(100.0),
                y: Some(100.0),
                width: Some(180.0),
                height: Some(120.0),
            },
            process,
        )
    }

    fn as_layout(
        data: &(ProcessReference, Option<flowcore::model::process::Process>),
    ) -> NodeLayout<'_> {
        NodeLayout {
            process_ref: &data.0,
            process: data.1.as_ref(),
        }
    }

    #[test]
    fn auto_fit_empty_resets() {
        let mut state = FlowCanvasState::default();
        state.auto_fit(&[], &[], &[], false, Size::new(800.0, 600.0));
        assert!((state.zoom - 1.0).abs() < 0.01);
        assert!((state.scroll_offset.x).abs() < 0.01);
    }

    #[test]
    fn auto_fit_single_node() {
        let mut state = FlowCanvasState::default();
        let d = test_node_data("n", "", None);
        let node = as_layout(&d);
        state.auto_fit(&[node], &[], &[], false, Size::new(800.0, 600.0));
        assert!(state.zoom > 0.0);
        assert!(state.zoom <= MAX_ZOOM);
    }

    #[test]
    fn auto_fit_with_flow_io() {
        let mut state = FlowCanvasState::default();
        let d = test_node_data("n", "", None);
        let node = as_layout(&d);
        let input = IO::new_named(vec![DataType::from("string")], Route::default(), "in0");
        let output = IO::new_named(vec![DataType::from("string")], Route::default(), "out0");
        state.auto_fit(&[node], &[input], &[output], true, Size::new(800.0, 600.0));
        assert!(state.zoom > 0.0);
    }

    #[test]
    fn content_extents_nodes_only() {
        let d = test_node_data("n", "", None);
        let node = as_layout(&d);
        let (min_x, min_y, max_x, max_y) =
            FlowCanvasState::content_extents(&[node], &[], &[], false);
        assert!(min_x <= 100.0);
        assert!(min_y <= 100.0);
        assert!(max_x >= 280.0);
        assert!(max_y >= 220.0);
    }

    #[test]
    fn content_extents_with_flow_io() {
        let d = test_node_data("n", "", None);
        let node = as_layout(&d);
        let input = IO::new_named(vec![DataType::from("string")], Route::default(), "input0");
        let (min_x, _, max_x, _) = FlowCanvasState::content_extents(&[node], &[input], &[], true);
        assert!(max_x - min_x > 280.0);
    }

    #[test]
    fn trigger_auto_fit_when_enabled() {
        let mut win = WindowState {
            auto_fit_enabled: true,
            ..Default::default()
        };
        win.trigger_auto_fit_if_enabled();
        assert!(win.auto_fit_pending);
    }

    #[test]
    fn trigger_auto_fit_when_disabled() {
        let mut win = WindowState {
            auto_fit_enabled: false,
            ..Default::default()
        };
        win.trigger_auto_fit_if_enabled();
        assert!(!win.auto_fit_pending);
    }
}
