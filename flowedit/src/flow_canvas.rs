//! Canvas view module that renders flow process nodes and connections on an iced Canvas.
//!
//! Each [`ProcessReference`] is drawn as a rounded rectangle with its alias
//! displayed as a title. Node fill color is determined by the resolved
//! [`Process`] variant: blue for library functions, green for context functions,
//! purple for provided implementations, and orange for nested flows.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::collections::HashMap;

use iced::keyboard;
use iced::mouse;
use iced::widget::canvas::{self, Event, Frame, Geometry, Path, Stroke, Text as CanvasText};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

use flowcore::model::io::IO;
use flowcore::model::name::HasName;

use crate::node_layout::{NodeLayout, PORT_FONT_SIZE};
use crate::utils::{
    base_port_name, check_port_type_compatibility, rounded_rect, split_route, truncate_source,
};
use crate::window_state::{FlowCanvasState, ZOOM_STEP};

/// Action returned by [`WindowState::handle_canvas_message`] to signal that
/// the caller needs to perform an operation that requires `FlowEdit` state.
pub(crate) enum CanvasAction {
    /// No further action needed.
    None,
    /// The user double-clicked a node — open it in a new window.
    OpenNode(usize),
}

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

/// Messages produced by the canvas interaction layer.
#[derive(Debug, Clone)]
pub(crate) enum CanvasMessage {
    /// A node was selected (or deselected if `None`).
    Selected(Option<usize>),
    /// A node was moved to a new position (continuous during drag).
    Moved(usize, f32, f32),
    /// A node move completed (`old_x`, `old_y`, `new_x`, `new_y`) — for undo history.
    MoveCompleted(usize, f32, f32, f32, f32),
    /// A node was resized (index, `new_x`, `new_y`, `new_width`, `new_height`) — continuous during drag.
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
    /// (`node_index`, `port_name`)
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
    HoverChanged(Option<crate::window_state::Tooltip>),
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
pub(crate) enum ResizeHandle {
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
    /// Current keyboard modifier state (tracked via `ModifiersChanged` events)
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

// TODO: Replace from_node/from_port strings with a Route reference and derive
// start_pos from the node layout, so renames don't cause stale data.
#[derive(Debug, Clone)]
struct ConnectingState {
    from_node: String,
    from_port: String,
    from_output: bool,
    start_pos: Point,
    current_screen_pos: Point,
}

/// Corner radius for rounded rectangles
pub(crate) const CORNER_RADIUS: f32 = 10.0;
/// Title font size (minimum readable)
const TITLE_FONT_SIZE: f32 = 16.0;
/// Source label font size (minimum readable)
pub(crate) const SOURCE_FONT_SIZE: f32 = 12.0;
/// Port circle radius
const PORT_RADIUS: f32 = 5.0;
/// Maximum characters for source label before truncation
const MAX_SOURCE_CHARS: usize = 22;

pub(crate) fn content_extents(
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

/// Transform a world-space point to screen-space using the given zoom and scroll offset.
fn transform_point(p: Point, zoom: f32, offset: Point) -> Point {
    Point::new((p.x + offset.x) * zoom, (p.y + offset.y) * zoom)
}

/// Convert a screen-space point back to world-space.
fn screen_to_world(screen: Point, zoom: f32, offset: Point) -> Point {
    Point::new(screen.x / zoom - offset.x, screen.y / zoom - offset.y)
}

/// The canvas program that draws flow nodes and connections.
pub(crate) struct FlowCanvas<'a> {
    /// Reference to the persistent canvas state (zoom, offset, cache)
    pub(crate) state: &'a FlowCanvasState,
    /// Render nodes built from `process_refs` (owned, rebuilt each frame)
    pub(crate) nodes: Vec<NodeLayout>,
    /// Connections to render
    pub(crate) connections: &'a [Connection],
    /// Flow name (displayed on sub-flow bounding box)
    pub(crate) flow_name: &'a str,
    /// Flow-level input ports (displayed on left edge for sub-flows)
    pub(crate) flow_inputs: &'a [IO],
    /// Flow-level output ports (displayed on right edge for sub-flows)
    pub(crate) flow_outputs: &'a [IO],
    /// Whether this is a sub-flow (always draws bounding box)
    pub(crate) is_subflow: bool,
    /// Whether an auto-fit should be triggered on the next event
    pub(crate) auto_fit_pending: bool,
    /// Whether auto-fit mode is active (continuously fits to window)
    pub(crate) auto_fit_enabled: bool,
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

/// Evaluate a quadratic bezier curve at parameter `t` (0.0..=1.0).
fn quadratic_bezier_pt(p0: Point, p1: Point, p2: Point, t: f32) -> Point {
    let mt = 1.0 - t;
    Point::new(
        mt * mt * p0.x + 2.0 * mt * t * p1.x + t * t * p2.x,
        mt * mt * p0.y + 2.0 * mt * t * p1.y + t * t * p2.y,
    )
}

/// Compute flow I/O port world positions (same layout as `draw_flow_io_ports`).
fn compute_flow_io_positions(
    nodes: &[NodeLayout],
    flow_inputs: &[IO],
    flow_outputs: &[IO],
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
            nodes.iter().map(NodeLayout::x).fold(f32::MAX, f32::min),
            nodes
                .iter()
                .map(|n| n.x() + n.width())
                .fold(f32::MIN, f32::max),
            nodes.iter().map(NodeLayout::y).fold(f32::MAX, f32::min),
            nodes
                .iter()
                .map(|n| n.y() + n.height())
                .fold(f32::MIN, f32::max),
        )
    };
    let box_x = min_x - padding;
    let box_w = (max_x - min_x) + 2.0 * padding;
    let center_y = min_y.midpoint(max_y);

