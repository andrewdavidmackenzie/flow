//! Canvas view module that renders flow process nodes and connections on an iced Canvas.
//!
//! Each [`ProcessReference`] is drawn as a rounded rectangle with its alias
//! displayed as a title. Node fill color is determined by the process source:
//! blue for `lib://`, green for `context://`, purple for provided implementations,
//! and orange for nested flows.

use std::collections::HashMap;

use iced::keyboard;
use iced::mouse;
use iced::widget::canvas::{
    self, Canvas, Event, Frame, Geometry, Path, Stroke, Text as CanvasText,
};
use iced::widget::{button, container, pick_list, stack, text_input, Column, Row, Text};
use iced::window;
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Size, Theme};
use log::info;

use flowcore::model::input::InputInitializer;

use crate::flow_io;
use crate::history::EditAction;
use crate::undo_redo;
use crate::InitializerEditor;
use crate::Message;
use crate::WindowState;

/// Action returned by [`handle_canvas_message`] to signal that the caller
/// (main.rs) needs to perform an operation that requires `FlowEdit` state.
pub(crate) enum CanvasAction {
    /// No further action needed.
    None,
    /// The user double-clicked a node — open it in a new window.
    OpenNode(usize),
}

/// Handle a [`CanvasMessage`] by mutating the given window state.
///
/// Returns a [`CanvasAction`] when the caller needs to perform cross-window
/// operations (e.g. opening a sub-flow in a new editor window).
pub(crate) fn handle_canvas_message(win: &mut WindowState, msg: CanvasMessage) -> CanvasAction {
    match msg {
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
                undo_redo::record_edit(
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
            if (old_x - new_x).abs() > 0.5
                || (old_y - new_y).abs() > 0.5
                || (old_w - new_w).abs() > 0.5
                || (old_h - new_h).abs() > 0.5
            {
                undo_redo::record_edit(
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
        }
        CanvasMessage::Deleted(idx) => {
            if idx < win.nodes.len() {
                let node = if let Some(node) = win.nodes.get(idx) {
                    node.clone()
                } else {
                    return CanvasAction::None;
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
                undo_redo::record_edit(
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
            undo_redo::record_edit(win, EditAction::CreateConnection { edge: edge.clone() });
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
                        flow_io::format_endpoint(&edge.from_node, &edge.from_port),
                        flow_io::format_endpoint(&edge.to_node, &edge.to_port),
                    );
                }
            } else {
                win.status = String::from("Ready");
            }
        }
        CanvasMessage::ConnectionDeleted(idx) => {
            if idx < win.edges.len() {
                let edge = win.edges.remove(idx);
                undo_redo::record_edit(win, EditAction::DeleteConnection { index: idx, edge });
                win.selected_connection = None;
                win.canvas_state.request_redraw();
                let nc = win.nodes.len();
                let ec = win.edges.len();
                win.status = format!("Connection deleted - {nc} nodes, {ec} connections");
            }
        }
        CanvasMessage::HoverChanged(data) => {
            win.tooltip = data;
        }
        CanvasMessage::AutoFitViewport(viewport) => {
            if win.auto_fit_enabled || win.auto_fit_pending {
                let has_flow_io = !win.flow_inputs.is_empty() || !win.flow_outputs.is_empty();
                win.canvas_state.auto_fit(&win.nodes, has_flow_io, viewport);
                win.auto_fit_pending = false;
            }
        }
        CanvasMessage::Pan(dx, dy) => {
            win.auto_fit_enabled = false; // Manual pan disables auto-fit
            win.auto_fit_pending = false;
            win.canvas_state.scroll_offset.x += dx;
            win.canvas_state.scroll_offset.y += dy;
            win.canvas_state.request_redraw();
        }
        CanvasMessage::ZoomBy(factor) => {
            win.auto_fit_enabled = false; // Manual zoom disables auto-fit
            win.auto_fit_pending = false;
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
            return CanvasAction::OpenNode(idx);
        }
        CanvasMessage::ContextMenu(x, y) => {
            win.context_menu = Some((x, y));
        }
    }
    CanvasAction::None
}

/// Handle the `ZoomIn` message by zooming in one step.
pub(crate) fn handle_zoom_in(win: &mut WindowState) {
    win.auto_fit_enabled = false;
    win.auto_fit_pending = false;
    win.canvas_state.zoom_in();
    let pct = (win.canvas_state.zoom * 100.0) as u32;
    win.status = format!("Zoom: {pct}%");
}

/// Handle the `ZoomOut` message by zooming out one step.
pub(crate) fn handle_zoom_out(win: &mut WindowState) {
    win.auto_fit_enabled = false;
    win.auto_fit_pending = false;
    win.canvas_state.zoom_out();
    let pct = (win.canvas_state.zoom * 100.0) as u32;
    win.status = format!("Zoom: {pct}%");
}

/// Handle the `ToggleAutoFit` message by toggling auto-fit mode.
pub(crate) fn handle_toggle_auto_fit(win: &mut WindowState) {
    win.auto_fit_enabled = !win.auto_fit_enabled;
    if win.auto_fit_enabled {
        win.auto_fit_pending = true;
        win.canvas_state.request_redraw();
        win.status = String::from("Auto-fit enabled");
    } else {
        win.status = String::from("Auto-fit disabled");
    }
}

/// Minimum allowed zoom level
const MIN_ZOOM: f32 = 0.1;
/// Maximum allowed zoom level
const MAX_ZOOM: f32 = 5.0;
/// Zoom factor applied per step (zoom-in multiplies, zoom-out divides)
const ZOOM_STEP: f32 = 1.1;
/// Padding in world units used when auto-fitting nodes into the viewport
const AUTO_FIT_PADDING: f32 = 50.0;
/// Scroll speed multiplier for panning with the scroll wheel (line-based)
const SCROLL_SPEED: f32 = 20.0;
/// Minimum allowed node width when resizing
const MIN_NODE_WIDTH: f32 = 120.0;
/// Minimum allowed node height when resizing
const MIN_NODE_HEIGHT: f32 = 80.0;
/// Half-size of resize handle squares in screen pixels
const RESIZE_HANDLE_HALF: f32 = 3.0;
/// Hit test radius for resize handles in screen pixels
const RESIZE_HANDLE_HIT: f32 = 6.0;
/// Hit test radius for port semi-circles in screen pixels
const PORT_HIT_RADIUS: f32 = 8.0;
/// Hit test distance for connection bezier curves in screen pixels
const CONNECTION_HIT_DISTANCE: f32 = 8.0;
/// Number of sample points along a bezier curve for hit testing
const BEZIER_SAMPLES: usize = 64;

use flowcore::model::connection::Connection;
use flowcore::model::process_reference::ProcessReference;
use flowcore::model::route::Route;

/// Messages produced by the canvas interaction layer.
#[derive(Debug, Clone)]
pub(crate) enum CanvasMessage {
    /// A node was selected (or deselected if `None`).
    Selected(Option<usize>),
    /// A node was moved to a new position (continuous during drag).
    Moved(usize, f32, f32),
    /// A node move completed (old_x, old_y, new_x, new_y) — for undo history.
    MoveCompleted(usize, f32, f32, f32, f32),
    /// A node was resized (index, new_x, new_y, new_width, new_height) — continuous during drag.
    Resized(usize, f32, f32, f32, f32),
    /// A node resize completed — for undo history.
    ResizeCompleted(usize, f32, f32, f32, f32, f32, f32, f32, f32),
    /// A node should be deleted.
    Deleted(usize),
    /// A new connection was created between two ports.
    ConnectionCreated {
        /// Source node alias
        from_node: String,
        /// Source port name
        from_port: String,
        /// Destination node alias
        to_node: String,
        /// Destination port name
        to_port: String,
    },
    /// A connection was selected (or deselected if `None`).
    ConnectionSelected(Option<usize>),
    /// A connection should be deleted.
    ConnectionDeleted(usize),
    /// Right-click on an input port to edit its initializer.
    /// (node_index, port_name)
    InitializerEdit(usize, String),
    /// Open a sub-flow or provided implementation in a new editor.
    OpenNode(usize),
    /// Pan the canvas by a world-space delta.
    Pan(f32, f32),
    /// Zoom the canvas by a multiplicative factor.
    ZoomBy(f32),
    /// Auto-fit with the actual viewport size (triggered on initial load).
    AutoFitViewport(Size),
    /// Hover state changed — full source path and screen position for tooltip (or None to hide)
    HoverChanged(Option<(String, f32, f32)>),
    /// Right-click on empty canvas — show context menu at screen position
    ContextMenu(f32, f32),
}

/// Tracks the drag-in-progress state: which node and the cursor offset from its origin.
#[derive(Debug, Clone)]
struct DragState {
    /// Index of the node being dragged
    node_index: usize,
    /// Horizontal offset from cursor to node origin at drag start
    offset_x: f32,
    /// Vertical offset from cursor to node origin at drag start
    offset_y: f32,
    /// Node position at drag start (for undo history)
    start_x: f32,
    /// Node position at drag start (for undo history)
    start_y: f32,
}

/// Which resize handle is being dragged.
#[derive(Debug, Clone, Copy)]
enum ResizeHandle {
    /// Top-left corner
    TopLeft,
    /// Top edge midpoint
    Top,
    /// Top-right corner
    TopRight,
    /// Left edge midpoint
    Left,
    /// Right edge midpoint
    Right,
    /// Bottom-left corner
    BottomLeft,
    /// Bottom edge midpoint
    Bottom,
    /// Bottom-right corner
    BottomRight,
}

/// Tracks a resize-in-progress state.
#[derive(Debug, Clone)]
struct ResizeState {
    /// Index of the node being resized
    node_index: usize,
    /// Which handle is being dragged
    handle: ResizeHandle,
    /// Cursor X in world space at drag start
    start_x: f32,
    /// Cursor Y in world space at drag start
    start_y: f32,
    /// Node X at drag start
    start_node_x: f32,
    /// Node Y at drag start
    start_node_y: f32,
    /// Node width at drag start
    start_width: f32,
    /// Node height at drag start
    start_height: f32,
}

/// Persistent interaction state for the canvas `Program`.
///
/// This is the `Program::State` associated type, kept alive across frames by iced.
#[derive(Debug, Clone, Default)]
pub(crate) struct CanvasInteractionState {
    /// Currently selected node index, if any
    selected_node: Option<usize>,
    /// Active drag operation, if any
    dragging: Option<DragState>,
    /// Active resize operation, if any
    resizing: Option<ResizeState>,
    /// Current keyboard modifier state (tracked via ModifiersChanged events)
    modifiers: keyboard::Modifiers,
    /// Active middle-mouse-button pan operation
    panning: Option<PanState>,
    /// Connection drag in progress
    connecting: Option<ConnectingState>,
    /// Currently selected connection index
    selected_connection: Option<usize>,
    /// Last known bounds size — used to detect window resize for auto-fit
    last_bounds: Option<Size>,
    /// Index of the node currently under the cursor (for hover tooltip)
    hover_node: Option<usize>,
}

/// Tracks a middle-mouse-button pan in progress.
#[derive(Debug, Clone)]
struct PanState {
    /// Last screen-space cursor position during the pan
    last_screen_pos: Point,
}

/// Tracks a connection drag in progress (started from a port).
#[derive(Debug, Clone)]
struct ConnectingState {
    /// Node alias of the starting port
    from_node: String,
    /// Port name of the starting port
    from_port: String,
    /// Whether we started from an output port (true) or input port (false)
    from_output: bool,
    /// World-space position of the starting port
    start_pos: Point,
    /// Current cursor position in screen space (updated during drag)
    current_screen_pos: Point,
}

/// Default node width when no layout width is specified
const DEFAULT_WIDTH: f32 = 180.0;
/// Default node height when no layout height is specified
const DEFAULT_HEIGHT: f32 = 120.0;
/// Horizontal spacing between auto-laid-out nodes
const GRID_SPACING_X: f32 = 250.0;
/// Vertical spacing between auto-laid-out nodes
const GRID_SPACING_Y: f32 = 170.0;
/// Number of columns in auto-layout grid
const GRID_COLUMNS: usize = 3;
/// Starting X offset for auto-layout
const GRID_ORIGIN_X: f32 = 50.0;
/// Starting Y offset for auto-layout
const GRID_ORIGIN_Y: f32 = 50.0;
/// Corner radius for rounded rectangles
const CORNER_RADIUS: f32 = 10.0;
/// Title font size (minimum readable)
const TITLE_FONT_SIZE: f32 = 16.0;
/// Source label font size (minimum readable)
const SOURCE_FONT_SIZE: f32 = 12.0;
/// Port label font size
const PORT_FONT_SIZE: f32 = 11.0;
/// Port circle radius
const PORT_RADIUS: f32 = 5.0;
/// Vertical spacing between ports
const PORT_SPACING: f32 = 20.0;
/// Vertical offset from top of node to first port
const PORT_START_Y: f32 = 55.0;
/// Maximum characters for source label before truncation
const MAX_SOURCE_CHARS: usize = 22;

/// Information about a port (input or output) on a node.
#[derive(Debug, Clone)]
pub(crate) struct PortInfo {
    /// The port name
    pub name: String,
    /// The data types accepted or produced by this port (stored for future hover display)
    #[allow(dead_code)]
    pub datatypes: Vec<String>,
}

impl PortInfo {
    #[cfg(test)]
    fn from_name(name: String) -> Self {
        Self {
            name,
            datatypes: Vec::new(),
        }
    }
}

/// A positioned node derived from a [`ProcessReference`], ready for rendering.
#[derive(Debug, Clone)]
pub(crate) struct NodeLayout {
    /// Display name (alias) for this node
    pub alias: String,
    /// Source path of the process
    pub(crate) source: String,
    /// Optional description of what this process does
    pub description: String,
    /// X coordinate on the canvas
    pub x: f32,
    /// Y coordinate on the canvas
    pub y: f32,
    /// Width of the node rectangle
    pub width: f32,
    /// Height of the node rectangle
    pub height: f32,
    /// Input ports with names and type information
    pub inputs: Vec<PortInfo>,
    /// Output ports with names and type information
    pub outputs: Vec<PortInfo>,
    /// Initializer display strings keyed by port name (e.g., "start" → "1 once")
    pub initializers: HashMap<String, String>,
}

impl Default for NodeLayout {
    fn default() -> Self {
        Self {
            alias: String::new(),
            source: String::new(),
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
}

impl NodeLayout {
    /// Determine the fill color based on the process source string.
    fn fill_color(&self) -> Color {
        if self.source.starts_with("lib://") {
            Color::from_rgb(0.3, 0.5, 0.9) // Blue for library
        } else if self.source.starts_with("context://") {
            Color::from_rgb(0.3, 0.75, 0.45) // Green for context
        } else if self.source.ends_with(".rs") || self.source.ends_with(".wasm") {
            Color::from_rgb(0.6, 0.3, 0.8) // Purple for provided implementations
        } else {
            Color::from_rgb(0.9, 0.6, 0.2) // Orange for nested flows
        }
    }

    /// Whether this node's source can be opened (sub-flow or provided implementation).
    pub(crate) fn is_openable(&self) -> bool {
        !self.source.starts_with("lib://") && !self.source.starts_with("context://")
    }

    /// Get the position of an output port (right edge of node)
    fn output_port_position(&self, port_index: usize) -> Point {
        Point::new(
            self.x + self.width,
            self.y + PORT_START_Y + port_index as f32 * PORT_SPACING,
        )
    }

    /// Get the position of an input port (left edge of node)
    fn input_port_position(&self, port_index: usize) -> Point {
        Point::new(
            self.x,
            self.y + PORT_START_Y + port_index as f32 * PORT_SPACING,
        )
    }

    /// Return the 8 resize handle positions in world coordinates.
    ///
    /// Order: TopLeft, Top, TopRight, Left, Right, BottomLeft, Bottom, BottomRight.
    fn resize_handle_positions(&self) -> [(ResizeHandle, Point); 8] {
        let mid_x = self.x + self.width / 2.0;
        let mid_y = self.y + self.height / 2.0;
        let right = self.x + self.width;
        let bottom = self.y + self.height;
        [
            (ResizeHandle::TopLeft, Point::new(self.x, self.y)),
            (ResizeHandle::Top, Point::new(mid_x, self.y)),
            (ResizeHandle::TopRight, Point::new(right, self.y)),
            (ResizeHandle::Left, Point::new(self.x, mid_y)),
            (ResizeHandle::Right, Point::new(right, mid_y)),
            (ResizeHandle::BottomLeft, Point::new(self.x, bottom)),
            (ResizeHandle::Bottom, Point::new(mid_x, bottom)),
            (ResizeHandle::BottomRight, Point::new(right, bottom)),
        ]
    }
}

/// A connection edge to render between two nodes
#[derive(Debug, Clone)]
pub(crate) struct EdgeLayout {
    /// Source node alias
    pub(crate) from_node: String,
    /// Source port name (may be empty for whole-node output)
    pub(crate) from_port: String,
    /// Destination node alias
    pub(crate) to_node: String,
    /// Destination port name
    pub(crate) to_port: String,
    /// Optional connection name for display on the line
    pub(crate) name: String,
}

/// Build a list of [`NodeLayout`] from process references and connections.
///
/// Ports and descriptions are taken from the resolved definitions loaded for each subprocess.
/// Layout uses the optional `x`, `y`, `width`, `height` fields from `ProcessReference`,
/// falling back to auto-grid positioning.
pub(crate) fn build_node_layouts(
    process_refs: &[ProcessReference],
    connections: &[Connection],
    resolved_ports: &HashMap<String, (Vec<PortInfo>, Vec<PortInfo>)>,
    subprocesses: &std::collections::BTreeMap<String, flowcore::model::process::Process>,
) -> Vec<NodeLayout> {
    let topo_positions = compute_topological_layout(process_refs, connections);

    let mut nodes = Vec::with_capacity(process_refs.len());

    for (i, pref) in process_refs.iter().enumerate() {
        let alias = if pref.alias.is_empty() {
            derive_short_name(&pref.source)
        } else {
            pref.alias.to_string()
        };

        let (inputs, outputs) = resolved_ports.get(&alias).cloned().unwrap_or_default();

        let min_ports = inputs.len().max(outputs.len());
        let min_height = PORT_START_Y + (min_ports as f32 + 1.0) * PORT_SPACING;

        // Use saved position, then topology position, then grid fallback
        let (default_x, default_y) = if let Some((tx, ty)) = topo_positions.get(&alias) {
            (*tx, *ty)
        } else {
            let col = i % GRID_COLUMNS;
            let row = i / GRID_COLUMNS;
            (
                GRID_ORIGIN_X + col as f32 * GRID_SPACING_X,
                GRID_ORIGIN_Y + row as f32 * GRID_SPACING_Y,
            )
        };
        let x = pref.x.unwrap_or(default_x);
        let y = pref.y.unwrap_or(default_y);
        let width = pref.width.unwrap_or(DEFAULT_WIDTH);
        let height = pref.height.unwrap_or(DEFAULT_HEIGHT.max(min_height));

        // Build initializer display strings
        let mut initializers = HashMap::new();
        for (port_name, init) in &pref.initializations {
            let display = match init {
                flowcore::model::input::InputInitializer::Once(v) => {
                    format!("once: {}", format_value(v))
                }
                flowcore::model::input::InputInitializer::Always(v) => {
                    format!("always: {}", format_value(v))
                }
            };
            initializers.insert(port_name.clone(), display);
        }

        // Extract description from the resolved subprocess definition
        let description = subprocesses
            .get(&alias)
            .map(|proc| match proc {
                flowcore::model::process::Process::FunctionProcess(func) => {
                    func.description.clone()
                }
                flowcore::model::process::Process::FlowProcess(flow) => flow.description.clone(),
            })
            .unwrap_or_default();

        nodes.push(NodeLayout {
            alias: alias.clone(),
            source: pref.source.clone(),
            description,
            x,
            y,
            width,
            height,
            inputs,
            outputs,
            initializers,
        });
    }

    nodes
}

impl EdgeLayout {
    /// Create a new edge layout with the given source and destination.
    pub(crate) fn new(
        from_node: String,
        from_port: String,
        to_node: String,
        to_port: String,
    ) -> Self {
        Self {
            from_node,
            from_port,
            to_node,
            to_port,
            name: String::new(),
        }
    }

    /// Check whether this edge references the given node alias as source or destination.
    pub(crate) fn references_node(&self, alias: &str) -> bool {
        self.from_node == alias || self.to_node == alias
    }
}

/// Compute topology-based positions for nodes without saved layout.
///
/// Assigns each node a column based on its depth from source nodes (nodes with no
/// incoming connections). Nodes are spread vertically within each column.
fn compute_topological_layout(
    process_refs: &[ProcessReference],
    connections: &[Connection],
) -> HashMap<String, (f32, f32)> {
    // Build alias list
    let aliases: Vec<String> = process_refs
        .iter()
        .map(|p| {
            if p.alias.is_empty() {
                derive_short_name(&p.source)
            } else {
                p.alias.to_string()
            }
        })
        .collect();

    // Build adjacency: which nodes feed which
    let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    for alias in &aliases {
        incoming.entry(alias.clone()).or_default();
        outgoing.entry(alias.clone()).or_default();
    }

    for conn in connections {
        let from_route = conn.from().to_string();
        let (from_node, _) = split_route(&from_route);
        for to_route in conn.to() {
            let to_str = to_route.to_string();
            let (to_node, _) = split_route(&to_str);
            if from_node != to_node {
                // Skip self-loops for layout purposes
                outgoing
                    .entry(from_node.clone())
                    .or_default()
                    .push(to_node.clone());
                incoming.entry(to_node).or_default().push(from_node.clone());
            }
        }
    }

    // Assign column depth using BFS from source nodes (no incoming edges)
    let mut depth: HashMap<String, usize> = HashMap::new();
    let mut queue: std::collections::VecDeque<String> = std::collections::VecDeque::new();

    for alias in &aliases {
        if incoming.get(alias).is_none_or(std::vec::Vec::is_empty) {
            depth.insert(alias.clone(), 0);
            queue.push_back(alias.clone());
        }
    }

    // BFS to assign max depth (longest path from any source).
    // Cap depth to prevent infinite loops on cyclic flows (e.g., fibonacci feedback).
    let max_depth = aliases.len().saturating_sub(1);
    while let Some(node) = queue.pop_front() {
        let node_depth = depth.get(&node).copied().unwrap_or(0);
        if let Some(neighbors) = outgoing.get(&node) {
            for neighbor in neighbors {
                let new_depth = (node_depth + 1).min(max_depth);
                let current = depth.get(neighbor).copied().unwrap_or(0);
                if new_depth > current {
                    depth.insert(neighbor.clone(), new_depth);
                    queue.push_back(neighbor.clone());
                }
            }
        }
    }

    // Assign any unvisited nodes depth 0
    for alias in &aliases {
        depth.entry(alias.clone()).or_insert(0);
    }

    // Group nodes by column
    let mut columns: HashMap<usize, Vec<String>> = HashMap::new();
    for alias in &aliases {
        let col = depth.get(alias).copied().unwrap_or(0);
        columns.entry(col).or_default().push(alias.clone());
    }

    // Compute positions: spread columns horizontally, nodes vertically within each column
    let mut positions = HashMap::new();
    for (col, col_nodes) in &columns {
        let x = GRID_ORIGIN_X + *col as f32 * GRID_SPACING_X;
        let total_height = col_nodes.len() as f32 * GRID_SPACING_Y;
        let start_y = GRID_ORIGIN_Y + (GRID_SPACING_Y - total_height) / 2.0;

        for (row, alias) in col_nodes.iter().enumerate() {
            let y = start_y.max(GRID_ORIGIN_Y) + row as f32 * GRID_SPACING_Y;
            positions.insert(alias.clone(), (x, y));
        }
    }

    positions
}

/// Build edge layouts from flow connections
pub(crate) fn build_edge_layouts(connections: &[Connection]) -> Vec<EdgeLayout> {
    let mut edges = Vec::new();

    for conn in connections {
        let from_route = conn.from().to_string();
        let (from_node, from_port) = split_route(&from_route);

        for to_route in conn.to() {
            let to_str = to_route.to_string();
            let (to_node, to_port) = split_route(&to_str);
            edges.push(EdgeLayout {
                from_node: from_node.clone(),
                from_port: from_port.clone(),
                to_node,
                to_port,
                name: conn.name().to_string(),
            });
        }
    }

    edges
}

/// Format a serde_json::Value for compact display
fn format_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => format!("\"{s}\""),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Array(a) => {
            if a.len() <= 3 {
                format!(
                    "[{}]",
                    a.iter().map(format_value).collect::<Vec<_>>().join(",")
                )
            } else {
                format!("[{}...]", a.len())
            }
        }
        serde_json::Value::Object(_) => "{...}".to_string(),
    }
}

/// Derive a short display name from a source URL.
/// e.g., `"lib://flowstdlib/math/sequence"` → `"sequence"`
/// e.g., `"context://stdio/stdout"` → `"stdout"`
pub(crate) fn derive_short_name(source: &str) -> String {
    source.rsplit('/').next().unwrap_or(source).to_string()
}

/// Split a route string like "sequence/number" into ("sequence", "number")
/// or "add1" into ("add1", "")
fn split_route(route: &str) -> (String, String) {
    let route = route.trim_start_matches('/');
    if let Some(pos) = route.find('/') {
        (route[..pos].to_string(), route[pos + 1..].to_string())
    } else {
        (route.to_string(), String::new())
    }
}

/// Persistent canvas state that caches the rendered geometry.
pub(crate) struct FlowCanvasState {
    /// The geometry cache — cleared when the flow data changes
    cache: canvas::Cache,
    /// Current zoom level (1.0 = 100%)
    pub zoom: f32,
    /// Scroll offset in world coordinates
    pub scroll_offset: Point,
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
    /// Create the canvas [`Element`] for displaying the given nodes and edges.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn view<'a>(
        &'a self,
        nodes: &'a [NodeLayout],
        edges: &'a [EdgeLayout],
        flow_name: &'a str,
        flow_inputs: &'a [PortInfo],
        flow_outputs: &'a [PortInfo],
        is_subflow: bool,
        auto_fit_pending: bool,
        auto_fit_enabled: bool,
    ) -> Element<'a, CanvasMessage> {
        Canvas::new(FlowCanvas {
            state: self,
            nodes,
            edges,
            flow_name,
            flow_inputs,
            flow_outputs,
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
    pub(crate) fn auto_fit(&mut self, nodes: &[NodeLayout], has_flow_io: bool, viewport: Size) {
        if nodes.is_empty() && !has_flow_io {
            self.zoom = 1.0;
            self.scroll_offset = Point::new(0.0, 0.0);
            self.cache.clear();
            return;
        }

        // Extra margin when flow I/O bounding box is drawn (padding + port labels)
        let flow_io_margin = if has_flow_io { 200.0 } else { 0.0 };

        let (mut min_x, mut min_y, mut max_x, mut max_y) = if nodes.is_empty() {
            (150.0, 50.0, 350.0, 450.0)
        } else {
            (f32::MAX, f32::MAX, f32::MIN, f32::MIN)
        };
        for node in nodes {
            let init_margin = if node.initializers.is_empty() {
                0.0
            } else {
                let max_len = node
                    .initializers
                    .values()
                    .map(|s| s.len())
                    .max()
                    .unwrap_or(0);
                max_len as f32 * 8.0
            };
            if node.x - init_margin < min_x {
                min_x = node.x - init_margin;
            }
            if node.y < min_y {
                min_y = node.y;
            }
            if node.x + node.width > max_x {
                max_x = node.x + node.width;
            }
            if node.y + node.height > max_y {
                max_y = node.y + node.height;
            }
        }

        let content_width = max_x - min_x + AUTO_FIT_PADDING * 2.0 + flow_io_margin * 2.0;
        let content_height = max_y - min_y + AUTO_FIT_PADDING * 2.0 + flow_io_margin;

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
        let content_center_x = (min_x + max_x) / 2.0;
        let content_center_y = (min_y + max_y) / 2.0;
        let viewport_center_x = viewport.width / 2.0 / self.zoom;
        let viewport_center_y = viewport.height / 2.0 / self.zoom;

        self.scroll_offset = Point::new(
            viewport_center_x - content_center_x,
            viewport_center_y - content_center_y,
        );
        self.cache.clear();
    }
}

/// Transform a world-space point to screen-space using the given zoom and scroll offset.
fn transform_point(p: Point, zoom: f32, offset: Point) -> Point {
    Point::new((p.x + offset.x) * zoom, (p.y + offset.y) * zoom)
}

/// Convert a screen-space point back to world-space.
fn screen_to_world(screen: Point, zoom: f32, offset: Point) -> Point {
    Point::new(screen.x / zoom - offset.x, screen.y / zoom - offset.y)
}

/// The canvas program that draws flow nodes and connections.
struct FlowCanvas<'a> {
    /// Reference to the persistent canvas state (zoom, offset, cache)
    state: &'a FlowCanvasState,
    /// Nodes to render
    nodes: &'a [NodeLayout],
    /// Edges to render
    edges: &'a [EdgeLayout],
    /// Flow name (displayed on sub-flow bounding box)
    flow_name: &'a str,
    /// Flow-level input ports (displayed on left edge for sub-flows)
    flow_inputs: &'a [PortInfo],
    /// Flow-level output ports (displayed on right edge for sub-flows)
    flow_outputs: &'a [PortInfo],
    /// Whether this is a sub-flow (always draws bounding box)
    is_subflow: bool,
    /// Whether an auto-fit should be triggered on the next event
    auto_fit_pending: bool,
    /// Whether auto-fit mode is active (continuously fits to window)
    auto_fit_enabled: bool,
}

/// Find the index of the first node whose bounding rectangle contains `point`.
fn hit_test_node(nodes: &[NodeLayout], point: Point) -> Option<usize> {
    nodes.iter().enumerate().find_map(|(i, node)| {
        if point.x >= node.x
            && point.x <= node.x + node.width
            && point.y >= node.y
            && point.y <= node.y + node.height
        {
            Some(i)
        } else {
            None
        }
    })
}

/// Check whether `point` (world coords) is within the source text zone of a node.
/// The source text zone is the area where the source path is displayed, centered
/// horizontally at 34px below the node top.
fn is_in_source_text_zone(node: &NodeLayout, point: Point) -> bool {
    let text_center_x = node.x + node.width / 2.0;
    let text_top_y = node.y + 34.0;
    let text_height = SOURCE_FONT_SIZE + 4.0;
    let text_half_width = node.width * 0.4;

    point.x >= text_center_x - text_half_width
        && point.x <= text_center_x + text_half_width
        && point.y >= text_top_y
        && point.y <= text_top_y + text_height
}

/// Check whether `point` (world coords) is on the open icon of an openable node.
/// The icon occupies a 16x16 area in the top-right corner of the node.
fn hit_test_open_icon(nodes: &[NodeLayout], point: Point) -> Option<usize> {
    nodes.iter().enumerate().find_map(|(i, node)| {
        if !node.is_openable() {
            return None;
        }
        let icon_x = node.x + node.width - 22.0;
        let icon_y = node.y + 4.0;
        if point.x >= icon_x
            && point.x <= icon_x + 24.0
            && point.y >= icon_y
            && point.y <= icon_y + 24.0
        {
            Some(i)
        } else {
            None
        }
    })
}

/// Check whether `screen_pos` is within [`RESIZE_HANDLE_HIT`] pixels of any resize handle
/// on the node at `node_index`. Returns the handle variant if hit.
///
/// The hit test is performed in screen space so the grab area is constant regardless of zoom.
fn hit_test_resize_handle(
    node: &NodeLayout,
    node_index: usize,
    screen_pos: Point,
    zoom: f32,
    offset: Point,
) -> Option<(usize, ResizeHandle)> {
    for (handle, world_pt) in &node.resize_handle_positions() {
        let screen_pt = transform_point(*world_pt, zoom, offset);
        let dx = (screen_pos.x - screen_pt.x).abs();
        let dy = (screen_pos.y - screen_pt.y).abs();
        if dx <= RESIZE_HANDLE_HIT && dy <= RESIZE_HANDLE_HIT {
            return Some((node_index, *handle));
        }
    }
    None
}

/// Hit test all ports across all nodes.
///
/// Returns `(node_index, port_name, is_output)` if the cursor is within
/// [`PORT_HIT_RADIUS`] screen pixels of a port center.
fn hit_test_port(
    nodes: &[NodeLayout],
    screen_pos: Point,
    zoom: f32,
    offset: Point,
) -> Option<(usize, String, bool)> {
    for (node_idx, node) in nodes.iter().enumerate() {
        // Check output ports (right side)
        for (port_idx, port_info) in node.outputs.iter().enumerate() {
            let world_pt = node.output_port_position(port_idx);
            let screen_pt = transform_point(world_pt, zoom, offset);
            let dx = screen_pos.x - screen_pt.x;
            let dy = screen_pos.y - screen_pt.y;
            if dx * dx + dy * dy <= PORT_HIT_RADIUS * PORT_HIT_RADIUS {
                return Some((node_idx, port_info.name.clone(), true));
            }
        }
        // Check input ports (left side)
        for (port_idx, port_info) in node.inputs.iter().enumerate() {
            let world_pt = node.input_port_position(port_idx);
            let screen_pt = transform_point(world_pt, zoom, offset);
            let dx = screen_pos.x - screen_pt.x;
            let dy = screen_pos.y - screen_pt.y;
            if dx * dx + dy * dy <= PORT_HIT_RADIUS * PORT_HIT_RADIUS {
                return Some((node_idx, port_info.name.clone(), false));
            }
        }
    }
    None
}

/// Evaluate a quadratic bezier curve at parameter `t` (0.0..=1.0).
fn quadratic_bezier_pt(p0: Point, p1: Point, p2: Point, t: f32) -> Point {
    let mt = 1.0 - t;
    Point::new(
        mt * mt * p0.x + 2.0 * mt * t * p1.x + t * t * p2.x,
        mt * mt * p0.y + 2.0 * mt * t * p1.y + t * t * p2.y,
    )
}

/// Evaluate a cubic bezier curve at parameter `t` (0.0..=1.0).
/// Compute flow I/O port world positions (same layout as draw_flow_io_ports).
fn compute_flow_io_positions(
    nodes: &[NodeLayout],
    flow_inputs: &[PortInfo],
    flow_outputs: &[PortInfo],
) -> (HashMap<String, Point>, HashMap<String, Point>) {
    use std::collections::HashMap;

    let mut input_positions = HashMap::new();
    let mut output_positions = HashMap::new();

    if flow_inputs.is_empty() && flow_outputs.is_empty() {
        return (input_positions, output_positions);
    }

    let padding = 80.0;
    let spacing = 28.0;
    let max_ports = flow_inputs.len().max(flow_outputs.len()).max(1) as f32;
    let default_h = max_ports * spacing + 60.0;
    let (min_x, max_x, min_y, max_y) = if nodes.is_empty() {
        (150.0, 350.0, 100.0, 100.0 + default_h)
    } else {
        (
            nodes.iter().map(|n| n.x).fold(f32::MAX, f32::min),
            nodes.iter().map(|n| n.x + n.width).fold(f32::MIN, f32::max),
            nodes.iter().map(|n| n.y).fold(f32::MAX, f32::min),
            nodes
                .iter()
                .map(|n| n.y + n.height)
                .fold(f32::MIN, f32::max),
        )
    };
    let box_x = min_x - padding;
    let box_w = (max_x - min_x) + 2.0 * padding;
    let center_y = (min_y + max_y) / 2.0;

    let input_start_y = center_y - (flow_inputs.len() as f32 - 1.0) * spacing / 2.0;
    for (i, input) in flow_inputs.iter().enumerate() {
        let y = input_start_y + i as f32 * spacing;
        input_positions.insert(input.name.clone(), Point::new(box_x, y));
    }

    let right_x = box_x + box_w;
    let output_start_y = center_y - (flow_outputs.len() as f32 - 1.0) * spacing / 2.0;
    for (i, output) in flow_outputs.iter().enumerate() {
        let y = output_start_y + i as f32 * spacing;
        output_positions.insert(output.name.clone(), Point::new(right_x, y));
    }

    (input_positions, output_positions)
}

/// Extract the base port name, stripping any trailing array index.
/// Uses flowcore's Route to detect array selectors properly.
fn base_port_name(port: &str) -> &str {
    if Route::from(port).is_array_selector() {
        port.rsplit_once('/').map_or(port, |(base, _)| base)
    } else {
        port
    }
}

fn find_node_output_pos_inline(node: &NodeLayout, port: &str) -> Point {
    if port.is_empty() {
        node.output_port_position(0)
    } else {
        let base = base_port_name(port);
        let idx = node
            .outputs
            .iter()
            .position(|p| p.name == base)
            .unwrap_or(0);
        node.output_port_position(idx)
    }
}

fn find_node_input_pos_inline(node: &NodeLayout, port: &str) -> Point {
    if port.is_empty() {
        node.input_port_position(0)
    } else {
        let base = base_port_name(port);
        let idx = node.inputs.iter().position(|p| p.name == base).unwrap_or(0);
        node.input_port_position(idx)
    }
}

/// Squared distance from point `p` to the line segment `a`–`b`.
fn distance_to_segment_sq(p: Point, a: Point, b: Point) -> f32 {
    let ab_x = b.x - a.x;
    let ab_y = b.y - a.y;
    let ap_x = p.x - a.x;
    let ap_y = p.y - a.y;
    let ab_len_sq = ab_x * ab_x + ab_y * ab_y;
    if ab_len_sq < 0.001 {
        return ap_x * ap_x + ap_y * ap_y;
    }
    let t = ((ap_x * ab_x + ap_y * ab_y) / ab_len_sq).clamp(0.0, 1.0);
    let proj_x = a.x + t * ab_x;
    let proj_y = a.y + t * ab_y;
    let dx = p.x - proj_x;
    let dy = p.y - proj_y;
    dx * dx + dy * dy
}

fn cubic_bezier(p0: Point, p1: Point, p2: Point, p3: Point, t: f32) -> Point {
    let u = 1.0 - t;
    let uu = u * u;
    let uuu = uu * u;
    let tt = t * t;
    let ttt = tt * t;
    Point::new(
        uuu * p0.x + 3.0 * uu * t * p1.x + 3.0 * u * tt * p2.x + ttt * p3.x,
        uuu * p0.y + 3.0 * uu * t * p1.y + 3.0 * u * tt * p2.y + ttt * p3.y,
    )
}

/// Hit test connections by sampling points along each edge's bezier curve.
///
/// Returns the edge index if the cursor is within [`CONNECTION_HIT_DISTANCE`]
/// screen pixels of any sample point on the curve.
#[allow(clippy::too_many_arguments)]
fn hit_test_connection(
    edges: &[EdgeLayout],
    nodes: &[NodeLayout],
    flow_inputs: &[PortInfo],
    flow_outputs: &[PortInfo],
    screen_pos: Point,
    zoom: f32,
    offset: Point,
) -> Option<usize> {
    use std::collections::HashMap;
    let node_map: HashMap<&str, &NodeLayout> =
        nodes.iter().map(|n| (n.alias.as_str(), n)).collect();

    // Compute flow I/O port positions (same layout as draw_flow_io_ports)
    let flow_io_positions = compute_flow_io_positions(nodes, flow_inputs, flow_outputs);

    let threshold_sq = CONNECTION_HIT_DISTANCE * CONNECTION_HIT_DISTANCE;

    for (edge_idx, edge) in edges.iter().enumerate() {
        // Resolve from_point
        let from_point = if edge.from_node == "input" {
            let input_name = base_port_name(&edge.from_port);
            flow_io_positions.0.get(input_name).copied()
        } else {
            node_map
                .get(edge.from_node.as_str())
                .map(|n| find_node_output_pos_inline(n, &edge.from_port))
        };

        let to_point = if edge.to_node == "output" {
            let output_name = base_port_name(&edge.to_port);
            flow_io_positions.1.get(output_name).copied()
        } else {
            node_map
                .get(edge.to_node.as_str())
                .map(|n| find_node_input_pos_inline(n, &edge.to_port))
        };

        if let (Some(from_point), Some(to_point)) = (from_point, to_point) {
            let from_s = transform_point(from_point, zoom, offset);
            let to_s = transform_point(to_point, zoom, offset);

            let is_self = edge.from_node == edge.to_node;

            // Build sample points along the actual drawn path
            let sample_points: Vec<Point> = if is_self {
                let from_node_ref = node_map.get(edge.from_node.as_str());
                let Some(from_n) = from_node_ref else {
                    continue;
                };
                let (box_right, box_bottom, box_left, mid_x) = loopback_waypoints(
                    from_n.x,
                    from_n.y,
                    from_n.width,
                    from_n.height,
                    zoom,
                    offset,
                );

                // Sample the path: from -> right -> curve down -> bottom -> curve up -> to
                let mut pts = Vec::with_capacity(BEZIER_SAMPLES + 1);
                let segments = BEZIER_SAMPLES / 5;
                // Segment 1: from -> right
                for i in 0..=segments {
                    let t = i as f32 / segments as f32;
                    pts.push(Point::new(from_s.x + (box_right - from_s.x) * t, from_s.y));
                }
                // Segment 2: curve right -> bottom
                for i in 0..=segments {
                    let t = i as f32 / segments as f32;
                    let p = quadratic_bezier_pt(
                        Point::new(box_right, from_s.y),
                        Point::new(box_right, box_bottom),
                        Point::new(mid_x, box_bottom),
                        t,
                    );
                    pts.push(p);
                }
                // Segment 3: curve bottom -> left
                for i in 0..=segments {
                    let t = i as f32 / segments as f32;
                    let p = quadratic_bezier_pt(
                        Point::new(mid_x, box_bottom),
                        Point::new(box_left, box_bottom),
                        Point::new(box_left, to_s.y),
                        t,
                    );
                    pts.push(p);
                }
                // Segment 4: left -> to
                for i in 0..=segments {
                    let t = i as f32 / segments as f32;
                    pts.push(Point::new(box_left + (to_s.x - box_left) * t, to_s.y));
                }
                pts
            } else {
                // Use matching control points for flow I/O vs normal connections
                let is_flow_io = edge.from_node == "input" || edge.to_node == "output";
                let dx_ctrl = if is_flow_io {
                    (to_s.x - from_s.x).abs().max(40.0 * zoom) * 0.4
                } else {
                    (to_s.x - from_s.x).abs().max(60.0 * zoom) * 0.5
                };
                let control1 = Point::new(from_s.x + dx_ctrl, from_s.y);
                let control2 = Point::new(to_s.x - dx_ctrl, to_s.y);
                (0..=BEZIER_SAMPLES)
                    .map(|i| {
                        let t = i as f32 / BEZIER_SAMPLES as f32;
                        cubic_bezier(from_s, control1, control2, to_s, t)
                    })
                    .collect()
            };

            for pair in sample_points.windows(2) {
                if let [a, b] = *pair {
                    if distance_to_segment_sq(screen_pos, a, b) <= threshold_sq {
                        return Some(edge_idx);
                    }
                }
            }
        }
    }
    None
}

/// Compute the appropriate mouse cursor for a given [`ResizeHandle`].
fn resize_cursor(handle: &ResizeHandle) -> mouse::Interaction {
    match handle {
        ResizeHandle::TopLeft | ResizeHandle::BottomRight => {
            mouse::Interaction::ResizingDiagonallyDown
        }
        ResizeHandle::TopRight | ResizeHandle::BottomLeft => {
            mouse::Interaction::ResizingDiagonallyUp
        }
        ResizeHandle::Left | ResizeHandle::Right => mouse::Interaction::ResizingHorizontally,
        ResizeHandle::Top | ResizeHandle::Bottom => mouse::Interaction::ResizingVertically,
    }
}

impl canvas::Program<CanvasMessage> for FlowCanvas<'_> {
    type State = CanvasInteractionState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<CanvasMessage>> {
        // Trigger auto-fit when pending or when auto-fit is enabled and bounds changed
        let bounds_changed = state.last_bounds.is_none_or(|last| {
            (last.width - bounds.width).abs() > 1.0 || (last.height - bounds.height).abs() > 1.0
        });
        if self.auto_fit_pending || (self.auto_fit_enabled && bounds_changed) {
            state.last_bounds = Some(bounds.size());
            return Some(
                canvas::Action::publish(CanvasMessage::AutoFitViewport(bounds.size()))
                    .and_capture(),
            );
        }

        // Handle keyboard events before cursor position check — keyboard events
        // should work even when the cursor is off-canvas
        match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.modifiers = *modifiers;
                return None;
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key:
                    keyboard::Key::Named(keyboard::key::Named::Delete | keyboard::key::Named::Backspace),
                ..
            }) => {
                if let Some(sel_conn) = state.selected_connection {
                    state.selected_connection = None;
                    return Some(
                        canvas::Action::publish(CanvasMessage::ConnectionDeleted(sel_conn))
                            .and_capture(),
                    );
                }
                if let Some(sel_idx) = state.selected_node {
                    state.selected_node = None;
                    return Some(
                        canvas::Action::publish(CanvasMessage::Deleted(sel_idx)).and_capture(),
                    );
                }
                return None;
            }
            // Clear stuck drag/resize/connect states when mouse released off-canvas
            Event::Mouse(mouse::Event::ButtonReleased(_))
                if cursor.position_in(bounds).is_none() =>
            {
                state.connecting = None;
                state.resizing = None;
                state.dragging = None;
                state.panning = None;
                return Some(canvas::Action::request_redraw());
            }
            _ => {}
        }

        let cursor_position = cursor.position_in(bounds)?;
        let zoom = self.state.zoom;
        let offset = self.state.scroll_offset;
        let world_pos = screen_to_world(cursor_position, zoom, offset);

        match event {
            // Left mouse button pressed — check resize handles, ports, connections, nodes, or deselect
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                // 1. Check if cursor is on a resize handle of the selected node
                if let Some(sel_idx) = state.selected_node {
                    if let Some(sel_node) = self.nodes.get(sel_idx) {
                        if let Some((_idx, handle)) =
                            hit_test_resize_handle(sel_node, sel_idx, cursor_position, zoom, offset)
                        {
                            state.resizing = Some(ResizeState {
                                node_index: sel_idx,
                                handle,
                                start_x: world_pos.x,
                                start_y: world_pos.y,
                                start_node_x: sel_node.x,
                                start_node_y: sel_node.y,
                                start_width: sel_node.width,
                                start_height: sel_node.height,
                            });
                            return Some(canvas::Action::request_redraw().and_capture());
                        }
                    }
                }

                // 2. Check if cursor is near a connection line (but NOT on a port) — select it
                let on_a_port = hit_test_port(self.nodes, cursor_position, zoom, offset).is_some();
                if !on_a_port {
                    if let Some(edge_idx) = hit_test_connection(
                        self.edges,
                        self.nodes,
                        self.flow_inputs,
                        self.flow_outputs,
                        cursor_position,
                        zoom,
                        offset,
                    ) {
                        state.selected_connection = Some(edge_idx);
                        state.selected_node = None;
                        state.dragging = None;
                        return Some(
                            canvas::Action::publish(CanvasMessage::ConnectionSelected(Some(
                                edge_idx,
                            )))
                            .and_capture(),
                        );
                    }
                }

                // 3. Check if cursor is on a port — start connection drag
                if let Some((node_idx, port_name, is_output)) =
                    hit_test_port(self.nodes, cursor_position, zoom, offset)
                {
                    if let Some(node) = self.nodes.get(node_idx) {
                        let port_world_pos = if is_output {
                            let port_idx = node
                                .outputs
                                .iter()
                                .position(|p| p.name == port_name)
                                .unwrap_or(0);
                            node.output_port_position(port_idx)
                        } else {
                            let port_idx = node
                                .inputs
                                .iter()
                                .position(|p| p.name == port_name)
                                .unwrap_or(0);
                            node.input_port_position(port_idx)
                        };
                        state.connecting = Some(ConnectingState {
                            from_node: node.alias.clone(),
                            from_port: port_name,
                            from_output: is_output,
                            start_pos: port_world_pos,
                            current_screen_pos: cursor_position,
                        });
                        return Some(canvas::Action::request_redraw().and_capture());
                    }
                }

                // 4. Check if cursor is on an openable node's open icon
                if let Some(idx) = hit_test_open_icon(self.nodes, world_pos) {
                    return Some(
                        canvas::Action::publish(CanvasMessage::OpenNode(idx)).and_capture(),
                    );
                }

                // 6. Check if cursor is on a node — select/drag it
                if let Some(idx) = hit_test_node(self.nodes, world_pos) {
                    let node = self.nodes.get(idx)?;
                    state.selected_node = Some(idx);
                    state.selected_connection = None;
                    state.dragging = Some(DragState {
                        node_index: idx,
                        offset_x: world_pos.x - node.x,
                        offset_y: world_pos.y - node.y,
                        start_x: node.x,
                        start_y: node.y,
                    });
                    Some(canvas::Action::publish(CanvasMessage::Selected(Some(idx))).and_capture())
                } else {
                    // 7. Clicked empty canvas — deselect all
                    state.selected_node = None;
                    state.selected_connection = None;
                    state.dragging = None;
                    Some(canvas::Action::publish(CanvasMessage::Selected(None)).and_capture())
                }
            }

            // Right mouse button pressed — edit initializer on input port, or context menu
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                if let Some((node_idx, port_name, is_output)) =
                    hit_test_port(self.nodes, cursor_position, zoom, offset)
                {
                    if !is_output {
                        return Some(
                            canvas::Action::publish(CanvasMessage::InitializerEdit(
                                node_idx, port_name,
                            ))
                            .and_capture(),
                        );
                    }
                }
                // Right-click on empty canvas — show context menu
                if hit_test_node(self.nodes, world_pos).is_none() {
                    return Some(
                        canvas::Action::publish(CanvasMessage::ContextMenu(
                            cursor_position.x,
                            cursor_position.y,
                        ))
                        .and_capture(),
                    );
                }
                None
            }

            // Middle mouse button pressed — start panning
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)) => {
                state.panning = Some(PanState {
                    last_screen_pos: cursor_position,
                });
                Some(canvas::Action::request_redraw().and_capture())
            }

            // Mouse moved — handle connecting, resize, drag, or pan
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(ref mut connecting) = state.connecting {
                    connecting.current_screen_pos = cursor_position;
                    return Some(canvas::Action::request_redraw().and_capture());
                }
                if let Some(ref resize) = state.resizing {
                    let dx = world_pos.x - resize.start_x;
                    let dy = world_pos.y - resize.start_y;
                    let (mut new_x, mut new_y, mut new_w, mut new_h) = (
                        resize.start_node_x,
                        resize.start_node_y,
                        resize.start_width,
                        resize.start_height,
                    );
                    match resize.handle {
                        ResizeHandle::TopLeft => {
                            new_w = (resize.start_width - dx).max(MIN_NODE_WIDTH);
                            new_h = (resize.start_height - dy).max(MIN_NODE_HEIGHT);
                            // Position moves by the amount size didn't change due to clamping
                            new_x = resize.start_node_x + resize.start_width - new_w;
                            new_y = resize.start_node_y + resize.start_height - new_h;
                        }
                        ResizeHandle::Top => {
                            new_h = (resize.start_height - dy).max(MIN_NODE_HEIGHT);
                            new_y = resize.start_node_y + resize.start_height - new_h;
                        }
                        ResizeHandle::TopRight => {
                            new_w = (resize.start_width + dx).max(MIN_NODE_WIDTH);
                            new_h = (resize.start_height - dy).max(MIN_NODE_HEIGHT);
                            new_y = resize.start_node_y + resize.start_height - new_h;
                        }
                        ResizeHandle::Left => {
                            new_w = (resize.start_width - dx).max(MIN_NODE_WIDTH);
                            new_x = resize.start_node_x + resize.start_width - new_w;
                        }
                        ResizeHandle::Right => {
                            new_w = (resize.start_width + dx).max(MIN_NODE_WIDTH);
                        }
                        ResizeHandle::BottomLeft => {
                            new_w = (resize.start_width - dx).max(MIN_NODE_WIDTH);
                            new_h = (resize.start_height + dy).max(MIN_NODE_HEIGHT);
                            new_x = resize.start_node_x + resize.start_width - new_w;
                        }
                        ResizeHandle::Bottom => {
                            new_h = (resize.start_height + dy).max(MIN_NODE_HEIGHT);
                        }
                        ResizeHandle::BottomRight => {
                            new_w = (resize.start_width + dx).max(MIN_NODE_WIDTH);
                            new_h = (resize.start_height + dy).max(MIN_NODE_HEIGHT);
                        }
                    }
                    let idx = resize.node_index;
                    Some(
                        canvas::Action::publish(CanvasMessage::Resized(
                            idx, new_x, new_y, new_w, new_h,
                        ))
                        .and_capture(),
                    )
                } else if let Some(ref pan) = state.panning {
                    // Pan: adjust scroll_offset based on screen-space delta
                    let dx = (cursor_position.x - pan.last_screen_pos.x) / zoom;
                    let dy = (cursor_position.y - pan.last_screen_pos.y) / zoom;
                    state.panning = Some(PanState {
                        last_screen_pos: cursor_position,
                    });
                    Some(canvas::Action::publish(CanvasMessage::Pan(dx, dy)).and_capture())
                } else if let Some(ref drag) = state.dragging {
                    let new_x = world_pos.x - drag.offset_x;
                    let new_y = world_pos.y - drag.offset_y;
                    Some(
                        canvas::Action::publish(CanvasMessage::Moved(
                            drag.node_index,
                            new_x,
                            new_y,
                        ))
                        .and_capture(),
                    )
                } else {
                    // Check port hover for type tooltip
                    if let Some((node_idx, port_name, is_output)) =
                        hit_test_port(self.nodes, cursor_position, zoom, offset)
                    {
                        if let Some(node) = self.nodes.get(node_idx) {
                            let ports = if is_output {
                                &node.outputs
                            } else {
                                &node.inputs
                            };
                            let type_text = ports
                                .iter()
                                .find(|p| p.name == port_name)
                                .map(|p| {
                                    if p.datatypes.is_empty() {
                                        format!("{port_name}: (any)")
                                    } else {
                                        format!("{port_name}: {}", p.datatypes.join(", "))
                                    }
                                })
                                .unwrap_or_else(|| port_name.clone());
                            state.hover_node = None;
                            return Some(canvas::Action::publish(CanvasMessage::HoverChanged(
                                Some((type_text, cursor_position.x, cursor_position.y - 20.0)),
                            )));
                        }
                    }

                    // Track hover for two-zone node tooltip
                    let new_hover = hit_test_node(self.nodes, world_pos);
                    if new_hover != state.hover_node || new_hover.is_some() {
                        state.hover_node = new_hover;
                        let tooltip_data =
                            new_hover.and_then(|idx| self.nodes.get(idx)).and_then(|n| {
                                let bottom_center = transform_point(
                                    Point::new(n.x + n.width / 2.0, n.y + n.height),
                                    zoom,
                                    offset,
                                );
                                if is_in_source_text_zone(n, world_pos) {
                                    Some((n.source.clone(), bottom_center.x, bottom_center.y))
                                } else if !n.description.is_empty() {
                                    Some((n.description.clone(), bottom_center.x, bottom_center.y))
                                } else {
                                    None
                                }
                            });
                        return Some(canvas::Action::publish(CanvasMessage::HoverChanged(
                            tooltip_data,
                        )));
                    }
                    None
                }
            }

            // Left mouse button released — stop connecting, dragging, or resizing
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if let Some(connecting) = state.connecting.take() {
                    // Check if cursor is on a compatible port
                    if let Some((target_idx, target_port, target_is_output)) =
                        hit_test_port(self.nodes, cursor_position, zoom, offset)
                    {
                        // Must connect output→input or input→output
                        if connecting.from_output != target_is_output {
                            if let Some(target_node) = self.nodes.get(target_idx) {
                                // Check type compatibility before creating connection
                                let source_node =
                                    self.nodes.iter().find(|n| n.alias == connecting.from_node);
                                let types_ok = check_port_type_compatibility(
                                    source_node,
                                    &connecting.from_port,
                                    connecting.from_output,
                                    target_node,
                                    &target_port,
                                    target_is_output,
                                );

                                if types_ok {
                                    let (from_node, from_port, to_node, to_port) =
                                        if connecting.from_output {
                                            (
                                                connecting.from_node,
                                                connecting.from_port,
                                                target_node.alias.clone(),
                                                target_port,
                                            )
                                        } else {
                                            (
                                                target_node.alias.clone(),
                                                target_port,
                                                connecting.from_node,
                                                connecting.from_port,
                                            )
                                        };
                                    return Some(
                                        canvas::Action::publish(CanvasMessage::ConnectionCreated {
                                            from_node,
                                            from_port,
                                            to_node,
                                            to_port,
                                        })
                                        .and_capture(),
                                    );
                                }
                            }
                        }
                    }
                    // Released on empty area or incompatible port — cancel
                    return Some(canvas::Action::request_redraw().and_capture());
                }
                if let Some(resize) = state.resizing.take() {
                    // Emit resize completed with old and new geometry
                    if let Some(node) = self.nodes.get(resize.node_index) {
                        return Some(
                            canvas::Action::publish(CanvasMessage::ResizeCompleted(
                                resize.node_index,
                                resize.start_node_x,
                                resize.start_node_y,
                                resize.start_width,
                                resize.start_height,
                                node.x,
                                node.y,
                                node.width,
                                node.height,
                            ))
                            .and_capture(),
                        );
                    }
                    Some(canvas::Action::request_redraw().and_capture())
                } else if let Some(drag) = state.dragging.take() {
                    // Emit move completed with old and new position
                    if let Some(node) = self.nodes.get(drag.node_index) {
                        return Some(
                            canvas::Action::publish(CanvasMessage::MoveCompleted(
                                drag.node_index,
                                drag.start_x,
                                drag.start_y,
                                node.x,
                                node.y,
                            ))
                            .and_capture(),
                        );
                    }
                    Some(canvas::Action::request_redraw().and_capture())
                } else {
                    None
                }
            }

            // Middle mouse button released — stop panning
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Middle)) => {
                if state.panning.is_some() {
                    state.panning = None;
                    Some(canvas::Action::request_redraw().and_capture())
                } else {
                    None
                }
            }

            // Scroll wheel: pan or zoom depending on modifier keys
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let (dx, dy) = match *delta {
                    mouse::ScrollDelta::Lines { x, y } => (x * SCROLL_SPEED, y * SCROLL_SPEED),
                    mouse::ScrollDelta::Pixels { x, y } => (x, y),
                };

                if state.modifiers.command() {
                    // Zoom: positive dy = zoom in, negative = zoom out
                    if dy > 0.0 {
                        Some(
                            canvas::Action::publish(CanvasMessage::ZoomBy(ZOOM_STEP)).and_capture(),
                        )
                    } else if dy < 0.0 {
                        Some(
                            canvas::Action::publish(CanvasMessage::ZoomBy(1.0 / ZOOM_STEP))
                                .and_capture(),
                        )
                    } else {
                        None
                    }
                } else {
                    // Pan
                    let pan_dx = dx / zoom;
                    let pan_dy = dy / zoom;
                    Some(canvas::Action::publish(CanvasMessage::Pan(pan_dx, pan_dy)).and_capture())
                }
            }

            _ => None,
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let zoom = self.state.zoom;
        let offset = self.state.scroll_offset;

        // Draw the main cached content (edges, nodes, and flow I/O ports)
        let content = self.state.cache.draw(renderer, bounds.size(), |frame| {
            draw_nodes(frame, self.nodes, zoom, offset);
            draw_flow_io_ports(
                frame,
                self.flow_name,
                self.flow_inputs,
                self.flow_outputs,
                self.nodes,
                self.edges,
                self.is_subflow,
                state.selected_connection,
                zoom,
                offset,
            );
            draw_edges(
                frame,
                self.edges,
                self.nodes,
                zoom,
                offset,
                state.selected_connection,
            );
        });

        // Build an overlay for selection highlights, connection previews, tooltips, etc.
        // (Selected connections are drawn inline by draw_edges, not as an overlay)
        let needs_overlay = state.selected_node.is_some()
            || state.connecting.is_some()
            || state.hover_node.is_some();

        if needs_overlay {
            let mut overlay = Frame::new(renderer, bounds.size());

            // Draw selected node highlight and resize handles
            if let Some(selected_idx) = state.selected_node {
                if let Some(node) = self.nodes.get(selected_idx) {
                    let screen_pos = transform_point(Point::new(node.x, node.y), zoom, offset);
                    let screen_size = Size::new(node.width * zoom, node.height * zoom);
                    let selection_color = Color::from_rgb(1.0, 0.85, 0.0);
                    let highlight = Path::new(|builder| {
                        rounded_rect(builder, screen_pos, screen_size, CORNER_RADIUS * zoom);
                    });
                    overlay.stroke(
                        &highlight,
                        Stroke::default()
                            .with_width(4.0)
                            .with_color(selection_color),
                    );

                    // Draw resize handles at the 8 positions
                    for (_handle, world_pt) in &node.resize_handle_positions() {
                        let sp = transform_point(*world_pt, zoom, offset);
                        let handle_rect = Path::rectangle(
                            Point::new(sp.x - RESIZE_HANDLE_HALF, sp.y - RESIZE_HANDLE_HALF),
                            Size::new(RESIZE_HANDLE_HALF * 2.0, RESIZE_HANDLE_HALF * 2.0),
                        );
                        overlay.fill(&handle_rect, selection_color);
                        overlay.stroke(
                            &handle_rect,
                            Stroke::default()
                                .with_width(1.0)
                                .with_color(Color::from_rgb(0.3, 0.3, 0.0)),
                        );
                    }
                }
            }

            // Draw connection preview (bezier from start port to cursor)
            if let Some(ref connecting) = state.connecting {
                let start_screen = transform_point(connecting.start_pos, zoom, offset);
                let end_screen = connecting.current_screen_pos;

                let preview_color = Color::from_rgb(0.3, 0.9, 0.3);
                let dx_ctrl = (end_screen.x - start_screen.x).abs().max(60.0 * zoom) * 0.5;

                // Direction of control points depends on whether we started from output or input
                let (ctrl1, ctrl2) = if connecting.from_output {
                    (
                        Point::new(start_screen.x + dx_ctrl, start_screen.y),
                        Point::new(end_screen.x - dx_ctrl, end_screen.y),
                    )
                } else {
                    (
                        Point::new(start_screen.x - dx_ctrl, start_screen.y),
                        Point::new(end_screen.x + dx_ctrl, end_screen.y),
                    )
                };

                let preview_path = Path::new(|builder| {
                    builder.move_to(start_screen);
                    builder.bezier_curve_to(ctrl1, ctrl2, end_screen);
                });
                overlay.stroke(
                    &preview_path,
                    Stroke::default()
                        .with_width(2.0 * zoom)
                        .with_color(preview_color),
                );

                // Highlight the target port if hovering over a compatible one
                if let Some((target_idx, target_port, target_is_output)) =
                    hit_test_port(self.nodes, end_screen, zoom, offset)
                {
                    if connecting.from_output != target_is_output {
                        if let Some(target_node) = self.nodes.get(target_idx) {
                            let port_world = if target_is_output {
                                let pidx = target_node
                                    .outputs
                                    .iter()
                                    .position(|p| p.name == target_port)
                                    .unwrap_or(0);
                                target_node.output_port_position(pidx)
                            } else {
                                let pidx = target_node
                                    .inputs
                                    .iter()
                                    .position(|p| p.name == target_port)
                                    .unwrap_or(0);
                                target_node.input_port_position(pidx)
                            };
                            let port_screen = transform_point(port_world, zoom, offset);
                            let highlight_circle = Path::circle(port_screen, PORT_HIT_RADIUS);
                            overlay.stroke(
                                &highlight_circle,
                                Stroke::default().with_width(2.0).with_color(preview_color),
                            );
                        }
                    }
                }
            }

            return vec![content, overlay.into_geometry()];
        }

        vec![content]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.panning.is_some() {
            return mouse::Interaction::Grabbing;
        }

        if let Some(ref resize) = state.resizing {
            return resize_cursor(&resize.handle);
        }

        if state.connecting.is_some() {
            return mouse::Interaction::Crosshair;
        }

        if state.dragging.is_some() {
            return mouse::Interaction::Grabbing;
        }

        if let Some(pos) = cursor.position_in(bounds) {
            // Check resize handles on the selected node first
            if let Some(sel_idx) = state.selected_node {
                if let Some(sel_node) = self.nodes.get(sel_idx) {
                    if let Some((_idx, handle)) = hit_test_resize_handle(
                        sel_node,
                        sel_idx,
                        pos,
                        self.state.zoom,
                        self.state.scroll_offset,
                    ) {
                        return resize_cursor(&handle);
                    }
                }
            }

            // Check if hovering over a port
            if hit_test_port(self.nodes, pos, self.state.zoom, self.state.scroll_offset).is_some() {
                return mouse::Interaction::Crosshair;
            }

            let world_pos = screen_to_world(pos, self.state.zoom, self.state.scroll_offset);

            if hit_test_open_icon(self.nodes, world_pos).is_some() {
                return mouse::Interaction::Pointer;
            }

            if hit_test_node(self.nodes, world_pos).is_some() {
                return mouse::Interaction::Grab;
            }
        }

        mouse::Interaction::default()
    }
}

