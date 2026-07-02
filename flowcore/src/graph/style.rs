//! Visual style constants shared between flowc SVG generation and flowedit.
//!
//! Colors are stored as hex strings so this module has no dependency on
//! any rendering framework (iced, svg, etc.).

#![allow(clippy::cast_precision_loss)]

use crate::model::process::Process;

// --- Layout geometry ---

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
/// Padding inside nodes
pub const NODE_PADDING: f32 = 10.0;

// --- Port geometry ---

/// Vertical spacing between ports on a node
pub const PORT_SPACING: f32 = 20.0;
/// Radius of port semi-circles
pub const PORT_RADIUS: f32 = 5.0;

// --- Node shape ---

/// Corner radius for rounded rectangles
pub const CORNER_RADIUS: f32 = 10.0;
/// Header height (title + source label area)
pub const HEADER_HEIGHT: f32 = 50.0;

// --- Text sizes ---

/// Font size for node title labels
pub const TITLE_FONT_SIZE: f32 = 16.0;
/// Font size for source path labels
pub const SOURCE_FONT_SIZE: f32 = 12.0;
/// Font size for port labels
pub const PORT_FONT_SIZE: f32 = 11.0;

// --- Edge geometry ---

/// Arrow head size in pixels
pub const ARROW_SIZE: f32 = 10.0;
/// Edge stroke width
pub const STROKE_WIDTH: f32 = 2.0;

/// Padding between internal nodes and the boundary box
pub const BOUNDARY_PADDING: f32 = 120.0;
/// Vertical spacing between boundary ports
pub const BOUNDARY_PORT_SPACING: f32 = 40.0;
/// Fill color for the boundary box
pub const COLOR_BOUNDARY_FILL: &str = "#F0F0F0";
/// Border color for the boundary box
pub const COLOR_BOUNDARY_BORDER: &str = "#CCCCCC";
/// Text color for boundary port labels (dark for light background)
pub const COLOR_BOUNDARY_TEXT: &str = "#444444";

// --- Colors (hex strings) ---

/// Node border color
pub const COLOR_BORDER: &str = "#444444";
/// Edge/connection color
pub const COLOR_EDGE: &str = "#808080";
/// Initializer edge color
pub const COLOR_INITIALIZER: &str = "#4488CC";
/// Port fill color (white)
pub const COLOR_PORT: &str = "#FFFFFF";
/// Text color on colored node backgrounds
pub const COLOR_TEXT: &str = "#FFFFFF";
/// Initializer port highlight color (yellow)
pub const COLOR_INITIALIZER_PORT: &str = "#E6D94D";

/// Classification of a node for visual styling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeCategory {
    /// A subflow (contains other processes)
    Subflow,
    /// A library function (referenced via `lib://`)
    Library,
    /// A context function (referenced via `context://`)
    Context,
    /// A custom/local function
    Custom,
}

impl NodeCategory {
    /// Hex color for this node category.
    #[must_use]
    pub fn color_hex(&self) -> &'static str {
        match self {
            Self::Subflow => "#E69933",
            Self::Library => "#4D80E6",
            Self::Context => "#4DBF73",
            Self::Custom => "#994DCC",
        }
    }

    /// RGB color components (0.0–1.0) for this node category.
    #[must_use]
    pub fn color_rgb(&self) -> (f32, f32, f32) {
        match self {
            Self::Subflow => (0.9, 0.6, 0.2),
            Self::Library => (0.3, 0.5, 0.9),
            Self::Context => (0.3, 0.75, 0.45),
            Self::Custom => (0.6, 0.3, 0.8),
        }
    }

    /// Classify a process into a node category.
    #[must_use]
    pub fn classify(process: Option<&Process>, source: &str) -> Self {
        match process {
            Some(Process::FlowProcess(_)) => Self::Subflow,
            Some(Process::FunctionProcess(f)) => {
                if f.get_lib_reference().is_some() {
                    Self::Library
                } else if f.get_context_reference().is_some() {
                    Self::Context
                } else {
                    Self::Custom
                }
            }
            None => Self::classify_from_source(source),
        }
    }

    fn classify_from_source(source: &str) -> Self {
        if source.starts_with("lib://") {
            Self::Library
        } else if source.starts_with("context://") {
            Self::Context
        } else {
            Self::Subflow
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::indexing_slicing
)]
mod test {
    use super::*;
    use crate::model::flow_definition::FlowDefinition;
    use crate::model::function_definition::FunctionDefinition;
    use url::Url;

    fn lib_function() -> FunctionDefinition {
        let mut f = FunctionDefinition::default();
        f.lib_reference = Some(Url::parse("lib://flowstdlib/math/add").expect("valid url"));
        f
    }

    fn context_function() -> FunctionDefinition {
        let mut f = FunctionDefinition::default();
        f.context_reference = Some(Url::parse("context://stdio/stdout").expect("valid url"));
        f
    }

    #[test]
    fn classify_flow_process() {
        let flow = Process::FlowProcess(FlowDefinition::default());
        assert_eq!(
            NodeCategory::classify(Some(&flow), ""),
            NodeCategory::Subflow
        );
    }

    #[test]
    fn classify_library_function() {
        let func = Process::FunctionProcess(lib_function());
        assert_eq!(
            NodeCategory::classify(Some(&func), ""),
            NodeCategory::Library
        );
    }

    #[test]
    fn classify_context_function() {
        let func = Process::FunctionProcess(context_function());
        assert_eq!(
            NodeCategory::classify(Some(&func), ""),
            NodeCategory::Context
        );
    }

    #[test]
    fn classify_custom_function() {
        let func = Process::FunctionProcess(FunctionDefinition::default());
        assert_eq!(
            NodeCategory::classify(Some(&func), ""),
            NodeCategory::Custom
        );
    }

    #[test]
    fn classify_none_with_lib_source() {
        assert_eq!(
            NodeCategory::classify(None, "lib://flowstdlib/math/add"),
            NodeCategory::Library
        );
    }

    #[test]
    fn classify_none_with_context_source() {
        assert_eq!(
            NodeCategory::classify(None, "context://stdio/stdout"),
            NodeCategory::Context
        );
    }

    #[test]
    fn classify_none_with_unknown_source() {
        assert_eq!(
            NodeCategory::classify(None, "my_flow.toml"),
            NodeCategory::Subflow
        );
    }

    #[test]
    fn all_categories_have_distinct_colors() {
        let categories = [
            NodeCategory::Subflow,
            NodeCategory::Library,
            NodeCategory::Context,
            NodeCategory::Custom,
        ];
        for (i, a) in categories.iter().enumerate() {
            for b in &categories[i + 1..] {
                assert_ne!(a.color_hex(), b.color_hex());
                assert_ne!(a.color_rgb(), b.color_rgb());
            }
        }
    }

    #[test]
    fn color_hex_starts_with_hash() {
        for cat in [
            NodeCategory::Subflow,
            NodeCategory::Library,
            NodeCategory::Context,
            NodeCategory::Custom,
        ] {
            assert!(cat.color_hex().starts_with('#'));
            assert_eq!(cat.color_hex().len(), 7);
        }
    }

    #[test]
    fn color_rgb_in_range() {
        for cat in [
            NodeCategory::Subflow,
            NodeCategory::Library,
            NodeCategory::Context,
            NodeCategory::Custom,
        ] {
            let (r, g, b) = cat.color_rgb();
            assert!((0.0..=1.0).contains(&r));
            assert!((0.0..=1.0).contains(&g));
            assert!((0.0..=1.0).contains(&b));
        }
    }
}