    let input_start_y = center_y - (flow_inputs.len() as f32 - 1.0) * spacing / 2.0;
    for (i, input) in flow_inputs.iter().enumerate() {
        let y = input_start_y + i as f32 * spacing;
        input_positions.insert(input.name().clone(), Point::new(box_x, y));
    }

    let right_x = box_x + box_w;
    let output_start_y = center_y - (flow_outputs.len() as f32 - 1.0) * spacing / 2.0;
    for (i, output) in flow_outputs.iter().enumerate() {
        let y = output_start_y + i as f32 * spacing;
        output_positions.insert(output.name().clone(), Point::new(right_x, y));
    }

    (input_positions, output_positions)
}

/// Squared distance from point `p` to the line segment `a`--`b`.
#[allow(clippy::similar_names)]
fn distance_to_segment_sq(p: Point, a: Point, b: Point) -> f32 {
    let ab_x = b.x - a.x;
    let ab_y = b.y - a.y;
    let ap_x = p.x - a.x;
    let ap_y = p.y - a.y;
    let seg_len_sq = ab_x * ab_x + ab_y * ab_y;
    if seg_len_sq < 0.001 {
        return ap_x * ap_x + ap_y * ap_y;
    }
    let t = ((ap_x * ab_x + ap_y * ab_y) / seg_len_sq).clamp(0.0, 1.0);
    let proj_x = a.x + t * ab_x;
    let proj_y = a.y + t * ab_y;
    let dx = p.x - proj_x;
    let dy = p.y - proj_y;
    dx * dx + dy * dy
}

/// Evaluate a cubic bezier curve at parameter `t` (0.0..=1.0).
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

/// Compute the appropriate mouse cursor for a given [`ResizeHandle`].
fn resize_cursor(handle: ResizeHandle) -> mouse::Interaction {
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

fn handle_delete_key(state: &mut CanvasInteractionState) -> Option<canvas::Action<CanvasMessage>> {
    if let Some(sel_conn) = state.selected_connection {
        state.selected_connection = None;
        return Some(
            canvas::Action::publish(CanvasMessage::ConnectionDeleted(sel_conn)).and_capture(),
        );
    }
    if let Some(sel_idx) = state.selected_node {
        state.selected_node = None;
        return Some(canvas::Action::publish(CanvasMessage::Deleted(sel_idx)).and_capture());
    }
    None
}

fn compute_resize(resize: &ResizeState, world_pos: Point) -> canvas::Action<CanvasMessage> {
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
    canvas::Action::publish(CanvasMessage::Resized(
        resize.node_index,
        new_x,
        new_y,
        new_w,
        new_h,
    ))
    .and_capture()
}

fn handle_scroll(
    state: &CanvasInteractionState,
    zoom: f32,
    delta: &mouse::ScrollDelta,
) -> Option<canvas::Action<CanvasMessage>> {
    let (dx, dy) = match *delta {
        mouse::ScrollDelta::Lines { x, y } => (x * SCROLL_SPEED, y * SCROLL_SPEED),
        mouse::ScrollDelta::Pixels { x, y } => (x, y),
    };

    if state.modifiers.command() {
        if dy > 0.0 {
            Some(canvas::Action::publish(CanvasMessage::ZoomBy(ZOOM_STEP)).and_capture())
        } else if dy < 0.0 {
            Some(canvas::Action::publish(CanvasMessage::ZoomBy(1.0 / ZOOM_STEP)).and_capture())
        } else {
            None
        }
    } else {
        let pan_x = dx / zoom;
        let pan_y = dy / zoom;
        Some(canvas::Action::publish(CanvasMessage::Pan(pan_x, pan_y)).and_capture())
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
    let mid_x = box_right.midpoint(box_left);
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
                Point::new(box_right.midpoint(box_left), box_bottom),
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

/// Draw a rounded bounding box around all subprocess nodes with flow I/O
/// ports on the box edges and bezier connections to internal nodes.
pub(crate) fn flow_io_bounding_box(
    nodes: &[NodeLayout],
    flow_inputs: &[IO],
    flow_outputs: &[IO],
) -> (f32, f32, f32, f32, f32, f32) {
    let spacing = 28.0;
    let padding = 80.0;
    let max_ports = flow_inputs.len().max(flow_outputs.len()).max(1) as f32;
    let default_h = max_ports * spacing + 60.0;
    let (min_x, max_x, min_y, max_y) = if nodes.is_empty() {
        (150.0, 350.0, 100.0, 100.0 + default_h)
    } else {
        (
            nodes.iter().map(NodeLayout::x).fold(f32::MAX, f32::min),
            nodes
                .iter()
                .map(|n| n.x() + n.width())
                .fold(f32::MIN, f32::max),
            nodes.iter().map(NodeLayout::y).fold(f32::MAX, f32::min),
            nodes
                .iter()
                .map(|n| n.y() + n.height())
                .fold(f32::MIN, f32::max),
        )
    };
    let box_x = min_x - padding;
    let box_y = min_y - padding;
    let box_w = (max_x - min_x) + 2.0 * padding;
    let box_h = (max_y - min_y) + 2.0 * padding;
    let center_y = min_y.midpoint(max_y);
    (box_x, box_y, box_w, box_h, center_y, spacing)
}

#[allow(clippy::too_many_arguments)]
fn draw_flow_input_port_labels(
    frame: &mut Frame,
    flow_inputs: &[IO],
    left_x: f32,
    center_y: f32,
    spacing: f32,
    port_radius: f32,
    font_size: f32,
    zoom: f32,
    offset: Point,
) -> HashMap<String, Point> {
    use std::f32::consts::PI;

    let input_color = Color::from_rgb(0.4, 0.8, 1.0);
    let mut positions: HashMap<String, Point> = HashMap::new();
    let start_y = center_y - (flow_inputs.len() as f32 - 1.0) * spacing / 2.0;
    for (i, input) in flow_inputs.iter().enumerate() {
        let world_y = start_y + i as f32 * spacing;
        let world_pos = Point::new(left_x, world_y);
        positions.insert(input.name().clone(), world_pos);
        let screen_pos = transform_point(world_pos, zoom, offset);
        let scaled_r = port_radius * zoom;
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
            content: input.name().clone(),
            position: label_pos,
            color: input_color,
            size: (font_size * zoom).into(),
            align_x: iced::alignment::Horizontal::Right.into(),
            align_y: iced::alignment::Vertical::Center,
            ..CanvasText::default()
        });
    }
    positions
}

#[allow(clippy::too_many_arguments)]
fn draw_flow_output_port_labels(
    frame: &mut Frame,
    flow_outputs: &[IO],
    right_x: f32,
    center_y: f32,
    spacing: f32,
    port_radius: f32,
    font_size: f32,
    zoom: f32,
    offset: Point,
) -> HashMap<String, Point> {
    use std::f32::consts::PI;

    let output_color = Color::from_rgb(1.0, 0.6, 0.3);
    let mut positions: HashMap<String, Point> = HashMap::new();
    let start_y = center_y - (flow_outputs.len() as f32 - 1.0) * spacing / 2.0;
    for (i, output) in flow_outputs.iter().enumerate() {
        let world_y = start_y + i as f32 * spacing;
        let world_pos = Point::new(right_x, world_y);
        positions.insert(output.name().clone(), world_pos);
        let screen_pos = transform_point(world_pos, zoom, offset);
        let scaled_r = port_radius * zoom;
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
            content: output.name().clone(),
            position: label_pos,
            color: output_color,
            size: (font_size * zoom).into(),
            align_y: iced::alignment::Vertical::Center,
            ..CanvasText::default()
        });
    }
    positions
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

impl FlowCanvas<'_> {
    /// Find the index of the first node whose bounding rectangle contains `point`.
    fn hit_test_node(&self, point: Point) -> Option<usize> {
        self.nodes.iter().enumerate().find_map(|(i, node)| {
            if point.x >= node.x()
                && point.x <= node.x() + node.width()
                && point.y >= node.y()
                && point.y <= node.y() + node.height()
            {
                Some(i)
            } else {
                None
            }
        })
    }

