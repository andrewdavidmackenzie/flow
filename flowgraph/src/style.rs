//! Visual style constants for SVG graph rendering.

/// Horizontal spacing between columns of nodes
pub const GRID_SPACING_X: f32 = 250.0;
/// X origin for the grid
pub const GRID_ORIGIN_X: f32 = 50.0;
/// Y origin for the grid
pub const GRID_ORIGIN_Y: f32 = 50.0;
/// Default node width
pub const NODE_WIDTH: f32 = 180.0;
/// Default node height
pub const NODE_HEIGHT: f32 = 120.0;
/// Vertical gap between nodes in the same column
pub const NODE_GAP_Y: f32 = 30.0;
/// Vertical spacing between ports on a node
pub const PORT_SPACING: f32 = 20.0;
/// Radius of port circles
pub const PORT_RADIUS: f32 = 5.0;
/// Corner radius for rounded rectangles
pub const CORNER_RADIUS: f32 = 10.0;
/// Header height (for node title)
pub const HEADER_HEIGHT: f32 = 30.0;
/// Font size for node labels
pub const FONT_SIZE: f32 = 14.0;
/// Font size for port labels
pub const PORT_FONT_SIZE: f32 = 10.0;
/// Arrow head size
pub const ARROW_SIZE: f32 = 8.0;
/// Padding inside nodes
pub const NODE_PADDING: f32 = 10.0;

/// Node fill colors by type
pub const COLOR_SUBFLOW: &str = "#7FFFD4";
/// Pure function fill
pub const COLOR_FUNCTION: &str = "#FF7F50";
/// Source (impure, no inputs) fill
pub const COLOR_SOURCE: &str = "#FFFFFF";
/// Sink (impure, has inputs) fill
pub const COLOR_SINK: &str = "#333333";
/// Node border color
pub const COLOR_BORDER: &str = "#444444";
/// Edge color
pub const COLOR_EDGE: &str = "#555555";
/// Initializer edge color
pub const COLOR_INITIALIZER: &str = "#4488CC";
/// Port circle fill
pub const COLOR_PORT: &str = "#888888";
/// Text color (dark)
pub const COLOR_TEXT: &str = "#222222";
/// Text color (light, for dark backgrounds)
pub const COLOR_TEXT_LIGHT: &str = "#FFFFFF";
/// Background color
pub const COLOR_BACKGROUND: &str = "#F8F8F8";
