//! SVG shape primitives for graph rendering.

use svg::node::element::{Anchor, Circle, Group, Path, Rectangle, Text, Title};

use super::style;

/// Rounded rectangle node (for sub-flows and pure functions).
#[must_use]
pub fn rounded_rect(
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    fill: &str,
    stroke: &str,
) -> Rectangle {
    Rectangle::new()
        .set("x", left)
        .set("y", top)
        .set("width", width)
        .set("height", height)
        .set("rx", style::CORNER_RADIUS)
        .set("ry", style::CORNER_RADIUS)
        .set("fill", fill)
        .set("stroke", stroke)
        .set("stroke-width", 1.5)
}

/// House shape (pentagon pointing up) for sink nodes.
#[must_use]
pub fn house(left: f32, top: f32, width: f32, height: f32, fill: &str, stroke: &str) -> Path {
    let mid_x = left + width / 2.0;
    let roof_y = top + height * 0.3;
    let bottom = top + height;
    let right = left + width;

    Path::new()
        .set(
            "d",
            format!("M {mid_x} {top} L {right} {roof_y} L {right} {bottom} L {left} {bottom} L {left} {roof_y} Z"),
        )
        .set("fill", fill)
        .set("stroke", stroke)
        .set("stroke-width", 1.5)
}

/// Inverted house shape (pentagon pointing down) for source nodes.
#[must_use]
pub fn inverted_house(
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    fill: &str,
    stroke: &str,
) -> Path {
    let mid_x = left + width / 2.0;
    let body_y = top + height * 0.7;
    let bottom = top + height;
    let right = left + width;

    Path::new()
        .set(
            "d",
            format!("M {left} {top} L {right} {top} L {right} {body_y} L {mid_x} {bottom} L {left} {body_y} Z"),
        )
        .set("fill", fill)
        .set("stroke", stroke)
        .set("stroke-width", 1.5)
}

/// Port circle (input or output).
#[must_use]
pub fn port_circle(center_x: f32, center_y: f32) -> Circle {
    Circle::new()
        .set("cx", center_x)
        .set("cy", center_y)
        .set("r", style::PORT_RADIUS)
        .set("fill", style::COLOR_PORT)
        .set("stroke", style::COLOR_BORDER)
        .set("stroke-width", 1)
}

/// Text label centered at a position.
#[must_use]
pub fn centered_text(pos_x: f32, pos_y: f32, label: &str, size: f32, color: &str) -> Text {
    Text::new(label)
        .set("x", pos_x)
        .set("y", pos_y)
        .set("text-anchor", "middle")
        .set("dominant-baseline", "central")
        .set("font-family", "sans-serif")
        .set("font-size", size)
        .set("fill", color)
}

/// Small text label aligned left or right near a port.
#[must_use]
pub fn port_label(pos_x: f32, pos_y: f32, label: &str, anchor: &str, color: &str) -> Text {
    Text::new(label)
        .set("x", pos_x)
        .set("y", pos_y)
        .set("text-anchor", anchor)
        .set("dominant-baseline", "central")
        .set("font-family", "sans-serif")
        .set("font-size", style::PORT_FONT_SIZE)
        .set("fill", color)
}

/// Tooltip element (renders as native browser tooltip on hover).
#[must_use]
pub fn tooltip(text: &str) -> Title {
    Title::new(text)
}

/// Wrap SVG elements in a clickable link.
#[must_use]
pub fn link(href: &str, content: Group) -> Anchor {
    Anchor::new()
        .set("href", href)
        .set("target", "_blank")
        .add(content)
}
