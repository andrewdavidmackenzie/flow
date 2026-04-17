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

use flowcore::model::connection::Connection;
use flowcore::model::process_reference::ProcessReference;

/// Messages produced by the canvas interaction layer.
#[derive(Debug, Clone)]
pub(crate) enum CanvasMessage {
    /// A node was selected (or deselected if `None`).
    Selected(Option<usize>),
    /// A node was moved to a new position.
    Moved(usize, f32, f32),
    /// A node should be deleted.
    Deleted(usize),
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

/// Persistent interaction state for the canvas `Program`.
///
/// This is the `Program::State` associated type, kept alive across frames by iced.
#[derive(Debug, Clone, Default)]
pub(crate) struct CanvasInteractionState {
    /// Currently selected node index, if any
    selected_node: Option<usize>,
    /// Active drag operation, if any
    dragging: Option<DragState>,
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

    // Second pass: build node layouts
    let mut nodes = Vec::with_capacity(process_refs.len());

    for (i, pref) in process_refs.iter().enumerate() {
        let col = i % GRID_COLUMNS;
        let row = i / GRID_COLUMNS;

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

        let x = pref
            .x
            .unwrap_or(GRID_ORIGIN_X + col as f32 * GRID_SPACING_X);
        let y = pref
            .y
            .unwrap_or(GRID_ORIGIN_Y + row as f32 * GRID_SPACING_Y);
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
}

impl Default for FlowCanvasState {
    fn default() -> Self {
        Self {
            cache: canvas::Cache::new(),
        }
    }
}

impl FlowCanvasState {
    /// Create the canvas [`Element`] for displaying the given nodes and edges.
    pub(crate) fn view<'a>(
        &'a self,
        nodes: &'a [NodeLayout],
        edges: &'a [EdgeLayout],
    ) -> Element<'a, CanvasMessage> {
        Canvas::new(FlowCanvas {
            state: self,
            nodes,
            edges,
        })
        .width(Fill)
        .height(Fill)
        .into()
    }

    /// Invalidate the cached geometry so the canvas redraws on the next frame.
    pub(crate) fn request_redraw(&mut self) {
        self.cache.clear();
    }
}

/// The canvas program that draws flow nodes and connections.
struct FlowCanvas<'a> {
    state: &'a FlowCanvasState,
    nodes: &'a [NodeLayout],
    edges: &'a [EdgeLayout],
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