/// Draw all connection edges as bezier curves.
fn draw_edges(
    frame: &mut Frame,
    edges: &[EdgeLayout],
    nodes: &[NodeLayout],
    zoom: f32,
    offset: Point,
    selected: Option<usize>,
) {
    // Build a lookup from alias to node
    let node_map: HashMap<&str, &NodeLayout> =
        nodes.iter().map(|n| (n.alias.as_str(), n)).collect();

    // Draw selected connection last so it renders on top of crossing connections
    let draw_order: Vec<usize> = (0..edges.len())
        .filter(|i| selected != Some(*i))
        .chain(selected.filter(|i| *i < edges.len()))
        .collect();

    for edge_idx in draw_order {
        let edge = if let Some(e) = edges.get(edge_idx) {
            e
        } else {
            continue;
        };
        let from_node = node_map.get(edge.from_node.as_str());
        let to_node = node_map.get(edge.to_node.as_str());

        if let (Some(from), Some(to)) = (from_node, to_node) {
            // Find port positions (in world space)
            let from_point = if edge.from_port.is_empty() {
                from.output_port_position(0)
            } else {
                let base = base_port_name(&edge.from_port);
                let port_idx = from
                    .outputs
                    .iter()
                    .position(|p| p.name == base)
                    .unwrap_or(0);
                from.output_port_position(port_idx)
            };

            let to_point = if edge.to_port.is_empty() {
                to.input_port_position(0)
            } else {
                let base = base_port_name(&edge.to_port);
                let port_idx = to.inputs.iter().position(|p| p.name == base).unwrap_or(0);
                to.input_port_position(port_idx)
            };

            let is_self_connection = edge.from_node == edge.to_node;
            let node_bounds = if is_self_connection {
                Some((from.x, from.y, from.width, from.height))
            } else {
                None
            };
            let is_selected = selected == Some(edge_idx);
            draw_bezier_connection(
                frame,
                from_point,
                to_point,
                zoom,
                offset,
                node_bounds,
                is_selected,
            );

            // Draw connection name along the path if present
            if !edge.name.is_empty() {
                let from_s = transform_point(from_point, zoom, offset);
                let to_s = transform_point(to_point, zoom, offset);
                let mid = if is_self_connection {
                    // For loopback: place label at the bottom of the curve, outside the box
                    let (_, box_bottom, box_left, mid_x) =
                        loopback_waypoints(from.x, from.y, from.width, from.height, zoom, offset);
                    let _ = box_left;
                    Point::new(mid_x, box_bottom)
                } else {
                    // For normal connections: midpoint, above the line
                    Point::new((from_s.x + to_s.x) / 2.0, (from_s.y + to_s.y) / 2.0)
                };
                let name_label = CanvasText {
                    content: edge.name.clone(),
                    position: mid,
                    color: Color::from_rgb(0.7, 0.7, 0.7),
                    size: (PORT_FONT_SIZE * zoom).into(),
                    align_x: iced::alignment::Horizontal::Center.into(),
                    align_y: iced::alignment::Vertical::Bottom,
                    ..CanvasText::default()
                };
                frame.fill_text(name_label);
            }
        }
    }
}

