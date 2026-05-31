//! Edge rendering with cubic Bézier curves and arrow heads.

use svg::node::element::{Group, Path};

use crate::shapes;
use crate::style;

/// Render a Bézier edge from (x1,y1) to (x2,y2) with an arrow head.
#[must_use]
pub fn bezier_edge(x1: f32, y1: f32, x2: f32, y2: f32, color: &str, dash: Option<&str>) -> Group {
    let dx = (x2 - x1).abs();
    let offset = (dx / 3.0).max(40.0);

    let cx1 = x1 + offset;
    let cy1 = y1;
    let cx2 = x2 - offset;
    let cy2 = y2;

    let d = format!("M {x1} {y1} C {cx1} {cy1}, {cx2} {cy2}, {x2} {y2}");

    let mut path = Path::new()
        .set("d", d)
        .set("fill", "none")
        .set("stroke", color)
        .set("stroke-width", 1.5);

    if let Some(pattern) = dash {
        path = path.set("stroke-dasharray", pattern);
    }

    let mut group = Group::new().add(path);
    group = group.add(arrow_head(x2, y2, cx2, cy2, color));
    group
}

/// Render a loopback edge that curves below/above the node.
#[must_use]
pub fn loopback_edge(
    x_out: f32,
    y_out: f32,
    x_in: f32,
    y_in: f32,
    node_bottom: f32,
    color: &str,
) -> Group {
    let loop_y = node_bottom + 30.0;

    let d = format!(
        "M {x_out} {y_out} C {cx1} {loop_y}, {cx2} {loop_y}, {x_in} {y_in}",
        cx1 = x_out + 30.0,
        cx2 = x_in - 30.0,
    );

    let path = Path::new()
        .set("d", d)
        .set("fill", "none")
        .set("stroke", color)
        .set("stroke-width", 1.5);

    let mut group = Group::new().add(path);
    group = group.add(arrow_head(x_in, y_in, x_in - 30.0, loop_y, color));
    group
}

/// Arrow head pointing toward (x, y) from direction (from_x, from_y).
fn arrow_head(x: f32, y: f32, from_x: f32, from_y: f32, color: &str) -> Path {
    let angle = (y - from_y).atan2(x - from_x);
    let s = style::ARROW_SIZE;

    let x1 = x - s * (angle - 0.4).cos();
    let y1 = y - s * (angle - 0.4).sin();
    let x2 = x - s * (angle + 0.4).cos();
    let y2 = y - s * (angle + 0.4).sin();

    let d = format!("M {x} {y} L {x1} {y1} L {x2} {y2} Z");

    Path::new()
        .set("d", d)
        .set("fill", color)
        .set("stroke", "none")
}

/// Render an edge with an optional label at the midpoint.
#[must_use]
pub fn labeled_edge(
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    label: Option<&str>,
    tooltip_text: Option<&str>,
    color: &str,
    dash: Option<&str>,
) -> Group {
    let mut group = bezier_edge(x1, y1, x2, y2, color, dash);

    if let Some(text) = label {
        let mid_x = (x1 + x2) / 2.0;
        let mid_y = (y1 + y2) / 2.0 - 8.0;
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