impl canvas::Program<CanvasMessage> for FlowCanvas<'_> {
    type State = CanvasInteractionState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<CanvasMessage>> {
        let cursor_position = cursor.position_in(bounds)?;

        match event {
            // Left mouse button pressed — select node or deselect
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(idx) = hit_test_node(self.nodes, cursor_position) {
                    // Get the node; bail if index is out of range
                    let node = self.nodes.get(idx)?;
                    state.selected_node = Some(idx);
                    state.dragging = Some(DragState {
                        node_index: idx,
                        offset_x: cursor_position.x - node.x,
                        offset_y: cursor_position.y - node.y,
                    });
                    Some(canvas::Action::publish(CanvasMessage::Selected(Some(idx))).and_capture())
                } else {
                    // Clicked empty canvas — deselect
                    state.selected_node = None;
                    state.dragging = None;
                    Some(canvas::Action::publish(CanvasMessage::Selected(None)).and_capture())
                }
            }

            // Mouse moved while dragging — publish new position
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(ref drag) = state.dragging {
                    let new_x = cursor_position.x - drag.offset_x;
                    let new_y = cursor_position.y - drag.offset_y;
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

            // Mouse button released — stop dragging
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.dragging.is_some() {
                    state.dragging = None;
                    Some(canvas::Action::request_redraw().and_capture())
                } else {
                    None
                }
            }

            // Delete / Backspace — remove selected node
            Event::Keyboard(keyboard::Event::KeyPressed {
                key:
                    keyboard::Key::Named(keyboard::key::Named::Delete | keyboard::key::Named::Backspace),
                ..
            }) => {
                if let Some(idx) = state.selected_node {
                    state.selected_node = None;
                    state.dragging = None;
                    Some(canvas::Action::publish(CanvasMessage::Deleted(idx)).and_capture())
                } else {
                    None
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
        // Draw the main cached content (edges and nodes)
        let content = self.state.cache.draw(renderer, bounds.size(), |frame| {
            draw_edges(frame, self.edges, self.nodes);
            draw_nodes(frame, self.nodes);
        });

        // Draw selection highlight as an overlay (not cached, so it updates instantly)
        if let Some(selected_idx) = state.selected_node {
            if let Some(node) = self.nodes.get(selected_idx) {
                let mut overlay = Frame::new(renderer, bounds.size());
                let highlight = Path::new(|builder| {
                    rounded_rect(
                        builder,
                        Point::new(node.x, node.y),
                        Size::new(node.width, node.height),
                        CORNER_RADIUS,
                    );
                });
                overlay.stroke(
                    &highlight,
                    Stroke::default()
                        .with_width(4.0)
                        .with_color(Color::from_rgb(1.0, 0.85, 0.0)),
                );
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
        if state.dragging.is_some() {
            return mouse::Interaction::Grabbing;
        }

        if let Some(pos) = cursor.position_in(bounds) {
            if hit_test_node(self.nodes, pos).is_some() {
                return mouse::Interaction::Grab;
            }
        }

        mouse::Interaction::default()
    }
}

/// Draw all connection edges as bezier curves.
fn draw_edges(frame: &mut Frame, edges: &[EdgeLayout], nodes: &[NodeLayout]) {
    // Build a lookup from alias to node
    let node_map: HashMap<&str, &NodeLayout> =
        nodes.iter().map(|n| (n.alias.as_str(), n)).collect();

    for edge in edges {
        let from_node = node_map.get(edge.from_node.as_str());
        let to_node = node_map.get(edge.to_node.as_str());

        if let (Some(from), Some(to)) = (from_node, to_node) {
            // Find port positions
            let from_point = if edge.from_port.is_empty() {
                // Whole-node output — use first output port
                from.output_port_position(0)
            } else {
                from.output_port_position(0) // Use first output for now
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

            draw_bezier_connection(frame, from_point, to_point);
        }
    }
}

/// Draw a bezier curve connection between two points
fn draw_bezier_connection(frame: &mut Frame, from: Point, to: Point) {
    let dx = (to.x - from.x).abs() * 0.5;
    let control1 = Point::new(from.x + dx, from.y);
    let control2 = Point::new(to.x - dx, to.y);

    let path = Path::new(|builder| {
        builder.move_to(from);
        builder.bezier_curve_to(control1, control2, to);
    });

    frame.stroke(
        &path,
        Stroke::default()
            .with_width(2.0)
            .with_color(Color::from_rgb(0.5, 0.5, 0.5)),
    );

    // Draw a small arrow head at the destination
    let arrow_size = 6.0;
    let arrow = Path::new(|builder| {
        builder.move_to(Point::new(to.x - arrow_size, to.y - arrow_size));
        builder.line_to(to);
        builder.line_to(Point::new(to.x - arrow_size, to.y + arrow_size));
    });
    frame.stroke(
        &arrow,
        Stroke::default()
            .with_width(2.0)
            .with_color(Color::from_rgb(0.5, 0.5, 0.5)),
    );
}

/// Draw all nodes onto the given frame.
fn draw_nodes(frame: &mut Frame, nodes: &[NodeLayout]) {
    for node in nodes {
        draw_node(frame, node);
    }
}

/// Draw a single node as a rounded rectangle with title, source, and ports.
fn draw_node(frame: &mut Frame, node: &NodeLayout) {
    let top_left = Point::new(node.x, node.y);
    let size = Size::new(node.width, node.height);
    let fill_color = node.fill_color();

    // Draw filled rounded rectangle
    let rect = Path::new(|builder| {
        rounded_rect(builder, top_left, size, CORNER_RADIUS);
    });
    frame.fill(&rect, fill_color);

    // Draw border
    let border = Path::new(|builder| {
        rounded_rect(builder, top_left, size, CORNER_RADIUS);
    });
    frame.stroke(
        &border,
        Stroke::default()
            .with_width(2.0)
            .with_color(Color::from_rgb(0.2, 0.2, 0.2)),
    );

    // Draw alias title centered near top of node
    let title = CanvasText {
        content: node.alias.clone(),
        position: Point::new(node.x + node.width / 2.0, node.y + 12.0),
        color: Color::WHITE,
        size: TITLE_FONT_SIZE.into(),
        align_x: iced::alignment::Horizontal::Center.into(),
        align_y: iced::alignment::Vertical::Top,
        ..CanvasText::default()
    };
    frame.fill_text(title);

    // Draw source label below title (truncated with ellipsis)
    let source_display = truncate_source(&node.source, MAX_SOURCE_CHARS);
    let source_label = CanvasText {
        content: source_display,
        position: Point::new(node.x + node.width / 2.0, node.y + 34.0),
        color: Color::from_rgba(1.0, 1.0, 1.0, 0.7),
        size: SOURCE_FONT_SIZE.into(),
        align_x: iced::alignment::Horizontal::Center.into(),
        align_y: iced::alignment::Vertical::Top,
        ..CanvasText::default()
    };
    frame.fill_text(source_label);

    // Draw input ports on the left edge
    for (i, input_name) in node.inputs.iter().enumerate() {
        let port_pos = node.input_port_position(i);
        let init_label = node.initializers.get(input_name).map(String::as_str);
        draw_port(frame, port_pos, input_name, true, init_label);
    }

    // Draw output ports on the right edge
    for (i, output_name) in node.outputs.iter().enumerate() {
        let port_pos = node.output_port_position(i);
        draw_port(frame, port_pos, output_name, false, None);
    }
}

/// Draw a port circle with a label and optional initializer value
fn draw_port(
    frame: &mut Frame,
    center: Point,
    name: &str,
    is_input: bool,
    initializer: Option<&str>,
) {
    // Port circle — filled if has initializer, hollow if not
    let has_init = initializer.is_some();
    let circle = Path::circle(center, PORT_RADIUS);
    if has_init {
        frame.fill(&circle, Color::from_rgb(1.0, 0.9, 0.3)); // Yellow for initialized
    } else {
        frame.fill(&circle, Color::WHITE);
    }
    frame.stroke(
        &Path::circle(center, PORT_RADIUS),
        Stroke::default()
            .with_width(1.5)
            .with_color(Color::from_rgb(0.3, 0.3, 0.3)),
    );

    // Port name label (inside the node)
    let (label_x, align) = if is_input {
        (
            center.x + PORT_RADIUS + 4.0,
            iced::alignment::Horizontal::Left,
        )
    } else {
        (
            center.x - PORT_RADIUS - 4.0,
            iced::alignment::Horizontal::Right,
        )
    };

    let label = CanvasText {
        content: name.to_string(),
        position: Point::new(label_x, center.y - 6.0),
        color: Color::WHITE,
        size: PORT_FONT_SIZE.into(),
        align_x: align.into(),
        align_y: iced::alignment::Vertical::Top,
        ..CanvasText::default()
    };
    frame.fill_text(label);

    // Initializer value label (outside the node, to the left of input ports)
    if let Some(init_text) = initializer {
        let init_label = CanvasText {
            content: init_text.to_string(),
            position: Point::new(center.x - PORT_RADIUS - 4.0, center.y - 6.0),
            color: Color::from_rgb(0.9, 0.85, 0.2),
            size: PORT_FONT_SIZE.into(),
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