/// Compute the key waypoints for a loopback (self-connection) path in screen space.
///
/// Returns `(box_right, box_bottom, box_left, mid_x)` — the screen-space coordinates
/// for routing around the node.
fn loopback_waypoints(
    nx: f32,
    ny: f32,
    nw: f32,
    nh: f32,
    zoom: f32,
    offset: Point,
) -> (f32, f32, f32, f32) {
    let margin = 25.0 * zoom;
    let box_right = (nx + nw + offset.x) * zoom + margin;
    let box_bottom = (ny + nh + offset.y) * zoom + margin;
    let box_left = (nx + offset.x) * zoom - margin;
    let mid_x = (box_right + box_left) / 2.0;
    (box_right, box_bottom, box_left, mid_x)
}

/// Draw a bezier curve connection between two world-space points, applying zoom and offset.
/// `node_bounds` is `Some((x, y, width, height))` in world coords for self-connections,
/// `None` for normal connections.
fn draw_bezier_connection(
    frame: &mut Frame,
    from: Point,
    to: Point,
    zoom: f32,
    offset: Point,
    node_bounds: Option<(f32, f32, f32, f32)>,
    highlighted: bool,
) {
    let from_s = transform_point(from, zoom, offset);
    let to_s = transform_point(to, zoom, offset);

    let conn_color = if highlighted {
        Color::from_rgb(1.0, 0.85, 0.0)
    } else {
        Color::from_rgb(0.5, 0.5, 0.5)
    };
    let line_width = if highlighted { 4.0 } else { 2.0 };
    let stroke = Stroke::default()
        .with_width(line_width * zoom)
        .with_color(conn_color);

    if let Some((nx, ny, nw, nh)) = node_bounds {
        let (box_right, box_bottom, box_left, _mid_x) =
            loopback_waypoints(nx, ny, nw, nh, zoom, offset);

        let path = Path::new(|builder| {
            builder.move_to(from_s);
            // Go right past the box
            builder.line_to(Point::new(box_right, from_s.y));
            // Curve down to below the box
            builder.quadratic_curve_to(
                Point::new(box_right, box_bottom),
                Point::new((box_right + box_left) / 2.0, box_bottom),
            );
            // Curve up to left of the box
            builder.quadratic_curve_to(
                Point::new(box_left, box_bottom),
                Point::new(box_left, to_s.y),
            );
            // Arrive at input
            builder.line_to(to_s);
        });
        frame.stroke(&path, stroke);
    } else {
        // Normal connection: bezier curve from right to left
        let dx = (to_s.x - from_s.x).abs().max(60.0 * zoom) * 0.5;
        let control1 = Point::new(from_s.x + dx, from_s.y);
        let control2 = Point::new(to_s.x - dx, to_s.y);

        let path = Path::new(|builder| {
            builder.move_to(from_s);
            builder.bezier_curve_to(control1, control2, to_s);
        });
        frame.stroke(&path, stroke);
    }

    // Filled arrow head at destination — triangle butts against the port semi-circle
    let arrow_size = 6.0 * zoom;
    let arrow = Path::new(|builder| {
        builder.move_to(Point::new(to_s.x - arrow_size, to_s.y - arrow_size));
        builder.line_to(to_s);
        builder.line_to(Point::new(to_s.x - arrow_size, to_s.y + arrow_size));
        builder.close();
    });
    frame.fill(&arrow, conn_color);
}

