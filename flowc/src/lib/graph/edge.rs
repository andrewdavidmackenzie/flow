//! Edge rendering with cubic Bézier curves and arrow heads.

#![allow(clippy::cast_precision_loss)]

use flowcore::graph::layout as shared;
use svg::node::element::{Group, Path};

use super::shapes;
use super::style;

/// Render a Bézier edge between two points with an arrow head.
#[must_use]
pub fn bezier_edge(
    from_x: f32,
    from_y: f32,
    to_x: f32,
    to_y: f32,
    color: &str,
    dash: Option<&str>,
) -> Group {
    let dx = to_x - from_x;
    let offset = shared::bezier_control_offset(dx);

    let ctrl_x1 = from_x + offset;
    let ctrl_x2 = to_x - offset;

    let path_data =
        format!("M {from_x} {from_y} C {ctrl_x1} {from_y}, {ctrl_x2} {to_y}, {to_x} {to_y}");

    let mut path = Path::new()
        .set("d", path_data)
        .set("fill", "none")
        .set("stroke", color)
        .set("stroke-width", style::STROKE_WIDTH);

    if let Some(pattern) = dash {
        path = path.set("stroke-dasharray", pattern);
    }

    Group::new()
        .add(path)
        .add(arrow_head(to_x, to_y, ctrl_x2, to_y, color))
}

/// Render a loopback edge that curves below the node.
#[must_use]
pub fn loopback_edge(
    out_x: f32,
    out_y: f32,
    in_x: f32,
    in_y: f32,
    node_bottom: f32,
    color: &str,
) -> Group {
    let loop_y = node_bottom + 30.0;
    let ctrl_x1 = out_x + 30.0;
    let ctrl_x2 = in_x - 30.0;

    let path_data =
        format!("M {out_x} {out_y} C {ctrl_x1} {loop_y}, {ctrl_x2} {loop_y}, {in_x} {in_y}");

    let path = Path::new()
        .set("d", path_data)
        .set("fill", "none")
        .set("stroke", color)
        .set("stroke-width", style::STROKE_WIDTH);

    Group::new()
        .add(path)
        .add(arrow_head(in_x, in_y, ctrl_x2, loop_y, color))
}

/// Arrow head pointing toward `(tip_x, tip_y)` from direction `(from_x, from_y)`.
fn arrow_head(tip_x: f32, tip_y: f32, from_x: f32, from_y: f32, color: &str) -> Path {
    let angle = (tip_y - from_y).atan2(tip_x - from_x);
    let size = style::ARROW_SIZE;

    let left_x = tip_x - size * (angle - 0.4).cos();
    let left_y = tip_y - size * (angle - 0.4).sin();
    let right_x = tip_x - size * (angle + 0.4).cos();
    let right_y = tip_y - size * (angle + 0.4).sin();

    Path::new()
        .set(
            "d",
            format!("M {tip_x} {tip_y} L {left_x} {left_y} L {right_x} {right_y} Z"),
        )
        .set("fill", color)
        .set("stroke", "none")
}

/// Render an edge with an optional label and tooltip.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn labeled_edge(
    from_x: f32,
    from_y: f32,
    to_x: f32,
    to_y: f32,
    label: Option<&str>,
    tooltip_text: Option<&str>,
    color: &str,
    dash: Option<&str>,
) -> Group {
    let mut group = bezier_edge(from_x, from_y, to_x, to_y, color, dash);

    if let Some(text) = label {
        let mid_x = f32::midpoint(from_x, to_x);
        let mid_y = f32::midpoint(from_y, to_y) - 8.0;
        group = group.add(shapes::centered_text(
            mid_x,
            mid_y,
            text,
            style::PORT_FONT_SIZE,
            color,
        ));
    }

    if let Some(tip) = tooltip_text {
        group = group.add(shapes::tooltip(tip));
    }

    group
}
