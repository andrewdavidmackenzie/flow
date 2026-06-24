//! SVG shape primitives for graph rendering.

use svg::node::element::{Anchor, Group, Path, Rectangle, Text, Title};

use super::style;

/// Rounded rectangle node shape.
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
        .set("stroke-width", style::STROKE_WIDTH)
}

/// Port semi-circle on the left edge (input port).
#[must_use]
pub fn input_port(center_x: f32, center_y: f32, fill: &str) -> Path {
    let r = style::PORT_RADIUS;
    let top_y = center_y - r;
    let bot_y = center_y + r;
    Path::new()
        .set(
            "d",
            format!("M {center_x} {top_y} A {r} {r} 0 0 0 {center_x} {bot_y} Z"),
        )
        .set("fill", fill)
        .set("stroke", style::COLOR_BORDER)
        .set("stroke-width", 1)
}

/// Port semi-circle on the right edge (output port).
#[must_use]
pub fn output_port(center_x: f32, center_y: f32, fill: &str) -> Path {
    let r = style::PORT_RADIUS;
    let top_y = center_y - r;
    let bot_y = center_y + r;
    Path::new()
        .set(
            "d",
            format!("M {center_x} {top_y} A {r} {r} 0 0 1 {center_x} {bot_y} Z"),
        )
        .set("fill", fill)
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
