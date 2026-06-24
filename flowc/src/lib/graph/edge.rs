//! Edge rendering with cubic Bézier curves and arrow heads.

#![allow(clippy::cast_precision_loss)]

use std::fmt::Write;

use flowcore::graph::connection::{self, Waypoint};
use svg::node::element::{Group, Path};

use super::shapes;
use super::style;

/// Format an arrow head as an SVG path element.
fn svg_arrow(tip_x: f32, tip_y: f32, from_x: f32, from_y: f32, color: &str) -> Path {
    let [(tx, ty), (lx, ly), (rx, ry)] =
        connection::arrow_head_points(tip_x, tip_y, from_x, from_y, style::ARROW_SIZE);

    Path::new()
        .set("d", format!("M {tx} {ty} L {lx} {ly} L {rx} {ry} Z"))
        .set("fill", color)
        .set("stroke", "none")
}

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
    let (cx1, cy1, cx2, cy2) = connection::bezier_controls(from_x, from_y, to_x, to_y);

    let path_data = format!("M {from_x} {from_y} C {cx1} {cy1}, {cx2} {cy2}, {to_x} {to_y}");

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
        .add(svg_arrow(to_x, to_y, cx2, cy2, color))
}

/// Render a loopback edge that routes around the node: right, down, left, up.
#[must_use]
pub fn loopback_edge(
    out_x: f32,
    out_y: f32,
    in_x: f32,
    in_y: f32,
    node_bottom: f32,
    color: &str,
) -> Group {
    let waypoints = connection::loopback_waypoints(out_x, out_y, in_x, in_y, node_bottom);

    let mut path_data = String::new();
    for (i, wp) in waypoints.iter().enumerate() {
        match wp {
            Waypoint::Point(x, y) => {
                if i == 0 {
                    let _ = write!(path_data, "M {x} {y}");
                } else {
                    let _ = write!(path_data, " L {x} {y}");
                }
            }
            Waypoint::Corner(cx, cy, ex, ey) => {
                let _ = write!(path_data, " Q {cx} {cy} {ex} {ey}");
            }
        }
    }

    let path = Path::new()
        .set("d", path_data)
        .set("fill", "none")
        .set("stroke", color)
        .set("stroke-width", style::STROKE_WIDTH);

    // Arrow from the last corner toward the input port
    let last_corner_x = in_x - 25.0 + 10.0;
    Group::new()
        .add(path)
        .add(svg_arrow(in_x, in_y, last_corner_x, in_y, color))
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
