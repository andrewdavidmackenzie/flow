//! Shared connection geometry for edges, arrows, and port offsets.
//!
//! Framework-agnostic functions that return plain coordinates.
//! Both SVG and iced renderers call these for consistent results.

#![allow(clippy::cast_precision_loss)]

use super::style;

/// Loopback routing margin around the node.
const LOOPBACK_MARGIN: f32 = 25.0;
/// Corner radius for loopback rounded corners.
const LOOPBACK_CORNER: f32 = 10.0;

/// Offset a port position from the node edge to the semi-circle edge.
///
/// Output ports extend right by `PORT_RADIUS`; input ports extend left.
#[must_use]
pub fn port_edge_point(port_x: f32, port_y: f32, is_output: bool) -> (f32, f32) {
    if is_output {
        (port_x + style::PORT_RADIUS, port_y)
    } else {
        (port_x - style::PORT_RADIUS, port_y)
    }
}

/// Compute cubic Bézier control points for a normal connection.
///
/// Returns `(ctrl_x1, ctrl_y1, ctrl_x2, ctrl_y2)`.
/// The control points create a smooth horizontal-dominant curve.
#[must_use]
pub fn bezier_controls(from_x: f32, from_y: f32, to_x: f32, to_y: f32) -> (f32, f32, f32, f32) {
    let offset = super::layout::bezier_control_offset(to_x - from_x);
    (from_x + offset, from_y, to_x - offset, to_y)
}

/// Compute the 3 vertices of an arrow head triangle.
///
/// The tip is at `(tip_x, tip_y)`, pointing from direction `(from_x, from_y)`.
/// `size` is the arrow length in the caller's coordinate space (use
/// `ARROW_SIZE` for world space, `ARROW_SIZE * zoom` for screen space).
/// Returns `[(tip_x, tip_y), (left_x, left_y), (right_x, right_y)]`.
#[must_use]
pub fn arrow_head_points(
    tip_x: f32,
    tip_y: f32,
    from_x: f32,
    from_y: f32,
    size: f32,
) -> [(f32, f32); 3] {
    let angle = (tip_y - from_y).atan2(tip_x - from_x);
    let half_angle = 0.4;

    let left_x = tip_x - size * (angle - half_angle).cos();
    let left_y = tip_y - size * (angle - half_angle).sin();
    let right_x = tip_x - size * (angle + half_angle).cos();
    let right_y = tip_y - size * (angle + half_angle).sin();

    [(tip_x, tip_y), (left_x, left_y), (right_x, right_y)]
}

/// Waypoint type for loopback path segments.
#[derive(Debug, Clone, Copy)]
pub enum Waypoint {
    /// Move/line to a point.
    Point(f32, f32),
    /// Quadratic curve with control point and end point.
    Corner(f32, f32, f32, f32),
}

/// Compute loopback waypoints routing around a node: right, down, left, up.
///
/// Coordinates are in world space. The path starts at the output port's
/// semi-circle edge and ends at the input port's semi-circle edge.
#[must_use]
pub fn loopback_waypoints(
    out_x: f32,
    out_y: f32,
    in_x: f32,
    in_y: f32,
    node_bottom: f32,
) -> Vec<Waypoint> {
    let r = LOOPBACK_CORNER;
    let right_x = out_x + LOOPBACK_MARGIN;
    let left_x = in_x - LOOPBACK_MARGIN;
    let bottom_y = node_bottom + LOOPBACK_MARGIN;

    vec![
        Waypoint::Point(out_x, out_y),
        Waypoint::Point(right_x - r, out_y),
        Waypoint::Corner(right_x, out_y, right_x, out_y + r),
        Waypoint::Point(right_x, bottom_y - r),
        Waypoint::Corner(right_x, bottom_y, right_x - r, bottom_y),
        Waypoint::Point(left_x + r, bottom_y),
        Waypoint::Corner(left_x, bottom_y, left_x, bottom_y - r),
        Waypoint::Point(left_x, in_y + r),
        Waypoint::Corner(left_x, in_y, left_x + r, in_y),
        Waypoint::Point(in_x, in_y),
    ]
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod test {
    use super::*;

    #[test]
    fn port_edge_output_offsets_right() {
        let (x, y) = port_edge_point(100.0, 50.0, true);
        assert!((x - 105.0).abs() < 0.01);
        assert!((y - 50.0).abs() < 0.01);
    }

    #[test]
    fn port_edge_input_offsets_left() {
        let (x, y) = port_edge_point(100.0, 50.0, false);
        assert!((x - 95.0).abs() < 0.01);
        assert!((y - 50.0).abs() < 0.01);
    }

    #[test]
    fn bezier_controls_horizontal() {
        let (cx1, cy1, cx2, cy2) = bezier_controls(50.0, 100.0, 300.0, 100.0);
        assert!(cx1 > 50.0);
        assert!(cx2 < 300.0);
        assert!((cy1 - 100.0).abs() < 0.01);
        assert!((cy2 - 100.0).abs() < 0.01);
    }

    #[test]
    fn arrow_rightward() {
        let pts = arrow_head_points(100.0, 50.0, 80.0, 50.0, style::ARROW_SIZE);
        assert!((pts[0].0 - 100.0).abs() < 0.01);
        assert!(pts[1].0 < 100.0);
        assert!(pts[2].0 < 100.0);
        assert!((pts[1].1 - pts[2].1).abs() > 1.0);
    }

    #[test]
    fn arrow_leftward() {
        let pts = arrow_head_points(50.0, 100.0, 80.0, 100.0, style::ARROW_SIZE);
        assert!((pts[0].0 - 50.0).abs() < 0.01);
        assert!(pts[1].0 > 50.0);
        assert!(pts[2].0 > 50.0);
    }

    #[test]
    fn loopback_stays_outside_node() {
        let wp = loopback_waypoints(230.0, 135.0, 50.0, 145.0, 170.0);
        for w in &wp {
            match w {
                Waypoint::Point(_, y) | Waypoint::Corner(_, _, _, y) => {
                    assert!(*y >= 135.0 - 1.0, "waypoint above output port");
                }
            }
        }
    }

    #[test]
    fn loopback_starts_at_output_ends_at_input() {
        let wp = loopback_waypoints(230.0, 135.0, 50.0, 145.0, 170.0);
        let first = match wp.first() {
            Some(Waypoint::Point(x, y)) => (*x, *y),
            _ => panic!("first waypoint should be Point"),
        };
        let last = match wp.last() {
            Some(Waypoint::Point(x, y)) => (*x, *y),
            _ => panic!("last waypoint should be Point"),
        };
        assert!((first.0 - 230.0).abs() < 0.01);
        assert!((last.0 - 50.0).abs() < 0.01);
    }
}