/// Draw all nodes onto the given frame, applying zoom and offset.
fn draw_nodes(frame: &mut Frame, nodes: &[NodeLayout], zoom: f32, offset: Point) {
    for node in nodes {
        draw_node(frame, node, zoom, offset);
    }
}

/// Draw a rounded bounding box around all subprocess nodes with flow I/O
/// ports on the box edges and bezier connections to internal nodes.
#[allow(clippy::too_many_arguments)]
fn draw_flow_io_ports(
    frame: &mut Frame,
    flow_name: &str,
    flow_inputs: &[PortInfo],
    flow_outputs: &[PortInfo],
    nodes: &[NodeLayout],
    edges: &[EdgeLayout],
    is_subflow: bool,
    selected_connection: Option<usize>,
    zoom: f32,
    offset: Point,
) {
    if !is_subflow {
        return;
    }

    let port_radius = 6.0;
    let font_size = 13.0;
    let spacing = 28.0;
    let padding = 80.0;
    let corner = 16.0;

    let max_ports = flow_inputs.len().max(flow_outputs.len()).max(1) as f32;
    let default_h = max_ports * spacing + 60.0;
    let (min_x, max_x, min_y, max_y) = if nodes.is_empty() {
        (150.0, 350.0, 100.0, 100.0 + default_h)
    } else {
        (
            nodes.iter().map(|n| n.x).fold(f32::MAX, f32::min),
            nodes.iter().map(|n| n.x + n.width).fold(f32::MIN, f32::max),
            nodes.iter().map(|n| n.y).fold(f32::MAX, f32::min),
            nodes
                .iter()
                .map(|n| n.y + n.height)
                .fold(f32::MIN, f32::max),
        )
    };

    let box_x = min_x - padding;
    let box_y = min_y - padding;
    let box_w = (max_x - min_x) + 2.0 * padding;
    let box_h = (max_y - min_y) + 2.0 * padding;

    // Draw the rounded bounding box
    let top_left = transform_point(Point::new(box_x, box_y), zoom, offset);
    let size = Size::new(box_w * zoom, box_h * zoom);
    let border_path = Path::new(|builder| {
        rounded_rect(builder, top_left, size, corner * zoom);
    });
    frame.stroke(
        &border_path,
        Stroke::default()
            .with_width(2.0)
            .with_color(Color::from_rgba(0.6, 0.6, 0.6, 0.5)),
    );

    // Draw flow name at top center of the bounding box
    if !flow_name.is_empty() {
        let name_pos = transform_point(Point::new(box_x + box_w / 2.0, box_y + 8.0), zoom, offset);
        frame.fill_text(CanvasText {
            content: flow_name.to_string(),
            position: name_pos,
            color: Color::from_rgb(0.9, 0.6, 0.2),
            size: (16.0 * zoom).into(),
            align_x: iced::alignment::Horizontal::Center.into(),
            ..CanvasText::default()
        });
    }

    let center_y = (min_y + max_y) / 2.0;
    let input_color = Color::from_rgb(0.4, 0.8, 1.0);
    let output_color = Color::from_rgb(1.0, 0.6, 0.3);

    // Compute and draw flow input ports on the left edge
    let mut input_positions: HashMap<String, Point> = HashMap::new();
    let input_start_y = center_y - (flow_inputs.len() as f32 - 1.0) * spacing / 2.0;
    for (i, input) in flow_inputs.iter().enumerate() {
        let world_y = input_start_y + i as f32 * spacing;
        let world_pos = Point::new(box_x, world_y);
        input_positions.insert(input.name.clone(), world_pos);
        let screen_pos = transform_point(world_pos, zoom, offset);
        let scaled_r = port_radius * zoom;

        use std::f32::consts::PI;
        let semi = Path::new(|builder| {
            builder.arc(canvas::path::Arc {
                center: screen_pos,
                radius: scaled_r,
                start_angle: (-PI / 2.0).into(),
                end_angle: (PI / 2.0).into(),
            });
            builder.close();
        });
        frame.fill(&semi, input_color);

        let label_pos = Point::new(screen_pos.x - scaled_r - 4.0, screen_pos.y);
        frame.fill_text(CanvasText {
            content: input.name.clone(),
            position: label_pos,
            color: input_color,
            size: (font_size * zoom).into(),
            align_x: iced::alignment::Horizontal::Right.into(),
            align_y: iced::alignment::Vertical::Center,
            ..CanvasText::default()
        });
    }

    // Compute and draw flow output ports on the right edge
    let mut output_positions: HashMap<String, Point> = HashMap::new();
    let right_x = box_x + box_w;
    let output_start_y = center_y - (flow_outputs.len() as f32 - 1.0) * spacing / 2.0;
    for (i, output) in flow_outputs.iter().enumerate() {
        let world_y = output_start_y + i as f32 * spacing;
        let world_pos = Point::new(right_x, world_y);
        output_positions.insert(output.name.clone(), world_pos);
        let screen_pos = transform_point(world_pos, zoom, offset);
        let scaled_r = port_radius * zoom;

        use std::f32::consts::PI;
        let semi = Path::new(|builder| {
            builder.arc(canvas::path::Arc {
                center: screen_pos,
                radius: scaled_r,
                start_angle: (PI / 2.0).into(),
                end_angle: (3.0 * PI / 2.0).into(),
            });
            builder.close();
        });
        frame.fill(&semi, output_color);

        let label_pos = Point::new(screen_pos.x + scaled_r + 4.0, screen_pos.y);
        frame.fill_text(CanvasText {
            content: output.name.clone(),
            position: label_pos,
            color: output_color,
            size: (font_size * zoom).into(),
            align_y: iced::alignment::Vertical::Center,
            ..CanvasText::default()
        });
    }

    // Draw bezier connections from flow inputs/outputs to internal node ports
    let conn_color = Color::from_rgba(0.7, 0.7, 0.7, 0.6);
    let sel_color = Color::from_rgb(1.0, 0.85, 0.0);
    for (edge_idx, edge) in edges.iter().enumerate() {
        let is_selected = selected_connection == Some(edge_idx);
        let color = if is_selected { sel_color } else { conn_color };
        let width = if is_selected { 3.0 } else { 1.5 };
        if edge.from_node == "input" {
            let input_name = base_port_name(&edge.from_port);
            if let Some(&from_world) = input_positions.get(input_name) {
                if let Some(to_world) = find_node_input_pos(nodes, &edge.to_node, &edge.to_port) {
                    draw_flow_io_bezier(frame, from_world, to_world, zoom, offset, color, width);
                }
            }
        }
        if edge.to_node == "output" {
            let output_name = base_port_name(&edge.to_port);
            if let Some(&to_world) = output_positions.get(output_name) {
                if let Some(from_world) =
                    find_node_output_pos(nodes, &edge.from_node, &edge.from_port)
                {
                    draw_flow_io_bezier(frame, from_world, to_world, zoom, offset, color, width);
                }
            }
        }
    }
}

