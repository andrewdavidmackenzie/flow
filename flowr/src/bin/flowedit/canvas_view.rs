//! Canvas view module that renders flow process nodes on an iced Canvas.
//!
//! Each [`ProcessReference`] is drawn as a rounded rectangle with its alias
//! displayed as a title. Node fill color is determined by the process source:
//! blue for `lib://`, green for `context://`, and orange for everything else.

use iced::mouse;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke, Text as CanvasText};
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Size, Theme};

use flowcore::model::process_reference::ProcessReference;

/// Default node width when no layout width is specified
const DEFAULT_WIDTH: f32 = 180.0;
/// Default node height when no layout height is specified
const DEFAULT_HEIGHT: f32 = 120.0;
/// Horizontal spacing between auto-laid-out nodes
const GRID_SPACING_X: f32 = 220.0;
/// Vertical spacing between auto-laid-out nodes
const GRID_SPACING_Y: f32 = 160.0;
/// Number of columns in auto-layout grid
const GRID_COLUMNS: usize = 3;
/// Starting X offset for auto-layout
const GRID_ORIGIN_X: f32 = 50.0;
/// Starting Y offset for auto-layout
const GRID_ORIGIN_Y: f32 = 50.0;
/// Corner radius for rounded rectangles
const CORNER_RADIUS: f32 = 10.0;
/// Title font size
const TITLE_FONT_SIZE: f32 = 14.0;
/// Source label font size
const SOURCE_FONT_SIZE: f32 = 10.0;

/// A positioned node derived from a [`ProcessReference`], ready for rendering.
#[derive(Debug, Clone)]
pub(crate) struct NodeLayout {
    /// Display name (alias) for this node
    alias: String,
    /// Source path of the process
    source: String,
    /// X coordinate on the canvas
    x: f32,
    /// Y coordinate on the canvas
    y: f32,
    /// Width of the node rectangle
    width: f32,
    /// Height of the node rectangle
    height: f32,
}

impl NodeLayout {
    /// Determine the fill color based on the process source string.
    fn fill_color(&self) -> Color {
        if self.source.starts_with("lib://") {
            // Blue for library references
            Color::from_rgb(0.3, 0.5, 0.9)
        } else if self.source.starts_with("context://") {
            // Green for context references
            Color::from_rgb(0.3, 0.75, 0.45)
        } else {
            // Orange for nested flows or other sources
            Color::from_rgb(0.9, 0.6, 0.2)
        }
    }
}

/// Build a list of [`NodeLayout`] from process references, using their layout
/// fields if available or falling back to auto-grid positioning.
pub(crate) fn build_node_layouts(process_refs: &[ProcessReference]) -> Vec<NodeLayout> {
    let mut nodes = Vec::with_capacity(process_refs.len());

    for (i, pref) in process_refs.iter().enumerate() {
        let col = i % GRID_COLUMNS;
        let row = i / GRID_COLUMNS;

        let x = pref
            .x
            .unwrap_or(GRID_ORIGIN_X + col as f32 * GRID_SPACING_X);
        let y = pref
            .y
            .unwrap_or(GRID_ORIGIN_Y + row as f32 * GRID_SPACING_Y);
        let width = pref.width.unwrap_or(DEFAULT_WIDTH);
        let height = pref.height.unwrap_or(DEFAULT_HEIGHT);

        nodes.push(NodeLayout {
            alias: pref.alias.clone(),
            source: pref.source.clone(),
            x,
            y,
            width,
            height,
        });
    }

    nodes
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
    /// Create the canvas [`Element`] for displaying the given nodes.
    pub(crate) fn view<'a>(&'a self, nodes: &'a [NodeLayout]) -> Element<'a, ()> {
        Canvas::new(FlowCanvas { state: self, nodes })
            .width(Fill)
            .height(Fill)
            .into()
    }
}

/// The canvas program that draws flow nodes.
struct FlowCanvas<'a> {
    state: &'a FlowCanvasState,
    nodes: &'a [NodeLayout],
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

/// Draw all nodes onto the given frame.
fn draw_nodes(frame: &mut Frame, nodes: &[NodeLayout]) {
    for node in nodes {
        draw_node(frame, node);
    }
}

/// Draw a single node as a rounded rectangle with a title and source label.
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
        position: Point::new(node.x + node.width / 2.0, node.y + 15.0),
        color: Color::WHITE,
        size: TITLE_FONT_SIZE.into(),
        align_x: iced::alignment::Horizontal::Center.into(),
        align_y: iced::alignment::Vertical::Top,
        ..CanvasText::default()
    };
    frame.fill_text(title);

    // Draw source label below title (truncated if too long)
    let source_display = truncate_source(&node.source, 28);
    let source_label = CanvasText {
        content: source_display,
        position: Point::new(node.x + node.width / 2.0, node.y + 35.0),
        color: Color::from_rgba(1.0, 1.0, 1.0, 0.7),
        size: SOURCE_FONT_SIZE.into(),
        align_x: iced::alignment::Horizontal::Center.into(),
        align_y: iced::alignment::Vertical::Top,
        ..CanvasText::default()
    };
    frame.fill_text(source_label);
}

/// Build a rounded rectangle path using quadratic bezier curves at corners.
fn rounded_rect(builder: &mut canvas::path::Builder, top_left: Point, size: Size, radius: f32) {
    let r = radius.min(size.width / 2.0).min(size.height / 2.0);
    let x = top_left.x;
    let y = top_left.y;
    let w = size.width;
    let h = size.height;

    // Start at top-left, after the corner radius
    builder.move_to(Point::new(x + r, y));

    // Top edge to top-right corner
    builder.line_to(Point::new(x + w - r, y));
    builder.quadratic_curve_to(Point::new(x + w, y), Point::new(x + w, y + r));

    // Right edge to bottom-right corner
    builder.line_to(Point::new(x + w, y + h - r));
    builder.quadratic_curve_to(Point::new(x + w, y + h), Point::new(x + w - r, y + h));

    // Bottom edge to bottom-left corner
    builder.line_to(Point::new(x + r, y + h));
    builder.quadratic_curve_to(Point::new(x, y + h), Point::new(x, y + h - r));

    // Left edge to top-left corner
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
