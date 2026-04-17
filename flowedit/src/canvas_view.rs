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
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Size, Theme};

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

use flowcore::model::connection::Connection;
use flowcore::model::process_reference::ProcessReference;

/// Messages produced by the canvas interaction layer.
#[derive(Debug, Clone)]
pub(crate) enum CanvasMessage {
    /// A node was selected (or deselected if `None`).
    Selected(Option<usize>),
    /// A node was moved to a new position.
    Moved(usize, f32, f32),
    /// A node was resized (index, new_x, new_y, new_width, new_height).
    Resized(usize, f32, f32, f32, f32),
    /// A node should be deleted.
    Deleted(usize),
    /// Pan the canvas by a world-space delta.
    Pan(f32, f32),
    /// Zoom the canvas by a multiplicative factor.
    ZoomBy(f32),
    /// Auto-fit with the actual viewport size (triggered on initial load).
    AutoFitViewport(Size),
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
    /// Last known bounds size — used to detect window resize for auto-fit
    last_bounds: Option<Size>,
}

/// Tracks a middle-mouse-button pan in progress.
#[derive(Debug, Clone)]
struct PanState {
    /// Last screen-space cursor position during the pan
    last_screen_pos: Point,
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

/// A positioned node derived from a [`ProcessReference`], ready for rendering.
#[derive(Debug, Clone)]
pub(crate) struct NodeLayout {
    /// Display name (alias) for this node
    pub alias: String,
    /// Source path of the process
    source: String,
    /// X coordinate on the canvas
    pub x: f32,
    /// Y coordinate on the canvas
    pub y: f32,
    /// Width of the node rectangle
    pub width: f32,
    /// Height of the node rectangle
    pub height: f32,
    /// Input port names
    pub inputs: Vec<String>,
    /// Output port names
    pub outputs: Vec<String>,
    /// Initializer display strings keyed by port name (e.g., "start" → "1 once")
    pub initializers: HashMap<String, String>,
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
    from_node: String,
    /// Source port name (may be empty for whole-node output)
    from_port: String,
    /// Destination node alias
    to_node: String,
    /// Destination port name
    to_port: String,
}

/// Build a list of [`NodeLayout`] from process references and connections.
///
/// Ports are derived from two sources:
/// - **Initializations** on each `ProcessReference` tell us about inputs with initial values
/// - **Connections** tell us which additional input/output ports each node has
///
/// Layout uses the optional `x`, `y`, `width`, `height` fields from `ProcessReference`,
/// falling back to auto-grid positioning.
pub(crate) fn build_node_layouts(
    process_refs: &[ProcessReference],
    connections: &[Connection],
) -> Vec<NodeLayout> {
    // First pass: collect ports from connections
    let mut node_inputs: HashMap<String, Vec<String>> = HashMap::new();
    let mut node_outputs: HashMap<String, Vec<String>> = HashMap::new();

    for conn in connections {
        let from_route = conn.from().to_string();
        let (from_node, from_port) = split_route(&from_route);
        let port_name = if from_port.is_empty() {
            "output".to_string()
        } else {
            from_port
        };
        let outputs = node_outputs.entry(from_node).or_default();
        if !outputs.contains(&port_name) {
            outputs.push(port_name);
        }

        for to_route in conn.to() {
            let to_str = to_route.to_string();
            let (to_node, to_port) = split_route(&to_str);
            let port_name = if to_port.is_empty() {
                "default".to_string()
            } else {
                to_port
            };
            let inputs = node_inputs.entry(to_node).or_default();
            if !inputs.contains(&port_name) {
                inputs.push(port_name);
            }
        }
    }

    // Check if any process has saved layout positions
    let has_saved_layout = process_refs.iter().any(|p| p.x.is_some() || p.y.is_some());

    // If no saved layout, compute topology-based positions
    let topo_positions = if has_saved_layout {
        HashMap::new()
    } else {
        compute_topological_layout(process_refs, connections)
    };

    // Second pass: build node layouts
    let mut nodes = Vec::with_capacity(process_refs.len());

    for (i, pref) in process_refs.iter().enumerate() {
        // Derive alias: use explicit alias, or extract short name from source URL
        let alias = if pref.alias.is_empty() {
            derive_short_name(&pref.source)
        } else {
            pref.alias.to_string()
        };

        // Merge inputs from initializations and connections
        let mut inputs: Vec<String> = pref.initializations.keys().cloned().collect();
        if let Some(conn_inputs) = node_inputs.get(&alias) {
            for port in conn_inputs {
                if !inputs.contains(port) {
                    inputs.push(port.clone());
                }
            }
        }

        // Outputs come from connections only
        let outputs = node_outputs.get(&alias).cloned().unwrap_or_default();

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
                    format!("{} once", format_value(v))
                }
                flowcore::model::input::InputInitializer::Always(v) => {
                    format!("{} always", format_value(v))
                }
            };
            initializers.insert(port_name.clone(), display);
        }

        nodes.push(NodeLayout {
            alias: alias.clone(),
            source: pref.source.clone(),
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
fn derive_short_name(source: &str) -> String {
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
    pub(crate) fn view<'a>(
        &'a self,
        nodes: &'a [NodeLayout],
        edges: &'a [EdgeLayout],
        auto_fit_pending: bool,
        auto_fit_enabled: bool,
    ) -> Element<'a, CanvasMessage> {
        Canvas::new(FlowCanvas {
            state: self,
            nodes,
            edges,
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
    pub(crate) fn auto_fit(&mut self, nodes: &[NodeLayout], viewport: Size) {
        if nodes.is_empty() {
            self.zoom = 1.0;
            self.scroll_offset = Point::new(0.0, 0.0);
            self.cache.clear();
            return;
        }

        // Find bounding box of all nodes in world coordinates
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for node in nodes {
            if node.x < min_x {
                min_x = node.x;
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

        let content_width = max_x - min_x + AUTO_FIT_PADDING * 2.0;
        let content_height = max_y - min_y + AUTO_FIT_PADDING * 2.0;

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
        let bounds_changed = state.last_bounds.map_or(true, |last| {
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
                if let Some(sel_idx) = state.selected_node {
                    state.selected_node = None;
                    return Some(
                        canvas::Action::publish(CanvasMessage::Deleted(sel_idx)).and_capture(),
                    );
                }
                return None;
            }
            _ => {}
        }

        let cursor_position = cursor.position_in(bounds)?;
        let zoom = self.state.zoom;
        let offset = self.state.scroll_offset;
        let world_pos = screen_to_world(cursor_position, zoom, offset);

        match event {
            // Left mouse button pressed — check resize handles, then select/drag node, or deselect
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                // First, check if cursor is on a resize handle of the selected node
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

                if let Some(idx) = hit_test_node(self.nodes, world_pos) {
                    // Get the node; bail if index is out of range
                    let node = self.nodes.get(idx)?;
                    state.selected_node = Some(idx);
                    state.dragging = Some(DragState {
                        node_index: idx,
                        offset_x: world_pos.x - node.x,
                        offset_y: world_pos.y - node.y,
                    });
                    Some(canvas::Action::publish(CanvasMessage::Selected(Some(idx))).and_capture())
                } else {
                    // Clicked empty canvas — deselect
                    state.selected_node = None;
                    state.dragging = None;
                    Some(canvas::Action::publish(CanvasMessage::Selected(None)).and_capture())
                }
            }

            // Middle mouse button pressed — start panning
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)) => {
                state.panning = Some(PanState {
                    last_screen_pos: cursor_position,
                });
                Some(canvas::Action::request_redraw().and_capture())
            }

            // Mouse moved — handle resize, drag, or pan
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
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
                    None
                }
            }

            // Left mouse button released — stop dragging or resizing
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.resizing.is_some() {
                    state.resizing = None;
                    Some(canvas::Action::request_redraw().and_capture())
                } else if state.dragging.is_some() {
                    state.dragging = None;
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

        // Draw the main cached content (edges and nodes) with zoom/scroll transform
        let content = self.state.cache.draw(renderer, bounds.size(), |frame| {
            draw_edges(frame, self.edges, self.nodes, zoom, offset);
            draw_nodes(frame, self.nodes, zoom, offset);
        });

        // Draw selection highlight and resize handles as an overlay (not cached, so it updates instantly)
        if let Some(selected_idx) = state.selected_node {
            if let Some(node) = self.nodes.get(selected_idx) {
                let mut overlay = Frame::new(renderer, bounds.size());
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

                return vec![content, overlay.into_geometry()];
            }
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

            let world_pos = screen_to_world(pos, self.state.zoom, self.state.scroll_offset);
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
) {
    // Build a lookup from alias to node
    let node_map: HashMap<&str, &NodeLayout> =
        nodes.iter().map(|n| (n.alias.as_str(), n)).collect();

    for edge in edges {
        let from_node = node_map.get(edge.from_node.as_str());
        let to_node = node_map.get(edge.to_node.as_str());

        if let (Some(from), Some(to)) = (from_node, to_node) {
            // Find port positions (in world space)
            let from_point = if edge.from_port.is_empty() {
                from.output_port_position(0)
            } else {
                let port_idx = from
                    .outputs
                    .iter()
                    .position(|p| p == &edge.from_port)
                    .unwrap_or(0);
                from.output_port_position(port_idx)
            };

            let to_point = if edge.to_port.is_empty() {
                to.input_port_position(0)
            } else {
                // Find the input port index by name
                let port_idx = to
                    .inputs
                    .iter()
                    .position(|p| p == &edge.to_port)
                    .unwrap_or(0);
                to.input_port_position(port_idx)
            };

            draw_bezier_connection(frame, from_point, to_point, zoom, offset);
        }
    }
}

/// Draw a bezier curve connection between two world-space points, applying zoom and offset.
fn draw_bezier_connection(frame: &mut Frame, from: Point, to: Point, zoom: f32, offset: Point) {
    let from_s = transform_point(from, zoom, offset);
    let to_s = transform_point(to, zoom, offset);

    // Detect loopback (from and to are very close or same node) and use wider arc
    let dist = ((to_s.x - from_s.x).powi(2) + (to_s.y - from_s.y).powi(2)).sqrt();
    let is_loopback = dist < 50.0 * zoom;

    let (control1, control2) = if is_loopback {
        // Wide arc that goes below/right and loops back
        let loop_radius = 80.0 * zoom;
        (
            Point::new(from_s.x + loop_radius, from_s.y + loop_radius),
            Point::new(to_s.x - loop_radius, to_s.y + loop_radius),
        )
    } else {
        let dx = (to_s.x - from_s.x).abs() * 0.5;
        (
            Point::new(from_s.x + dx, from_s.y),
            Point::new(to_s.x - dx, to_s.y),
        )
    };

    let path = Path::new(|builder| {
        builder.move_to(from_s);
        builder.bezier_curve_to(control1, control2, to_s);
    });

    frame.stroke(
        &path,
        Stroke::default()
            .with_width(2.0 * zoom)
            .with_color(Color::from_rgb(0.5, 0.5, 0.5)),
    );

    // Draw a small arrow head at the destination
    let arrow_size = 6.0 * zoom;
    let arrow = Path::new(|builder| {
        builder.move_to(Point::new(to_s.x - arrow_size, to_s.y - arrow_size));
        builder.line_to(to_s);
        builder.line_to(Point::new(to_s.x - arrow_size, to_s.y + arrow_size));
    });
    frame.stroke(
        &arrow,
        Stroke::default()
            .with_width(2.0 * zoom)
            .with_color(Color::from_rgb(0.5, 0.5, 0.5)),
    );
}

/// Draw all nodes onto the given frame, applying zoom and offset.
fn draw_nodes(frame: &mut Frame, nodes: &[NodeLayout], zoom: f32, offset: Point) {
    for node in nodes {
        draw_node(frame, node, zoom, offset);
    }
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

    // Draw border
    let border = Path::new(|builder| {
        rounded_rect(builder, top_left, size, CORNER_RADIUS * zoom);
    });
    frame.stroke(
        &border,
        Stroke::default()
            .with_width(2.0 * zoom)
            .with_color(Color::from_rgb(0.2, 0.2, 0.2)),
    );

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

    // Draw input ports on the left edge
    for (i, input_name) in node.inputs.iter().enumerate() {
        let port_pos = node.input_port_position(i);
        let init_label = node.initializers.get(input_name).map(String::as_str);
        draw_port(frame, port_pos, input_name, true, init_label, zoom, offset);
    }

    // Draw output ports on the right edge
    for (i, output_name) in node.outputs.iter().enumerate() {
        let port_pos = node.output_port_position(i);
        draw_port(frame, port_pos, output_name, false, None, zoom, offset);
    }
}

/// Draw a port circle with a label and optional initializer value.
///
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

    // Port circle — filled if has initializer, hollow if not
    let has_init = initializer.is_some();
    let circle = Path::circle(screen_center, scaled_radius);
    if has_init {
        frame.fill(&circle, Color::from_rgb(1.0, 0.9, 0.3)); // Yellow for initialized
    } else {
        frame.fill(&circle, Color::WHITE);
    }
    frame.stroke(
        &Path::circle(screen_center, scaled_radius),
        Stroke::default()
            .with_width(1.5 * zoom)
            .with_color(Color::from_rgb(0.3, 0.3, 0.3)),
    );

    // Port name label (inside the node)
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
        position: Point::new(label_x, screen_center.y - 6.0 * zoom),
        color: Color::WHITE,
        size: (PORT_FONT_SIZE * zoom).into(),
        align_x: align.into(),
        align_y: iced::alignment::Vertical::Top,
        ..CanvasText::default()
    };
    frame.fill_text(label);

    // Initializer value label (outside the node, to the left of input ports)
    if let Some(init_text) = initializer {
        let init_label = CanvasText {
            content: init_text.to_string(),
            position: Point::new(
                screen_center.x - scaled_radius - 4.0 * zoom,
                screen_center.y - 6.0 * zoom,
            ),
            color: Color::from_rgb(0.9, 0.85, 0.2),
            size: (PORT_FONT_SIZE * zoom).into(),
            align_x: iced::alignment::Horizontal::Right.into(),
            align_y: iced::alignment::Vertical::Top,
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
