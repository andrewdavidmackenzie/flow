//! Canvas view module that renders flow process nodes and connections on an iced Canvas.
//!
//! Each [`ProcessReference`] is drawn as a rounded rectangle with its alias
//! displayed as a title. Node fill color is determined by the process source:
//! blue for `lib://`, green for `context://`, purple for provided implementations,
//! and orange for nested flows.

use std::collections::HashMap;

use iced::mouse;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke, Text as CanvasText};
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Size, Theme};

use flowcore::model::connection::Connection;
use flowcore::model::process_reference::ProcessReference;

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
    /// Input port names (parsed from initializations)
    pub inputs: Vec<String>,
    /// Output port name (simplified — just "output" for now)
    pub outputs: Vec<String>,
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

/// Build a list of [`NodeLayout`] from process references, using their layout
/// fields if available or falling back to auto-grid positioning.
pub(crate) fn build_node_layouts(process_refs: &[ProcessReference]) -> Vec<NodeLayout> {
    let mut nodes = Vec::with_capacity(process_refs.len());

    for (i, pref) in process_refs.iter().enumerate() {
        let col = i % GRID_COLUMNS;
        let row = i / GRID_COLUMNS;

        // Collect input names from initializations
        let inputs: Vec<String> = pref.initializations.keys().cloned().collect();
        // For now, use a generic "output" port — we don't know output names without
        // loading the function definition
        let outputs = vec!["output".to_string()];

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

        nodes.push(NodeLayout {
            alias: pref.alias.clone(),
            source: pref.source.clone(),
            x,
            y,
            width,
            height,
            inputs,
            outputs,
        });
    }

    nodes
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
    ) -> Element<'a, ()> {
        Canvas::new(FlowCanvas {
            state: self,
            nodes,
            edges,
        })
        .width(Fill)
        .height(Fill)
        .into()
    }
}

/// The canvas program that draws flow nodes and connections.
struct FlowCanvas<'a> {
    state: &'a FlowCanvasState,
    nodes: &'a [NodeLayout],
    edges: &'a [EdgeLayout],
}

impl canvas::Program<()> for FlowCanvas<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let content = self.state.cache.draw(renderer, bounds.size(), |frame| {
            draw_edges(frame, self.edges, self.nodes);
            draw_nodes(frame, self.nodes);
        });

        vec![content]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        let _ = (bounds, cursor);
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
        draw_port(frame, port_pos, input_name, true);
    }

    // Draw output ports on the right edge
    for (i, output_name) in node.outputs.iter().enumerate() {
        let port_pos = node.output_port_position(i);
        draw_port(frame, port_pos, output_name, false);
    }
}

/// Draw a port circle with a label
fn draw_port(frame: &mut Frame, center: Point, name: &str, is_input: bool) {
    // Port circle
    let circle = Path::circle(center, PORT_RADIUS);
    frame.fill(&circle, Color::WHITE);
    frame.stroke(
        &Path::circle(center, PORT_RADIUS),
        Stroke::default()
            .with_width(1.5)
            .with_color(Color::from_rgb(0.3, 0.3, 0.3)),
    );

    // Port label
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