fn find_node_input_pos(nodes: &[NodeLayout], alias: &str, port: &str) -> Option<Point> {
    let node = nodes.iter().find(|n| n.alias == alias)?;
    let base = base_port_name(port);
    let port_idx = node.inputs.iter().position(|p| p.name == base).unwrap_or(0);
    Some(node.input_port_position(port_idx))
}

fn find_node_output_pos(nodes: &[NodeLayout], alias: &str, port: &str) -> Option<Point> {
    let node = nodes.iter().find(|n| n.alias == alias)?;
    if port.is_empty() {
        Some(node.output_port_position(0))
    } else {
        let base = base_port_name(port);
        let port_idx = node
            .outputs
            .iter()
            .position(|p| p.name == base)
            .unwrap_or(0);
        Some(node.output_port_position(port_idx))
    }
}

fn draw_flow_io_bezier(
    frame: &mut Frame,
    from: Point,
    to: Point,
    zoom: f32,
    offset: Point,
    color: Color,
    stroke_width: f32,
) {
    let from_s = transform_point(from, zoom, offset);
    let to_s = transform_point(to, zoom, offset);
    let dx = (to_s.x - from_s.x).abs().max(40.0 * zoom) * 0.4;
    let path = Path::new(|builder| {
        builder.move_to(from_s);
        builder.bezier_curve_to(
            Point::new(from_s.x + dx, from_s.y),
            Point::new(to_s.x - dx, to_s.y),
            to_s,
        );
    });
    frame.stroke(
        &path,
        Stroke::default().with_width(stroke_width).with_color(color),
    );
}

