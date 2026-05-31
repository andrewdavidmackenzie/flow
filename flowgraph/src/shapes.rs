//! SVG shape primitives for graph rendering.

use svg::node::element::{Anchor, Circle, Group, Path, Rectangle, Text, Title};

use crate::style;

/// Rounded rectangle node (for sub-flows and pure functions).
#[must_use]
pub fn rounded_rect(x: f32, y: f32, w: f32, h: f32, fill: &str, stroke: &str) -> Rectangle {
    Rectangle::new()
        .set("x", x)
        .set("y", y)
        .set("width", w)
        .set("height", h)
        .set("rx", style::CORNER_RADIUS)
        .set("ry", style::CORNER_RADIUS)
        .set("fill", fill)
        .set("stroke", stroke)
        .set("stroke-width", 1.5)
}

/// House shape (pentagon pointing up) for sink nodes.
#[must_use]
pub fn house(x: f32, y: f32, w: f32, h: f32, fill: &str, stroke: &str) -> Path {
    let peak_y = y;
    let mid_x = x + w / 2.0;
    let roof_y = y + h * 0.3;
    let bottom_y = y + h;

    let d = format!(
        "M {mid_x} {peak_y} L {right} {roof_y} L {right} {bottom_y} L {left} {bottom_y} L {left} {roof_y} Z",
        left = x,
        right = x + w,
    );

    Path::new()
        .set("d", d)
        .set("fill", fill)
        .set("stroke", stroke)
        .set("stroke-width", 1.5)
}

/// Inverted house shape (pentagon pointing down) for source nodes.
#[must_use]
pub fn inverted_house(x: f32, y: f32, w: f32, h: f32, fill: &str, stroke: &str) -> Path {
    let top_y = y;
    let mid_x = x + w / 2.0;
    let body_y = y + h * 0.7;
    let bottom_y = y + h;

    let d = format!(
        "M {left} {top_y} L {right} {top_y} L {right} {body_y} L {mid_x} {bottom_y} L {left} {body_y} Z",
        left = x,
        right = x + w,
    );

    Path::new()
        .set("d", d)
        .set("fill", fill)
        .set("stroke", stroke)
        .set("stroke-width", 1.5)
}

/// Port circle (input or output).
#[must_use]
pub fn port_circle(cx: f32, cy: f32) -> Circle {
    Circle::new()
        .set("cx", cx)
        .set("cy", cy)
        .set("r", style::PORT_RADIUS)
        .set("fill", style::COLOR_PORT)
        .set("stroke", style::COLOR_BORDER)
        .set("stroke-width", 1)
}

/// Text label centered at a position.
#[must_use]
pub fn centered_text(x: f32, y: f32, label: &str, size: f32, color: &str) -> Text {
    Text::new(label)
        .set("x", x)
        .set("y", y)
        .set("text-anchor", "middle")
        .set("dominant-baseline", "central")
        .set("font-family", "sans-serif")
        .set("font-size", size)
        .set("fill", color)
}

/// Small text label aligned left or right near a port.
#[must_use]
pub fn port_label(x: f32, y: f32, label: &str, anchor: &str) -> Text {
    Text::new(label)
        .set("x", x)
        .set("y", y)
        .set("text-anchor", anchor)
        .set("dominant-baseline", "central")
        .set("font-family", "sans-serif")
        .set("font-size", style::PORT_FONT_SIZE)
        .set("fill", style::COLOR_TEXT)
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