    /// Check whether `point` (world coords) is on the open icon of an openable node.
    /// The icon occupies a 16x16 area in the top-right corner of the node.
    fn hit_test_open_icon(&self, point: Point) -> Option<usize> {
        self.nodes.iter().enumerate().find_map(|(i, node)| {
            if !node.is_openable() {
                return None;
            }
            let icon_x = node.x() + node.width() - 22.0;
            let icon_y = node.y() + 4.0;
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

    /// Hit test all ports across all nodes.
    ///
    /// Returns `(node_index, port_name, is_output)` if the cursor is within
    /// [`PORT_HIT_RADIUS`] screen pixels of a port center.
    fn hit_test_port(
        &self,
        screen_pos: Point,
        zoom: f32,
        offset: Point,
    ) -> Option<(usize, String, bool)> {
        for (node_idx, node) in self.nodes.iter().enumerate() {
            // Check output ports (right side)
            for (port_idx, port_info) in node.outputs().iter().enumerate() {
                let world_pt = node.output_port_position(port_idx);
                let screen_pt = transform_point(world_pt, zoom, offset);
                let dx = screen_pos.x - screen_pt.x;
                let dy = screen_pos.y - screen_pt.y;
                if dx * dx + dy * dy <= PORT_HIT_RADIUS * PORT_HIT_RADIUS {
                    return Some((node_idx, port_info.name().clone(), true));
                }
            }
            // Check input ports (left side)
            for (port_idx, port_info) in node.inputs().iter().enumerate() {
                let world_pt = node.input_port_position(port_idx);
                let screen_pt = transform_point(world_pt, zoom, offset);
                let dx = screen_pos.x - screen_pt.x;
                let dy = screen_pos.y - screen_pt.y;
                if dx * dx + dy * dy <= PORT_HIT_RADIUS * PORT_HIT_RADIUS {
                    return Some((node_idx, port_info.name().clone(), false));
                }
            }
        }
        None
    }

    /// Hit test connections by sampling points along each connection's bezier curve.
    ///
    /// Returns the connection index if the cursor is within [`CONNECTION_HIT_DISTANCE`]
    /// screen pixels of any sample point on the curve.
    fn hit_test_connection(&self, screen_pos: Point, zoom: f32, offset: Point) -> Option<usize> {
        let node_map: HashMap<&str, &NodeLayout> =
            self.nodes.iter().map(|n| (n.alias(), n)).collect();

        // Compute flow I/O port positions (same layout as draw_flow_io_ports)
        let flow_io_positions =
            compute_flow_io_positions(&self.nodes, self.flow_inputs, self.flow_outputs);

        let threshold_sq = CONNECTION_HIT_DISTANCE * CONNECTION_HIT_DISTANCE;

        for (conn_idx, conn) in self.connections.iter().enumerate() {
            let (from_node_str, from_port_str) = split_route(conn.from().as_ref());
            for to_route in conn.to() {
                let (to_node_str, to_port_str) = split_route(to_route.as_ref());

                // Resolve from_point
                let from_point = if from_node_str == "input" {
                    let input_name = base_port_name(&from_port_str);
                    flow_io_positions.0.get(input_name).copied()
                } else {
                    node_map
                        .get(from_node_str.as_str())
                        .map(|n| n.find_output_pos_inline(&from_port_str))
                };

                let to_point = if to_node_str == "output" {
                    let output_name = base_port_name(&to_port_str);
                    flow_io_positions.1.get(output_name).copied()
                } else {
                    node_map
                        .get(to_node_str.as_str())
                        .map(|n| n.find_input_pos_inline(&to_port_str))
                };

                if let (Some(from_point), Some(to_point)) = (from_point, to_point) {
                    let from_s = transform_point(from_point, zoom, offset);
                    let to_s = transform_point(to_point, zoom, offset);

                    let is_self = from_node_str == to_node_str;

                    // Build sample points along the actual drawn path
                    let sample_points: Vec<Point> = if is_self {
                        let from_node_ref = node_map.get(from_node_str.as_str());
                        let Some(from_n) = from_node_ref else {
                            continue;
                        };
                        let (box_right, box_bottom, box_left, mid_x) = loopback_waypoints(
                            from_n.x(),
                            from_n.y(),
                            from_n.width(),
                            from_n.height(),
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
                        let is_flow_io = from_node_str == "input" || to_node_str == "output";
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
                                return Some(conn_idx);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn try_start_connection(
        &self,
        state: &mut CanvasInteractionState,
        cursor_position: Point,
        zoom: f32,
        offset: Point,
    ) -> Option<canvas::Action<CanvasMessage>> {
        let (node_idx, port_name, is_output) = self.hit_test_port(cursor_position, zoom, offset)?;
        let node = self.nodes.get(node_idx)?;
        let port_world_pos = if is_output {
            let port_idx = node
                .outputs()
                .iter()
                .position(|p| p.name() == &port_name)
                .unwrap_or(0);
            node.output_port_position(port_idx)
        } else {
            let port_idx = node
                .inputs()
                .iter()
                .position(|p| p.name() == &port_name)
                .unwrap_or(0);
            node.input_port_position(port_idx)
        };
        state.connecting = Some(ConnectingState {
            from_node: node.alias().to_string(),
            from_port: port_name,
            from_output: is_output,
            start_pos: port_world_pos,
            current_screen_pos: cursor_position,
        });
        Some(canvas::Action::request_redraw().and_capture())
    }

    fn handle_left_press(
        &self,
        state: &mut CanvasInteractionState,
        cursor_position: Point,
        zoom: f32,
        offset: Point,
        world_pos: Point,
    ) -> Option<canvas::Action<CanvasMessage>> {
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
                        start_node_x: sel_node.x(),
                        start_node_y: sel_node.y(),
                        start_width: sel_node.width(),
                        start_height: sel_node.height(),
                    });
                    return Some(canvas::Action::request_redraw().and_capture());
                }
            }
        }

        let on_a_port = self.hit_test_port(cursor_position, zoom, offset).is_some();
        if !on_a_port {
            if let Some(conn_idx) = self.hit_test_connection(cursor_position, zoom, offset) {
                state.selected_connection = Some(conn_idx);
                state.selected_node = None;
                state.dragging = None;
                return Some(
                    canvas::Action::publish(CanvasMessage::ConnectionSelected(Some(conn_idx)))
                        .and_capture(),
                );
            }
        }

        if let Some(action) = self.try_start_connection(state, cursor_position, zoom, offset) {
            return Some(action);
        }

        if let Some(idx) = self.hit_test_open_icon(world_pos) {
            return Some(canvas::Action::publish(CanvasMessage::OpenNode(idx)).and_capture());
        }

        if let Some(idx) = self.hit_test_node(world_pos) {
            let node = self.nodes.get(idx)?;
            state.selected_node = Some(idx);
            state.selected_connection = None;
            state.dragging = Some(DragState {
                node_index: idx,
                offset_x: world_pos.x - node.x(),
                offset_y: world_pos.y - node.y(),
                start_x: node.x(),
                start_y: node.y(),
            });
            Some(canvas::Action::publish(CanvasMessage::Selected(Some(idx))).and_capture())
        } else {
            state.selected_node = None;
            state.selected_connection = None;
            state.dragging = None;
            Some(canvas::Action::publish(CanvasMessage::Selected(None)).and_capture())
        }
    }

    fn handle_right_press(
        &self,
        cursor_position: Point,
        zoom: f32,
        offset: Point,
        world_pos: Point,
    ) -> Option<canvas::Action<CanvasMessage>> {
        if let Some((node_idx, port_name, is_output)) =
            self.hit_test_port(cursor_position, zoom, offset)
        {
            if !is_output {
                return Some(
                    canvas::Action::publish(CanvasMessage::InitializerEdit(node_idx, port_name))
                        .and_capture(),
                );
            }
        }
        if self.hit_test_node(world_pos).is_none() {
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

    fn handle_cursor_moved(
        &self,
        state: &mut CanvasInteractionState,
        cursor_position: Point,
        zoom: f32,
        offset: Point,
        world_pos: Point,
    ) -> Option<canvas::Action<CanvasMessage>> {
        if let Some(ref mut connecting) = state.connecting {
            connecting.current_screen_pos = cursor_position;
            return Some(canvas::Action::request_redraw().and_capture());
        }
        if let Some(ref resize) = state.resizing {
            return Some(compute_resize(resize, world_pos));
        }
        if let Some(ref pan) = state.panning {
            let dx = (cursor_position.x - pan.last_screen_pos.x) / zoom;
            let dy = (cursor_position.y - pan.last_screen_pos.y) / zoom;
            state.panning = Some(PanState {
                last_screen_pos: cursor_position,
            });
            return Some(canvas::Action::publish(CanvasMessage::Pan(dx, dy)).and_capture());
        }
        if let Some(ref drag) = state.dragging {
            let new_x = world_pos.x - drag.offset_x;
            let new_y = world_pos.y - drag.offset_y;
            return Some(
                canvas::Action::publish(CanvasMessage::Moved(drag.node_index, new_x, new_y))
                    .and_capture(),
            );
        }

        self.handle_hover(state, cursor_position, zoom, offset, world_pos)
    }

    fn handle_hover(
        &self,
        state: &mut CanvasInteractionState,
        cursor_position: Point,
        zoom: f32,
        offset: Point,
        world_pos: Point,
    ) -> Option<canvas::Action<CanvasMessage>> {
        if let Some((node_idx, port_name, is_output)) =
            self.hit_test_port(cursor_position, zoom, offset)
        {
            if let Some(node) = self.nodes.get(node_idx) {
                let ports = if is_output {
                    node.outputs()
                } else {
                    node.inputs()
                };
                let type_text = ports.iter().find(|p| p.name() == &port_name).map_or_else(
                    || port_name.clone(),
                    |p| {
                        if p.datatypes().is_empty() {
                            format!("{port_name}: (any)")
                        } else {
                            format!(
                                "{port_name}: {}",
                                p.datatypes()
                                    .iter()
                                    .map(ToString::to_string)
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        }
                    },
                );
                state.hover_node = None;
                return Some(canvas::Action::publish(CanvasMessage::HoverChanged(Some(
                    crate::window_state::Tooltip {
                        text: type_text,
                        x: cursor_position.x,
                        y: cursor_position.y - 20.0,
                    },
                ))));
            }
        }

        let new_hover = self.hit_test_node(world_pos);
        if new_hover != state.hover_node || new_hover.is_some() {
            state.hover_node = new_hover;
            let tooltip_data = new_hover.and_then(|idx| self.nodes.get(idx)).and_then(|n| {
                let bottom_center = transform_point(
                    Point::new(n.x() + n.width() / 2.0, n.y() + n.height()),
                    zoom,
                    offset,
                );
                if n.is_in_source_text_zone(world_pos) {
                    Some(crate::window_state::Tooltip {
                        text: n.source().to_string(),
                        x: bottom_center.x,
                        y: bottom_center.y,
                    })
                } else if !n.description().is_empty() {
                    Some(crate::window_state::Tooltip {
                        text: n.description(),
                        x: bottom_center.x,
                        y: bottom_center.y,
                    })
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

    fn handle_left_release(
        &self,
        state: &mut CanvasInteractionState,
        cursor_position: Point,
        zoom: f32,
        offset: Point,
    ) -> Option<canvas::Action<CanvasMessage>> {
        if let Some(connecting) = state.connecting.take() {
            return Some(self.finish_connecting(&connecting, cursor_position, zoom, offset));
        }
        if let Some(resize) = state.resizing.take() {
            if let Some(node) = self.nodes.get(resize.node_index) {
                return Some(
                    canvas::Action::publish(CanvasMessage::ResizeCompleted(
                        resize.node_index,
                        resize.start_node_x,
                        resize.start_node_y,
                        resize.start_width,
                        resize.start_height,
                        node.x(),
                        node.y(),
                        node.width(),
                        node.height(),
                    ))
                    .and_capture(),
                );
            }
            Some(canvas::Action::request_redraw().and_capture())
        } else if let Some(drag) = state.dragging.take() {
            if let Some(node) = self.nodes.get(drag.node_index) {
                return Some(
                    canvas::Action::publish(CanvasMessage::MoveCompleted(
                        drag.node_index,
                        drag.start_x,
                        drag.start_y,
                        node.x(),
                        node.y(),
                    ))
                    .and_capture(),
                );
            }
            Some(canvas::Action::request_redraw().and_capture())
        } else {
            None
        }
    }

    fn finish_connecting(
        &self,
        connecting: &ConnectingState,
        cursor_position: Point,
        zoom: f32,
        offset: Point,
    ) -> canvas::Action<CanvasMessage> {
        if let Some((target_idx, target_port, target_is_output)) =
            self.hit_test_port(cursor_position, zoom, offset)
        {
            if connecting.from_output != target_is_output {
                if let Some(target_node) = self.nodes.get(target_idx) {
                    let source_node = self
                        .nodes
                        .iter()
                        .find(|n| n.alias() == connecting.from_node);
                    let types_ok = check_port_type_compatibility(
                        source_node,
                        &connecting.from_port,
                        connecting.from_output,
                        target_node,
                        &target_port,
                        target_is_output,
                    );

                    if types_ok {
                        let (from_node, from_port, to_node, to_port) = if connecting.from_output {
                            (
                                connecting.from_node.clone(),
                                connecting.from_port.clone(),
                                target_node.alias().to_string(),
                                target_port,
                            )
                        } else {
                            (
                                target_node.alias().to_string(),
                                target_port,
                                connecting.from_node.clone(),
                                connecting.from_port.clone(),
                            )
                        };
                        return canvas::Action::publish(CanvasMessage::ConnectionCreated {
                            from_node,
                            from_port,
                            to_node,
                            to_port,
                        })
                        .and_capture();
                    }
                }
            }
        }
        canvas::Action::request_redraw().and_capture()
    }

    fn draw_selection_highlight(
        &self,
        overlay: &mut Frame,
        selected_idx: usize,
        zoom: f32,
        offset: Point,
    ) {
        if let Some(node) = self.nodes.get(selected_idx) {
            let screen_pos = transform_point(Point::new(node.x(), node.y()), zoom, offset);
            let screen_size = Size::new(node.width() * zoom, node.height() * zoom);
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

    fn draw_connection_preview(
        &self,
        overlay: &mut Frame,
        connecting: &ConnectingState,
        zoom: f32,
        offset: Point,
    ) {
        let start_screen = transform_point(connecting.start_pos, zoom, offset);
        let end_screen = connecting.current_screen_pos;

        let preview_color = Color::from_rgb(0.3, 0.9, 0.3);
        let dx_ctrl = (end_screen.x - start_screen.x).abs().max(60.0 * zoom) * 0.5;

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

        if let Some((target_idx, target_port, target_is_output)) =
            self.hit_test_port(end_screen, zoom, offset)
        {
            if connecting.from_output != target_is_output {
                if let Some(target_node) = self.nodes.get(target_idx) {
                    let port_world = if target_is_output {
                        let pidx = target_node
                            .outputs()
                            .iter()
                            .position(|p| p.name() == &target_port)
                            .unwrap_or(0);
                        target_node.output_port_position(pidx)
                    } else {
                        let pidx = target_node
                            .inputs()
                            .iter()
                            .position(|p| p.name() == &target_port)
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

    /// Draw all connection edges as bezier curves.
    fn draw_edges(&self, frame: &mut Frame, zoom: f32, offset: Point, selected: Option<usize>) {
        // Build a lookup from alias to node
        let node_map: HashMap<&str, &NodeLayout> =
            self.nodes.iter().map(|n| (n.alias(), n)).collect();

        // Draw selected connection last so it renders on top of crossing connections
        let draw_order: Vec<usize> = (0..self.connections.len())
            .filter(|i| selected != Some(*i))
            .chain(selected.filter(|i| *i < self.connections.len()))
            .collect();

        for conn_idx in draw_order {
            let Some(conn) = self.connections.get(conn_idx) else {
                continue;
            };
            let (from_node_str, from_port_str) = split_route(conn.from().as_ref());
            for to_route in conn.to() {
                let (to_node_str, to_port_str) = split_route(to_route.as_ref());
                let from_node = node_map.get(from_node_str.as_str());
                let to_node = node_map.get(to_node_str.as_str());

                if let (Some(from), Some(to)) = (from_node, to_node) {
                    // Find port positions (in world space)
                    let from_point = if from_port_str.is_empty() {
                        from.output_port_position(0)
                    } else {
                        let base = base_port_name(&from_port_str);
                        let port_idx = from
                            .outputs()
                            .iter()
                            .position(|p| p.name() == base)
                            .unwrap_or(0);
                        from.output_port_position(port_idx)
                    };

                    let to_point = if to_port_str.is_empty() {
                        to.input_port_position(0)
                    } else {
                        let base = base_port_name(&to_port_str);
                        let port_idx = to
                            .inputs()
                            .iter()
                            .position(|p| p.name() == base)
                            .unwrap_or(0);
                        to.input_port_position(port_idx)
                    };

                    let is_self_connection = from_node_str == to_node_str;
                    let node_bounds = if is_self_connection {
                        Some((from.x(), from.y(), from.width(), from.height()))
                    } else {
                        None
                    };
                    let is_selected = selected == Some(conn_idx);
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
                    let conn_name = conn.name();
                    if !conn_name.is_empty() {
                        let from_s = transform_point(from_point, zoom, offset);
                        let to_s = transform_point(to_point, zoom, offset);
                        let mid = if is_self_connection {
                            let (_, box_bottom, box_left, mid_x) = loopback_waypoints(
                                from.x(),
                                from.y(),
                                from.width(),
                                from.height(),
                                zoom,
                                offset,
                            );
                            let _ = box_left;
                            Point::new(mid_x, box_bottom)
                        } else {
                            Point::new(from_s.x.midpoint(to_s.x), from_s.y.midpoint(to_s.y))
                        };
                        let name_label = CanvasText {
                            content: conn_name.clone(),
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
    }

    /// Draw all nodes onto the given frame, applying zoom and offset.
    fn draw_nodes(&self, frame: &mut Frame, zoom: f32, offset: Point) {
        for node in &self.nodes {
            draw_node(frame, node, zoom, offset);
        }
    }

    fn draw_flow_io_ports(
        &self,
        frame: &mut Frame,
        selected_connection: Option<usize>,
        zoom: f32,
        offset: Point,
    ) {
        if !self.is_subflow {
            return;
        }

        let port_radius = 6.0;
        let font_size = 13.0;
        let corner = 16.0;

        let (box_x, box_y, box_w, box_h, center_y, spacing) =
            flow_io_bounding_box(&self.nodes, self.flow_inputs, self.flow_outputs);

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

        if !self.flow_name.is_empty() {
            let name_pos =
                transform_point(Point::new(box_x + box_w / 2.0, box_y + 8.0), zoom, offset);
            frame.fill_text(CanvasText {
                content: self.flow_name.to_string(),
                position: name_pos,
                color: Color::from_rgb(0.9, 0.6, 0.2),
                size: (16.0 * zoom).into(),
                align_x: iced::alignment::Horizontal::Center.into(),
                ..CanvasText::default()
            });
        }

        let input_positions = draw_flow_input_port_labels(
            frame,
            self.flow_inputs,
            box_x,
            center_y,
            spacing,
            port_radius,
            font_size,
            zoom,
            offset,
        );
        let output_positions = draw_flow_output_port_labels(
            frame,
            self.flow_outputs,
            box_x + box_w,
            center_y,
            spacing,
            port_radius,
            font_size,
            zoom,
            offset,
        );

        self.draw_flow_io_connections(
            frame,
            &input_positions,
            &output_positions,
            selected_connection,
            zoom,
            offset,
        );
    }

    fn draw_flow_io_connections(
        &self,
        frame: &mut Frame,
        input_positions: &HashMap<String, Point>,
        output_positions: &HashMap<String, Point>,
        selected_connection: Option<usize>,
        zoom: f32,
        offset: Point,
    ) {
        let conn_color = Color::from_rgba(0.7, 0.7, 0.7, 0.6);
        let sel_color = Color::from_rgb(1.0, 0.85, 0.0);
        for (conn_idx, conn) in self.connections.iter().enumerate() {
            let is_selected = selected_connection == Some(conn_idx);
            let color = if is_selected { sel_color } else { conn_color };
            let width = if is_selected { 3.0 } else { 1.5 };
            let (from_node_str, from_port_str) = split_route(conn.from().as_ref());
            for to_route in conn.to() {
                let (to_node_str, to_port_str) = split_route(to_route.as_ref());
                if from_node_str == "input" {
                    let input_name = base_port_name(&from_port_str);
                    if let Some(&from_world) = input_positions.get(input_name) {
                        if let Some(to_world) = self.find_node_input_pos(&to_node_str, &to_port_str)
                        {
                            draw_flow_io_bezier(
                                frame, from_world, to_world, zoom, offset, color, width,
                            );
                        }
                    }
                }
                if to_node_str == "output" {
                    let output_name = base_port_name(&to_port_str);
                    if let Some(&to_world) = output_positions.get(output_name) {
                        if let Some(from_world) =
                            self.find_node_output_pos(&from_node_str, &from_port_str)
                        {
                            draw_flow_io_bezier(
                                frame, from_world, to_world, zoom, offset, color, width,
                            );
                        }
                    }
                }
            }
        }
    }

    fn find_node_input_pos(&self, alias: &str, port: &str) -> Option<Point> {
        let node = self.nodes.iter().find(|n| n.alias() == alias)?;
        let base = base_port_name(port);
        let port_idx = node
            .inputs()
            .iter()
            .position(|p| p.name() == base)
            .unwrap_or(0);
        Some(node.input_port_position(port_idx))
    }

    fn find_node_output_pos(&self, alias: &str, port: &str) -> Option<Point> {
        let node = self.nodes.iter().find(|n| n.alias() == alias)?;
        if port.is_empty() {
            Some(node.output_port_position(0))
        } else {
            let base = base_port_name(port);
            let port_idx = node
                .outputs()
                .iter()
                .position(|p| p.name() == base)
                .unwrap_or(0);
            Some(node.output_port_position(port_idx))
        }
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

        match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.modifiers = *modifiers;
                return None;
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key:
                    keyboard::Key::Named(keyboard::key::Named::Delete | keyboard::key::Named::Backspace),
                ..
            }) => return handle_delete_key(state),
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
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                self.handle_left_press(state, cursor_position, zoom, offset, world_pos)
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                self.handle_right_press(cursor_position, zoom, offset, world_pos)
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)) => {
                state.panning = Some(PanState {
                    last_screen_pos: cursor_position,
                });
                Some(canvas::Action::request_redraw().and_capture())
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                self.handle_cursor_moved(state, cursor_position, zoom, offset, world_pos)
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                self.handle_left_release(state, cursor_position, zoom, offset)
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Middle)) => {
                if state.panning.is_some() {
                    state.panning = None;
                    Some(canvas::Action::request_redraw().and_capture())
                } else {
                    None
                }
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                handle_scroll(state, zoom, delta)
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

        // Draw the main cached content (connections, nodes, and flow I/O ports)
        let content = self.state.cache.draw(renderer, bounds.size(), |frame| {
            self.draw_nodes(frame, zoom, offset);
            self.draw_flow_io_ports(frame, state.selected_connection, zoom, offset);
            self.draw_edges(frame, zoom, offset, state.selected_connection);
        });

        let needs_overlay = state.selected_node.is_some()
            || state.connecting.is_some()
            || state.hover_node.is_some();

        if needs_overlay {
            let mut overlay = Frame::new(renderer, bounds.size());

            if let Some(selected_idx) = state.selected_node {
                self.draw_selection_highlight(&mut overlay, selected_idx, zoom, offset);
            }

            if let Some(ref connecting) = state.connecting {
                self.draw_connection_preview(&mut overlay, connecting, zoom, offset);
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
            return resize_cursor(resize.handle);
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
                        return resize_cursor(handle);
                    }
                }
            }

            // Check if hovering over a port
            if self
                .hit_test_port(pos, self.state.zoom, self.state.scroll_offset)
                .is_some()
            {
                return mouse::Interaction::Crosshair;
            }

            let world_pos = screen_to_world(pos, self.state.zoom, self.state.scroll_offset);

            if self.hit_test_open_icon(world_pos).is_some() {
                return mouse::Interaction::Pointer;
            }

            if self.hit_test_node(world_pos).is_some() {
                return mouse::Interaction::Grab;
            }
        }

        mouse::Interaction::default()
    }
}

/// Draw a single node as a rounded rectangle with title, source, and ports.
fn draw_node(frame: &mut Frame, node: &NodeLayout, zoom: f32, offset: Point) {
    let top_left = transform_point(Point::new(node.x(), node.y()), zoom, offset);
    let size = Size::new(node.width() * zoom, node.height() * zoom);
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
        Point::new(node.x() + node.width() / 2.0, node.y() + 12.0),
        zoom,
        offset,
    );
    let title = CanvasText {
        content: node.alias().to_string(),
        position: title_pos,
        color: Color::WHITE,
        size: (TITLE_FONT_SIZE * zoom).into(),
        align_x: iced::alignment::Horizontal::Center.into(),
        align_y: iced::alignment::Vertical::Top,
        ..CanvasText::default()
    };
    frame.fill_text(title);

    // Draw source label below title (truncated with ellipsis)
    let source_display = truncate_source(node.source(), MAX_SOURCE_CHARS);
    let source_pos = transform_point(
        Point::new(node.x() + node.width() / 2.0, node.y() + 34.0),
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
        let icon_x = node.x() + node.width() - 22.0;
        let icon_y = node.y() + 4.0;
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
    for (i, input_port) in node.inputs().iter().enumerate() {
        let port_pos = node.input_port_position(i);
        let init_label = node.initializer_display(input_port.name());
        draw_port(
            frame,
            port_pos,
            input_port.name(),
            true,
            init_label.as_deref(),
            zoom,
            offset,
        );
    }

    // Draw output ports on the right edge
    for (i, output_port) in node.outputs().iter().enumerate() {
        let port_pos = node.output_port_position(i);
        draw_port(
            frame,
            port_pos,
            output_port.name(),
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
    use std::f32::consts::PI;

    let screen_center = transform_point(center, zoom, offset);
    let scaled_radius = PORT_RADIUS * zoom;

    let has_init = initializer.is_some();
    let fill_color = if has_init {
        Color::from_rgb(1.0, 0.9, 0.3)
    } else {
        Color::WHITE
    };

    // Draw semi-circle: curved side faces inside the box, flat edge on the box boundary
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

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod test {
    use super::*;
    use crate::node_layout::NodeLayout;
    use flowcore::model::flow_definition::FlowDefinition;
    use flowcore::model::function_definition::FunctionDefinition;
    use flowcore::model::io::IO;
    use flowcore::model::process::Process;
    use flowcore::model::process_reference::ProcessReference;
    use flowcore::model::route::Route;
    use url::Url;

    fn make_flow_canvas(state: &FlowCanvasState, nodes: Vec<NodeLayout>) -> FlowCanvas<'_> {
        FlowCanvas {
            state,
            nodes,
            connections: &[],
            flow_name: "",
            flow_inputs: &[],
            flow_outputs: &[],
            is_subflow: false,
            auto_fit_pending: false,
            auto_fit_enabled: false,
        }
    }

    fn test_node(alias: &str, source: &str, process: Option<Process>) -> NodeLayout {
        NodeLayout {
            process_ref: ProcessReference {
                alias: alias.into(),
                source: source.into(),
                initializations: std::collections::BTreeMap::new(),
                x: Some(100.0),
                y: Some(100.0),
                width: Some(180.0),
                height: Some(120.0),
            },
            process,
        }
    }

    fn lib_function() -> FunctionDefinition {
        let mut f = FunctionDefinition::default();
        f.lib_reference = Some(Url::parse("lib://test").expect("valid url"));
        f
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
    fn hit_test_node_inside() {
        let state = FlowCanvasState::default();
        let canvas = make_flow_canvas(&state, vec![test_node("test", "lib://test", None)]);
        assert_eq!(canvas.hit_test_node(Point::new(150.0, 150.0)), Some(0));
    }

    #[test]
    fn hit_test_node_outside() {
        let state = FlowCanvasState::default();
        let canvas = make_flow_canvas(&state, vec![test_node("test", "lib://test", None)]);
        assert_eq!(canvas.hit_test_node(Point::new(50.0, 50.0)), None);
    }

    #[test]
    fn hit_test_node_miss() {
        let state = FlowCanvasState::default();
        let node = test_node("n", "lib://test", None);
        let canvas = make_flow_canvas(&state, vec![node.clone()]);
        assert_eq!(canvas.hit_test_node(Point::new(150.0, 150.0)), Some(0));
        let canvas2 = make_flow_canvas(&state, vec![node]);
        assert_eq!(canvas2.hit_test_node(Point::new(50.0, 50.0)), None);
    }

    #[test]
    fn hit_test_open_icon_only_openable() {
        let state = FlowCanvasState::default();
        let lib_node = test_node(
            "n",
            "lib://test",
            Some(Process::FunctionProcess(lib_function())),
        );
        let local_node = test_node(
            "n",
            "subflow",
            Some(Process::FlowProcess(FlowDefinition::default())),
        );
        // Library nodes are not openable
        let canvas = make_flow_canvas(&state, vec![lib_node]);
        assert_eq!(canvas.hit_test_open_icon(Point::new(278.0, 104.0)), None);
        // Flow nodes are openable
        let canvas = make_flow_canvas(&state, vec![local_node]);
        assert!(canvas
            .hit_test_open_icon(Point::new(278.0, 104.0))
            .is_some());
    }

    #[test]
    fn compute_flow_io_positions_with_nodes() {
        let nodes = vec![test_node("n", "", None)];
        let inputs = vec![IO::new_named(vec![], Route::default(), "data")];
        let outputs = vec![IO::new_named(vec![], Route::default(), "result")];
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
        let inputs = vec![IO::new_named(vec![], Route::default(), "in")];
        let outputs = vec![IO::new_named(vec![], Route::default(), "out")];
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
}