/// Draw a single node as a rounded rectangle with title, source, and ports.
fn draw_node(frame: &mut Frame, node: &NodeLayout, zoom: f32, offset: Point) {
    let top_left = transform_point(Point::new(node.x, node.y), zoom, offset);
    let size = Size::new(node.width * zoom, node.height * zoom);
    let fill_color = node.fill_color();

    // Draw filled rounded rectangle
    let rect = Path::new(|builder| {
        rounded_rect(builder, top_left, size, CORNER_RADIUS * zoom);
    });
    frame.fill(&rect, fill_color);

    // No border when unselected — selection overlay draws the highlight border.
    // This avoids the border obscuring arrow heads arriving at ports.

    // Draw alias title centered near top of node
    let title_pos = transform_point(
        Point::new(node.x + node.width / 2.0, node.y + 12.0),
        zoom,
        offset,
    );
    let title = CanvasText {
        content: node.alias.clone(),
        position: title_pos,
        color: Color::WHITE,
        size: (TITLE_FONT_SIZE * zoom).into(),
        align_x: iced::alignment::Horizontal::Center.into(),
        align_y: iced::alignment::Vertical::Top,
        ..CanvasText::default()
    };
    frame.fill_text(title);

    // Draw source label below title (truncated with ellipsis)
    let source_display = truncate_source(&node.source, MAX_SOURCE_CHARS);
    let source_pos = transform_point(
        Point::new(node.x + node.width / 2.0, node.y + 34.0),
        zoom,
        offset,
    );
    let source_label = CanvasText {
        content: source_display,
        position: source_pos,
        color: Color::from_rgba(1.0, 1.0, 1.0, 0.7),
        size: (SOURCE_FONT_SIZE * zoom).into(),
        align_x: iced::alignment::Horizontal::Center.into(),
        align_y: iced::alignment::Vertical::Top,
        ..CanvasText::default()
    };
    frame.fill_text(source_label);

    // Draw open icon for sub-flows and provided implementations
    if node.is_openable() {
        let icon_size = 26.0 * zoom;
        let icon_x = node.x + node.width - 22.0;
        let icon_y = node.y + 4.0;
        let icon_pos = transform_point(Point::new(icon_x, icon_y), zoom, offset);

        let icon_text = CanvasText {
            content: "\u{270E}".to_string(), // ✎ pencil
            position: icon_pos,
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.8),
            size: icon_size.into(),
            ..CanvasText::default()
        };
        frame.fill_text(icon_text);
    }

    // Draw input ports on the left edge
    for (i, input_port) in node.inputs.iter().enumerate() {
        let port_pos = node.input_port_position(i);
        let init_label = node.initializers.get(&input_port.name).map(String::as_str);
        draw_port(
            frame,
            port_pos,
            &input_port.name,
            true,
            init_label,
            zoom,
            offset,
        );
    }

    // Draw output ports on the right edge
    for (i, output_port) in node.outputs.iter().enumerate() {
        let port_pos = node.output_port_position(i);
        draw_port(
            frame,
            port_pos,
            &output_port.name,
            false,
            None,
            zoom,
            offset,
        );
    }
}

/// Draw a port as a semi-circle on the edge of the node with a label and optional initializer.
///
/// Input ports: semi-circle on the left edge, flat side against the box, curved side facing left.
/// Output ports: semi-circle on the right edge, flat side against the box, curved side facing right.
/// The `center` parameter is in world coordinates; zoom and offset are applied internally.
fn draw_port(
    frame: &mut Frame,
    center: Point,
    name: &str,
    is_input: bool,
    initializer: Option<&str>,
    zoom: f32,
    offset: Point,
) {
    let screen_center = transform_point(center, zoom, offset);
    let scaled_radius = PORT_RADIUS * zoom;

    let has_init = initializer.is_some();
    let fill_color = if has_init {
        Color::from_rgb(1.0, 0.9, 0.3)
    } else {
        Color::WHITE
    };

    // Draw semi-circle: curved side faces inside the box, flat edge on the box boundary
    use std::f32::consts::PI;
    let semi = Path::new(|builder| {
        let (start_angle, end_angle) = if is_input {
            (-PI / 2.0, PI / 2.0) // Right-facing (inside the box)
        } else {
            (PI / 2.0, 3.0 * PI / 2.0) // Left-facing (inside the box)
        };
        builder.arc(canvas::path::Arc {
            center: screen_center,
            radius: scaled_radius,
            start_angle: start_angle.into(),
            end_angle: end_angle.into(),
        });
        builder.close();
    });
    frame.fill(&semi, fill_color);

    // Port name label (inside the node) — skip if port is unnamed
    if name.is_empty() {
        // Still draw initializer if present
        if let Some(init_text) = initializer {
            let init_label = CanvasText {
                content: init_text.to_string(),
                position: Point::new(
                    screen_center.x - 2.0 * zoom,
                    screen_center.y - scaled_radius - 2.0 * zoom,
                ),
                color: Color::from_rgb(0.9, 0.85, 0.2),
                size: (PORT_FONT_SIZE * zoom).into(),
                align_x: iced::alignment::Horizontal::Right.into(),
                align_y: iced::alignment::Vertical::Bottom,
                ..CanvasText::default()
            };
            frame.fill_text(init_label);
        }
        return;
    }

    let (label_x, align) = if is_input {
        (
            screen_center.x + scaled_radius + 4.0 * zoom,
            iced::alignment::Horizontal::Left,
        )
    } else {
        (
            screen_center.x - scaled_radius - 4.0 * zoom,
            iced::alignment::Horizontal::Right,
        )
    };

    let label = CanvasText {
        content: name.to_string(),
        position: Point::new(label_x, screen_center.y),
        color: Color::WHITE,
        size: (PORT_FONT_SIZE * zoom).into(),
        align_x: align.into(),
        align_y: iced::alignment::Vertical::Center,
        ..CanvasText::default()
    };
    frame.fill_text(label);

    // Initializer value label (outside the node, above-left of input port)
    if let Some(init_text) = initializer {
        let init_label = CanvasText {
            content: init_text.to_string(),
            position: Point::new(
                screen_center.x - 2.0 * zoom,
                screen_center.y - scaled_radius - 2.0 * zoom,
            ),
            color: Color::from_rgb(0.9, 0.85, 0.2),
            size: (PORT_FONT_SIZE * zoom).into(),
            align_x: iced::alignment::Horizontal::Right.into(),
            align_y: iced::alignment::Vertical::Bottom,
            ..CanvasText::default()
        };
        frame.fill_text(init_label);
    }
}

/// Build a rounded rectangle path using quadratic bezier curves at corners.
fn rounded_rect(builder: &mut canvas::path::Builder, top_left: Point, size: Size, radius: f32) {
    let r = radius.min(size.width / 2.0).min(size.height / 2.0);
    let x = top_left.x;
    let y = top_left.y;
    let w = size.width;
    let h = size.height;

    builder.move_to(Point::new(x + r, y));
    builder.line_to(Point::new(x + w - r, y));
    builder.quadratic_curve_to(Point::new(x + w, y), Point::new(x + w, y + r));
    builder.line_to(Point::new(x + w, y + h - r));
    builder.quadratic_curve_to(Point::new(x + w, y + h), Point::new(x + w - r, y + h));
    builder.line_to(Point::new(x + r, y + h));
    builder.quadratic_curve_to(Point::new(x, y + h), Point::new(x, y + h - r));
    builder.line_to(Point::new(x, y + r));
    builder.quadratic_curve_to(Point::new(x, y), Point::new(x + r, y));
    builder.close();
}

/// Truncate a source string to fit within the node, adding an ellipsis if needed.
fn truncate_source(source: &str, max_len: usize) -> String {
    if source.len() <= max_len {
        source.to_string()
    } else {
        let end = source
            .char_indices()
            .nth(max_len.saturating_sub(3))
            .map_or(source.len(), |(i, _)| i);
        let mut truncated = source.get(..end).unwrap_or(source).to_string();
        truncated.push_str("...");
        truncated
    }
}

/// Check if the types of two ports are compatible for a connection.
///
/// Returns true if:
/// - Either port has no type info (unknown types are assumed compatible)
/// - At least one type from the source port matches a type on the destination port
fn check_port_type_compatibility(
    source_node: Option<&NodeLayout>,
    source_port: &str,
    source_is_output: bool,
    target_node: &NodeLayout,
    target_port: &str,
    target_is_output: bool,
) -> bool {
    let source_types = source_node.and_then(|n| {
        let ports = if source_is_output {
            &n.outputs
        } else {
            &n.inputs
        };
        ports.iter().find(|p| p.name == source_port)
    });

    let target_types = {
        let ports = if target_is_output {
            &target_node.outputs
        } else {
            &target_node.inputs
        };
        ports.iter().find(|p| p.name == target_port)
    };

    match (source_types, target_types) {
        (Some(src), Some(tgt)) => {
            log::info!(
                "Type check: src port '{}' types {:?} → tgt port '{}' types {:?}",
                src.name,
                src.datatypes,
                tgt.name,
                tgt.datatypes
            );
            // If either has no type info (empty list or only empty strings),
            // allow the connection — untyped ports accept anything
            let src_untyped =
                src.datatypes.is_empty() || src.datatypes.iter().all(|t| t.is_empty());
            let tgt_untyped =
                tgt.datatypes.is_empty() || tgt.datatypes.iter().all(|t| t.is_empty());
            if src_untyped || tgt_untyped {
                return true;
            }
            // Check for at least one matching type
            src.datatypes
                .iter()
                .any(|st| tgt.datatypes.iter().any(|tt| st == tt))
        }
        // Unknown port or no type info — allow
        (src, tgt) => {
            log::info!(
                "Type check: src={}, tgt={} — allowing (unknown port)",
                src.is_some(),
                tgt.is_some()
            );
            true
        }
    }
}

/// Build the complete canvas area for a flow-editor window, including the
/// interactive canvas, zoom controls, tooltip overlay, initializer editor
/// dialog, and right-click context menu.
pub(crate) fn view_canvas_area<'a>(
    win: &'a WindowState,
    window_id: window::Id,
) -> Element<'a, Message> {
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

        let dialog =
            container(
                container(dialog_col)
                    .width(280)
                    .style(|_theme: &Theme| container::Style {
                        background: Some(iced::Background::Color(Color::from_rgb(
                            0.15, 0.15, 0.15,
                        ))),
                        border: iced::Border {
                            color: Color::from_rgb(0.4, 0.4, 0.4),
                            width: 1.0,
                            radius: 8.0.into(),
                        },
                        ..Default::default()
                    }),
            )
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

        let positioned = container(menu).padding(iced::Padding {
            top: cy,
            left: cx,
            right: 0.0,
            bottom: 0.0,
        });
        canvas_stack.push(positioned.into());
    }

    stack(canvas_stack).into()
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod test {
    use super::*;
    use iced::Point;

    #[test]
    fn split_route_with_port() {
        let (node, port) = split_route("sequence/number");
        assert_eq!(node, "sequence");
        assert_eq!(port, "number");
    }

    #[test]
    fn split_route_no_port() {
        let (node, port) = split_route("add1");
        assert_eq!(node, "add1");
        assert_eq!(port, "");
    }

    #[test]
    fn split_route_leading_slash() {
        let (node, port) = split_route("/sequence/number");
        assert_eq!(node, "sequence");
        assert_eq!(port, "number");
    }

    #[test]
    fn derive_short_name_lib() {
        assert_eq!(
            derive_short_name("lib://flowstdlib/math/sequence"),
            "sequence"
        );
    }

    #[test]
    fn derive_short_name_context() {
        assert_eq!(derive_short_name("context://stdio/stdout"), "stdout");
    }

    #[test]
    fn derive_short_name_simple() {
        assert_eq!(derive_short_name("add"), "add");
    }

    #[test]
    fn format_value_string() {
        assert_eq!(format_value(&serde_json::json!("hello")), "\"hello\"");
    }

    #[test]
    fn format_value_number() {
        assert_eq!(format_value(&serde_json::json!(42)), "42");
    }

    #[test]
    fn format_value_bool() {
        assert_eq!(format_value(&serde_json::json!(true)), "true");
    }

    #[test]
    fn format_value_null() {
        assert_eq!(format_value(&serde_json::json!(null)), "null");
    }

    #[test]
    fn format_value_small_array() {
        assert_eq!(format_value(&serde_json::json!([1, 2, 3])), "[1,2,3]");
    }

    #[test]
    fn format_value_large_array() {
        assert_eq!(format_value(&serde_json::json!([1, 2, 3, 4])), "[4...]");
    }

    #[test]
    fn format_value_object() {
        assert_eq!(format_value(&serde_json::json!({"a": 1})), "{...}");
    }

    #[test]
    fn truncate_source_short() {
        assert_eq!(truncate_source("short", 10), "short");
    }

    #[test]
    fn truncate_source_long() {
        let result = truncate_source("this is a very long source string", 15);
        assert!(result.ends_with("..."));
        assert!(result.len() <= 15);
    }

    #[test]
    fn transform_point_identity() {
        let p = transform_point(Point::new(10.0, 20.0), 1.0, Point::new(0.0, 0.0));
        assert!((p.x - 10.0).abs() < 0.01);
        assert!((p.y - 20.0).abs() < 0.01);
    }

    #[test]
    fn transform_point_with_zoom() {
        let p = transform_point(Point::new(10.0, 20.0), 2.0, Point::new(0.0, 0.0));
        assert!((p.x - 20.0).abs() < 0.01);
        assert!((p.y - 40.0).abs() < 0.01);
    }

    #[test]
    fn transform_point_with_offset() {
        let p = transform_point(Point::new(10.0, 20.0), 1.0, Point::new(5.0, 10.0));
        assert!((p.x - 15.0).abs() < 0.01);
        assert!((p.y - 30.0).abs() < 0.01);
    }

    #[test]
    fn screen_to_world_roundtrip() {
        let zoom = 1.5;
        let offset = Point::new(10.0, 20.0);
        let world = Point::new(100.0, 200.0);
        let screen = transform_point(world, zoom, offset);
        let back = screen_to_world(screen, zoom, offset);
        assert!((back.x - world.x).abs() < 0.01);
        assert!((back.y - world.y).abs() < 0.01);
    }

    #[test]
    fn hit_test_node_inside() {
        let nodes = vec![NodeLayout {
            alias: "test".into(),
            source: "lib://test".into(),
            ..Default::default()
        }];
        assert_eq!(hit_test_node(&nodes, Point::new(150.0, 150.0)), Some(0));
    }

    #[test]
    fn hit_test_node_outside() {
        let nodes = vec![NodeLayout {
            alias: "test".into(),
            source: "lib://test".into(),
            ..Default::default()
        }];
        assert_eq!(hit_test_node(&nodes, Point::new(50.0, 50.0)), None);
    }

    #[test]
    fn hit_test_source_text_zone() {
        let node = NodeLayout {
            alias: "test".into(),
            source: "lib://flowstdlib/math/add".into(),
            ..Default::default()
        };
        // Source text is centered at (node.x + width/2, node.y + 34.0)
        let source_center = Point::new(190.0, 134.0);
        assert!(is_in_source_text_zone(&node, source_center));
        // Point clearly outside source text zone but inside node
        let node_body = Point::new(110.0, 200.0);
        assert!(!is_in_source_text_zone(&node, node_body));
    }

    #[test]
    fn build_edge_layouts_single() {
        use flowcore::model::connection::Connection;
        let conn = Connection::new("sequence/number", "add1/i1");
        let edges = build_edge_layouts(&[conn]);
        assert_eq!(edges.len(), 1);
        assert_eq!(
            edges.first().map(|e| e.from_node.as_str()),
            Some("sequence")
        );
        assert_eq!(edges.first().map(|e| e.from_port.as_str()), Some("number"));
        assert_eq!(edges.first().map(|e| e.to_node.as_str()), Some("add1"));
        assert_eq!(edges.first().map(|e| e.to_port.as_str()), Some("i1"));
    }

    #[test]
    fn edge_references_node() {
        let edge = EdgeLayout {
            from_node: "a".into(),
            from_port: "out".into(),
            to_node: "b".into(),
            to_port: "in".into(),
            name: String::new(),
        };
        assert!(edge.references_node("a"));
        assert!(edge.references_node("b"));
        assert!(!edge.references_node("c"));
    }

    #[test]
    fn node_layout_port_positions() {
        let node = NodeLayout {
            alias: "test".into(),
            source: "lib://test".into(),
            inputs: vec![
                PortInfo::from_name("i1".into()),
                PortInfo::from_name("i2".into()),
            ],
            outputs: vec![PortInfo::from_name("out".into())],
            ..Default::default()
        };
        let ip0 = node.input_port_position(0);
        let ip1 = node.input_port_position(1);
        let op0 = node.output_port_position(0);

        // Input ports on left edge
        assert!((ip0.x - 100.0).abs() < 0.01);
        assert!((ip1.x - 100.0).abs() < 0.01);
        // Output ports on right edge
        assert!((op0.x - 280.0).abs() < 0.01);
        // Ports vertically spaced
        assert!(ip1.y > ip0.y);
    }

    #[test]
    fn base_port_name_simple() {
        assert_eq!(base_port_name("string"), "string");
    }

    #[test]
    fn base_port_name_with_array_index() {
        assert_eq!(base_port_name("string/1"), "string");
    }

    #[test]
    fn base_port_name_with_deep_array_index() {
        assert_eq!(base_port_name("json/3"), "json");
    }

    #[test]
    fn base_port_name_no_index() {
        assert_eq!(base_port_name("array/number"), "array/number");
    }

    #[test]
    fn base_port_name_empty() {
        assert_eq!(base_port_name(""), "");
    }

    #[test]
    fn transform_and_inverse() {
        let p = Point::new(100.0, 200.0);
        let zoom = 2.0;
        let offset = Point::new(10.0, 20.0);
        let screen = transform_point(p, zoom, offset);
        let back = screen_to_world(screen, zoom, offset);
        assert!((back.x - p.x).abs() < 0.01);
        assert!((back.y - p.y).abs() < 0.01);
    }

    #[test]
    fn distance_to_segment_on_segment() {
        let a = Point::new(0.0, 0.0);
        let b = Point::new(10.0, 0.0);
        let p = Point::new(5.0, 0.0);
        assert!(distance_to_segment_sq(p, a, b) < 0.01);
    }

    #[test]
    fn distance_to_segment_perpendicular() {
        let a = Point::new(0.0, 0.0);
        let b = Point::new(10.0, 0.0);
        let p = Point::new(5.0, 3.0);
        assert!((distance_to_segment_sq(p, a, b) - 9.0).abs() < 0.01);
    }

    #[test]
    fn distance_to_segment_beyond_endpoint() {
        let a = Point::new(0.0, 0.0);
        let b = Point::new(10.0, 0.0);
        let p = Point::new(15.0, 0.0);
        assert!((distance_to_segment_sq(p, a, b) - 25.0).abs() < 0.01);
    }

    #[test]
    fn distance_to_segment_zero_length() {
        let a = Point::new(5.0, 5.0);
        let p = Point::new(8.0, 5.0);
        assert!((distance_to_segment_sq(p, a, a) - 9.0).abs() < 0.01);
    }

    #[test]
    fn cubic_bezier_endpoints() {
        let p0 = Point::new(0.0, 0.0);
        let p1 = Point::new(1.0, 2.0);
        let p2 = Point::new(3.0, 2.0);
        let p3 = Point::new(4.0, 0.0);
        let start = cubic_bezier(p0, p1, p2, p3, 0.0);
        let end = cubic_bezier(p0, p1, p2, p3, 1.0);
        assert!((start.x - p0.x).abs() < 0.01);
        assert!((start.y - p0.y).abs() < 0.01);
        assert!((end.x - p3.x).abs() < 0.01);
        assert!((end.y - p3.y).abs() < 0.01);
    }

    #[test]
    fn quadratic_bezier_endpoints() {
        let p0 = Point::new(0.0, 0.0);
        let p1 = Point::new(2.0, 4.0);
        let p2 = Point::new(4.0, 0.0);
        let start = quadratic_bezier_pt(p0, p1, p2, 0.0);
        let end = quadratic_bezier_pt(p0, p1, p2, 1.0);
        assert!((start.x - p0.x).abs() < 0.01);
        assert!((end.x - p2.x).abs() < 0.01);
    }

    #[test]
    fn hit_test_node_miss() {
        let node = NodeLayout {
            alias: "n".into(),
            source: "lib://test".into(),
            ..Default::default()
        };
        assert_eq!(
            hit_test_node(&[node.clone()], Point::new(150.0, 150.0)),
            Some(0)
        );
        assert_eq!(hit_test_node(&[node], Point::new(50.0, 50.0)), None);
    }

    #[test]
    fn hit_test_open_icon_only_openable() {
        let lib_node = NodeLayout {
            alias: "n".into(),
            source: "lib://test".into(),
            ..Default::default()
        };
        let local_node = NodeLayout {
            source: "subflow".into(),
            ..lib_node.clone()
        };
        // lib:// nodes are not openable
        assert_eq!(
            hit_test_open_icon(&[lib_node], Point::new(278.0, 104.0)),
            None
        );
        // Local nodes are openable
        assert!(hit_test_open_icon(&[local_node], Point::new(278.0, 104.0)).is_some());
    }

    #[test]
    fn is_openable_lib() {
        let node = NodeLayout {
            alias: "n".into(),
            source: "lib://flowstdlib/math/add".into(),
            x: 0.0,
            y: 0.0,
            ..Default::default()
        };
        assert!(!node.is_openable());
    }

    #[test]
    fn is_openable_context() {
        let node = NodeLayout {
            alias: "n".into(),
            source: "context://stdio/stdout".into(),
            x: 0.0,
            y: 0.0,
            ..Default::default()
        };
        assert!(!node.is_openable());
    }

    #[test]
    fn is_openable_local() {
        let node = NodeLayout {
            alias: "n".into(),
            source: "subflow/subflow".into(),
            x: 0.0,
            y: 0.0,
            ..Default::default()
        };
        assert!(node.is_openable());
    }

    #[test]
    fn truncate_source_under_limit() {
        assert_eq!(truncate_source("short", 22), "short");
    }

    #[test]
    fn truncate_source_with_ellipsis() {
        let long = "lib://flowstdlib/math/very_long_function_name";
        let result = truncate_source(long, 22);
        assert!(result.len() <= 25); // with ellipsis
        assert!(result.contains("..."));
    }

    #[test]
    fn check_type_compat_same_type() {
        let nodes = vec![
            NodeLayout {
                alias: "a".into(),
                x: 0.0,
                y: 0.0,
                outputs: vec![PortInfo {
                    name: "out".into(),
                    datatypes: vec!["number".into()],
                }],
                ..Default::default()
            },
            NodeLayout {
                alias: "b".into(),
                x: 0.0,
                y: 0.0,
                inputs: vec![PortInfo {
                    name: "in".into(),
                    datatypes: vec!["number".into()],
                }],
                ..Default::default()
            },
        ];
        assert!(check_port_type_compatibility(
            Some(&nodes[0]),
            "out",
            true,
            &nodes[1],
            "in",
            false
        ));
    }

    #[test]
    fn check_type_compat_different_type() {
        let nodes = vec![
            NodeLayout {
                alias: "a".into(),
                x: 0.0,
                y: 0.0,
                outputs: vec![PortInfo {
                    name: "out".into(),
                    datatypes: vec!["number".into()],
                }],
                ..Default::default()
            },
            NodeLayout {
                alias: "b".into(),
                x: 0.0,
                y: 0.0,
                inputs: vec![PortInfo {
                    name: "in".into(),
                    datatypes: vec!["string".into()],
                }],
                ..Default::default()
            },
        ];
        assert!(!check_port_type_compatibility(
            Some(&nodes[0]),
            "out",
            true,
            &nodes[1],
            "in",
            false
        ));
    }

    #[test]
    fn check_type_compat_untyped_allows_any() {
        let nodes = vec![
            NodeLayout {
                alias: "a".into(),
                x: 0.0,
                y: 0.0,
                outputs: vec![PortInfo {
                    name: "out".into(),
                    datatypes: vec![],
                }],
                ..Default::default()
            },
            NodeLayout {
                alias: "b".into(),
                x: 0.0,
                y: 0.0,
                inputs: vec![PortInfo {
                    name: "in".into(),
                    datatypes: vec!["string".into()],
                }],
                ..Default::default()
            },
        ];
        assert!(check_port_type_compatibility(
            Some(&nodes[0]),
            "out",
            true,
            &nodes[1],
            "in",
            false
        ));
    }

    #[test]
    fn compute_flow_io_positions_with_nodes() {
        let nodes = vec![NodeLayout {
            alias: "n".into(),
            ..Default::default()
        }];
        let inputs = vec![PortInfo {
            name: "data".into(),
            datatypes: vec![],
        }];
        let outputs = vec![PortInfo {
            name: "result".into(),
            datatypes: vec![],
        }];
        let (inp, outp) = compute_flow_io_positions(&nodes, &inputs, &outputs);
        assert!(inp.contains_key("data"));
        assert!(outp.contains_key("result"));
        // Input on the left of nodes
        assert!(inp["data"].x < 100.0);
        // Output on the right of nodes
        assert!(outp["result"].x > 280.0);
    }

    #[test]
    fn compute_flow_io_positions_empty_nodes() {
        let inputs = vec![PortInfo {
            name: "in".into(),
            datatypes: vec![],
        }];
        let outputs = vec![PortInfo {
            name: "out".into(),
            datatypes: vec![],
        }];
        let (inp, outp) = compute_flow_io_positions(&[], &inputs, &outputs);
        assert!(inp.contains_key("in"));
        assert!(outp.contains_key("out"));
    }

    #[test]
    fn compute_flow_io_positions_no_ports() {
        let (inp, outp) = compute_flow_io_positions(&[], &[], &[]);
        assert!(inp.is_empty());
        assert!(outp.is_empty());
    }

    #[test]
    fn find_node_output_inline_with_subroute() {
        let node = NodeLayout {
            alias: "get".into(),
            outputs: vec![
                PortInfo {
                    name: "string".into(),
                    datatypes: vec![],
                },
                PortInfo {
                    name: "json".into(),
                    datatypes: vec![],
                },
            ],
            ..Default::default()
        };
        let string_pos = find_node_output_pos_inline(&node, "string/1");
        let json_pos = find_node_output_pos_inline(&node, "json/2");
        // string is output 0, json is output 1 — different y positions
        assert!((json_pos.y - string_pos.y).abs() > 1.0);
    }

    #[test]
    fn fill_color_by_source() {
        let make = |source: &str| NodeLayout {
            alias: "n".into(),
            source: source.into(),
            x: 0.0,
            y: 0.0,
            ..Default::default()
        };
        let lib = make("lib://flowstdlib/math/add");
        let ctx = make("context://stdio/stdout");
        let rs = make("impl.rs");
        let flow = make("subflow");
        // Different sources should produce different colors
        assert_ne!(lib.fill_color(), ctx.fill_color());
        assert_ne!(lib.fill_color(), rs.fill_color());
        assert_ne!(lib.fill_color(), flow.fill_color());
    }
}
