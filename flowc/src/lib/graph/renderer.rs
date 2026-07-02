//! SVG renderer for flow diagrams.
//!
//! Renders flow definitions as interactive SVG documents with clickable
//! navigation, tooltips, and styled nodes/edges.

use std::collections::HashMap;

use svg::node::element::{Group, Style};
use svg::Document;

use flowcore::graph::layout::{connection_label, split_route};
use flowcore::graph::style::NodeCategory;
use flowcore::model::connection::Connection;
use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::name::HasName;
use flowcore::model::process::Process::{FlowProcess, FunctionProcess};
use flowcore::model::process_reference::ProcessReference;

use super::layout::{self, process_alias, PositionedNode};
use super::{edge, shapes, style};

/// Positions of boundary ports (the flow's own input/output ports).
struct BoundaryPorts {
    /// Input port name → (x, y) anchor point (right side, facing inward)
    inputs: HashMap<String, (f32, f32)>,
    /// Output port name → (x, y) anchor point (left side, facing inward)
    outputs: HashMap<String, (f32, f32)>,
}

/// Information about a process needed for rendering.
struct ProcessInfo {
    category: NodeCategory,
    /// (name, type) pairs
    inputs: Vec<(String, String)>,
    /// (name, type) pairs
    outputs: Vec<(String, String)>,
    source: String,
}

/// Render a flow definition to an SVG string.
#[must_use]
pub fn render_flow(flow: &FlowDefinition) -> String {
    let process_info = collect_process_info(flow);
    let node_info = build_node_info(&process_info);

    let layouts = layout::compute_layout(&flow.process_refs, &flow.connections, &node_info);

    let mut svg_group = Group::new().set("class", "flow-graph");

    // Render boundary box first (behind everything) if this flow has inputs/outputs
    let has_boundary = !flow.inputs.is_empty() || !flow.outputs.is_empty();
    let boundary_ports = if has_boundary {
        let (boundary, ports) = compute_and_render_boundary(flow, &layouts);
        svg_group = svg_group.add(boundary);
        Some(ports)
    } else {
        None
    };

    // Render internal nodes (on top of boundary)
    for pr in &flow.process_refs {
        let alias = process_alias(pr);
        if let (Some(layout), Some(info)) = (layouts.get(&alias), process_info.get(&alias)) {
            svg_group = svg_group.add(render_node(layout, info, &alias));
        }
    }

    // Render connections (including boundary connections)
    let mut loopback_counts: HashMap<String, usize> = HashMap::new();
    for conn in &flow.connections {
        svg_group = svg_group.add(render_connection(
            conn,
            &layouts,
            &mut loopback_counts,
            boundary_ports.as_ref(),
        ));
    }

    // Render initializers
    for pr in &flow.process_refs {
        let alias = process_alias(pr);
        if let Some(layout) = layouts.get(&alias) {
            let node_loopbacks = loopback_counts.get(&alias).copied().unwrap_or(0);
            svg_group = svg_group.add(render_initializers(pr, layout, node_loopbacks));
        }
    }

    let has_initializers = flow
        .process_refs
        .iter()
        .any(|pr| !pr.initializations.is_empty());
    let max_loopbacks = loopback_counts.values().copied().max().unwrap_or(0);
    let (vb_x, vb_y, doc_width, doc_height) =
        compute_document_bounds(&layouts, has_initializers, max_loopbacks, has_boundary);

    let document = Document::new()
        .set(
            "viewBox",
            format!(
                "{} {} {} {}",
                vb_x.round(),
                vb_y.round(),
                doc_width.round(),
                doc_height.round()
            ),
        )
        .set("width", doc_width)
        .set("height", doc_height)
        .set("xmlns", "http://www.w3.org/2000/svg")
        .set("xmlns:xlink", "http://www.w3.org/1999/xlink")
        .add(css_styles())
        .add(svg_group);

    document.to_string()
}

fn css_styles() -> Style {
    Style::new(
        ".node-subflow rect { cursor: pointer; }
.node-subflow:hover rect { stroke-width: 3; }
.node:hover rect { stroke-width: 3; }
.port-label { pointer-events: none; }
.edge-line { }
.edge-arrow { }
.edge:hover .edge-line { stroke-width: 3; stroke: #FFD900; }
.edge:hover .edge-arrow { fill: #FFD900; }
.edge:hover text { fill: #FFD900; }
text { user-select: none; }",
    )
}

fn io_name_and_type(io: &flowcore::model::io::IO) -> (String, String) {
    let name = io.name().clone();
    let type_str = io
        .datatypes()
        .iter()
        .map(std::string::ToString::to_string)
        .collect::<Vec<_>>()
        .join("|");
    let type_str = if type_str.is_empty() {
        "generic".to_string()
    } else {
        type_str
    };
    (name, type_str)
}

fn collect_process_info(flow: &FlowDefinition) -> HashMap<String, ProcessInfo> {
    let mut info = HashMap::new();

    for pr in &flow.process_refs {
        let alias = process_alias(pr);
        let process = flow.subprocesses.get(&alias);
        let category = NodeCategory::classify(process, &pr.source);

        match process {
            Some(FlowProcess(sub_flow)) => {
                let inputs: Vec<(String, String)> =
                    sub_flow.inputs.iter().map(io_name_and_type).collect();
                let outputs: Vec<(String, String)> =
                    sub_flow.outputs.iter().map(io_name_and_type).collect();
                info.insert(
                    alias,
                    ProcessInfo {
                        category,
                        inputs,
                        outputs,
                        source: pr.source.clone(),
                    },
                );
            }
            Some(FunctionProcess(func)) => {
                let inputs: Vec<(String, String)> =
                    func.inputs.iter().map(io_name_and_type).collect();
                let outputs: Vec<(String, String)> =
                    func.outputs.iter().map(io_name_and_type).collect();
                info.insert(
                    alias,
                    ProcessInfo {
                        category,
                        inputs,
                        outputs,
                        source: pr.source.clone(),
                    },
                );
            }
            None => {}
        }
    }

    info
}

fn build_node_info(
    process_info: &HashMap<String, ProcessInfo>,
) -> HashMap<String, (Vec<String>, Vec<String>)> {
    process_info
        .iter()
        .map(|(alias, info)| {
            (
                alias.clone(),
                (
                    info.inputs.iter().map(|(n, _)| n.clone()).collect(),
                    info.outputs.iter().map(|(n, _)| n.clone()).collect(),
                ),
            )
        })
        .collect()
}

fn render_node(layout: &PositionedNode, info: &ProcessInfo, alias: &str) -> Group {
    let mut group = Group::new();

    let fill = info.category.color_hex();
    let css_class = if info.category == NodeCategory::Subflow {
        "node node-subflow"
    } else {
        "node"
    };

    group = group.set("class", css_class);

    // All nodes use rounded rectangles
    group = group.add(shapes::rounded_rect(
        layout.x,
        layout.y,
        layout.width,
        layout.height,
        fill,
        style::COLOR_BORDER,
    ));

    // Node title
    group = group.add(shapes::centered_text(
        layout.x + layout.width / 2.0,
        layout.y + 18.0,
        alias,
        style::TITLE_FONT_SIZE,
        style::COLOR_TEXT,
    ));

    // Source label (truncated)
    let source_label = truncate_source(&info.source);
    group = group.add(shapes::centered_text(
        layout.x + layout.width / 2.0,
        layout.y + 36.0,
        &source_label,
        style::SOURCE_FONT_SIZE,
        style::COLOR_TEXT,
    ));

    // Tooltip
    group = group.add(shapes::tooltip(&format!("{} ({})", alias, info.source)));

    // Input ports
    for (i, (port_name, port_type)) in info.inputs.iter().enumerate() {
        let px = layout.input_port_x();
        let py = layout.input_port_y(i);
        let mut port_group = Group::new();
        port_group = port_group.add(shapes::input_port(px, py, style::COLOR_PORT));
        port_group = port_group.add(shapes::tooltip(&format!("{port_name}: {port_type}")));
        group = group.add(port_group);
        group = group.add(shapes::port_label(
            px + style::PORT_RADIUS + 3.0,
            py,
            port_name,
            "start",
            style::COLOR_TEXT,
        ));
    }

    // Output ports
    for (i, (port_name, port_type)) in info.outputs.iter().enumerate() {
        let px = layout.output_port_x();
        let py = layout.output_port_y(i);
        let mut port_group = Group::new();
        port_group = port_group.add(shapes::output_port(px, py, style::COLOR_PORT));
        port_group = port_group.add(shapes::tooltip(&format!("{port_name}: {port_type}")));
        group = group.add(port_group);
        group = group.add(shapes::port_label(
            px - style::PORT_RADIUS - 3.0,
            py,
            port_name,
            "end",
            style::COLOR_TEXT,
        ));
    }

    if info.category == NodeCategory::Subflow {
        let href = format!("{alias}.svg");
        return Group::new().add(shapes::link(&href, group));
    }

    group
}

fn truncate_source(source: &str) -> String {
    let max_len = 22;
    if source.len() <= max_len {
        source.to_string()
    } else {
        format!("...{}", &source[source.len() - max_len + 3..])
    }
}

/// Compute the boundary box geometry and port positions, then render the
/// bounding rectangle and its ports. Returns the SVG group and port positions.
#[allow(clippy::cast_precision_loss)]
fn compute_and_render_boundary(
    flow: &FlowDefinition,
    layouts: &HashMap<String, PositionedNode>,
) -> (Group, BoundaryPorts) {
    // Find extents of all internal nodes
    let mut min_x: f32 = 0.0;
    let mut min_y: f32 = 0.0;
    let mut max_x: f32 = 0.0;
    let mut max_y: f32 = 0.0;

    if let Some(first) = layouts.values().next() {
        min_x = first.x;
        min_y = first.y;
        max_x = first.x + first.width;
        max_y = first.y + first.height;
    }

    for layout in layouts.values() {
        min_x = min_x.min(layout.x);
        min_y = min_y.min(layout.y);
        max_x = max_x.max(layout.x + layout.width);
        max_y = max_y.max(layout.y + layout.height);
    }

    let pad = style::BOUNDARY_PADDING;
    let box_left = min_x - pad;
    let box_top = min_y - pad;
    let box_width = (max_x - min_x) + pad * 2.0;
    let box_height = (max_y - min_y) + pad * 2.0;

    let mut group = Group::new().set("class", "boundary");

    // Draw the bounding rectangle
    group = group.add(shapes::rounded_rect(
        box_left,
        box_top,
        box_width,
        box_height,
        style::COLOR_BOUNDARY_FILL,
        style::COLOR_BOUNDARY_BORDER,
    ));

    // Compute and render input ports on the left inner wall
    let input_count = flow.inputs.len();
    let mut input_ports = HashMap::new();
    for (i, io) in flow.inputs.iter().enumerate() {
        let (port_name, port_type) = io_name_and_type(io);
        let px = box_left;
        let py = boundary_port_y(box_top, box_height, i, input_count);

        // Draw output-style semi-circle (faces right/inward)
        let mut port_group = Group::new();
        port_group = port_group.add(shapes::output_port(px, py, style::COLOR_PORT));
        port_group = port_group.add(shapes::tooltip(&format!("input: {port_name}: {port_type}")));
        group = group.add(port_group);
        group = group.add(shapes::port_label(
            px + style::PORT_RADIUS + 3.0,
            py - style::PORT_RADIUS - 4.0,
            &port_name,
            "start",
            style::COLOR_BOUNDARY_TEXT,
        ));

        // Anchor for connections is on the right side of the port
        input_ports.insert(port_name, (px + style::PORT_RADIUS, py));
    }

    // Compute and render output ports on the right inner wall
    let output_count = flow.outputs.len();
    let mut output_ports = HashMap::new();
    for (i, io) in flow.outputs.iter().enumerate() {
        let (port_name, port_type) = io_name_and_type(io);
        let px = box_left + box_width;
        let py = boundary_port_y(box_top, box_height, i, output_count);

        // Draw input-style semi-circle (faces left/inward)
        let mut port_group = Group::new();
        port_group = port_group.add(shapes::input_port(px, py, style::COLOR_PORT));
        port_group = port_group.add(shapes::tooltip(&format!(
            "output: {port_name}: {port_type}"
        )));
        group = group.add(port_group);
        group = group.add(shapes::port_label(
            px - style::PORT_RADIUS - 3.0,
            py - style::PORT_RADIUS - 4.0,
            &port_name,
            "end",
            style::COLOR_BOUNDARY_TEXT,
        ));

        // Anchor for connections is on the left side of the port
        output_ports.insert(port_name, (px - style::PORT_RADIUS, py));
    }

    (
        group,
        BoundaryPorts {
            inputs: input_ports,
            outputs: output_ports,
        },
    )
}

/// Compute the Y position of a boundary port, centered with fixed spacing.
#[allow(clippy::cast_precision_loss)]
fn boundary_port_y(box_top: f32, box_height: f32, index: usize, count: usize) -> f32 {
    let total = (count.saturating_sub(1)) as f32 * style::BOUNDARY_PORT_SPACING;
    let offset = (box_height - total) / 2.0;
    box_top + offset + index as f32 * style::BOUNDARY_PORT_SPACING
}

fn render_connection(
    conn: &Connection,
    layouts: &HashMap<String, PositionedNode>,
    loopback_counts: &mut HashMap<String, usize>,
    boundary_ports: Option<&BoundaryPorts>,
) -> Group {
    let mut group = Group::new().set("class", "edge");

    let from_route = conn.from().to_string();
    let (from_node, from_port) = split_route(&from_route);

    // Resolve source position: boundary input port or internal node output
    let (x1, y1) = if from_node == "input" {
        let port_base = from_port.split('/').next().unwrap_or(&from_port);
        if let Some((x, y)) = boundary_ports.and_then(|bp| bp.inputs.get(port_base)) {
            (*x, *y)
        } else {
            return group;
        }
    } else {
        let Some(from_layout) = layouts.get(&from_node) else {
            return group;
        };
        let port_base = from_port.split('/').next().unwrap_or(&from_port);
        let from_port_idx = from_layout
            .outputs
            .iter()
            .position(|p| p == port_base)
            .unwrap_or(0);
        (
            from_layout.output_port_x() + style::PORT_RADIUS,
            from_layout.output_port_y(from_port_idx),
        )
    };

    for to_route in conn.to() {
        let to_str = to_route.to_string();
        let (to_node, to_port) = split_route(&to_str);

        // Resolve destination position: boundary output port or internal node input
        let (x2, y2) = if to_node == "output" {
            let to_port_base = to_port.split('/').next().unwrap_or(&to_port);
            if let Some((x, y)) = boundary_ports.and_then(|bp| bp.outputs.get(to_port_base)) {
                (*x, *y)
            } else {
                continue;
            }
        } else {
            let Some(to_layout) = layouts.get(&to_node) else {
                continue;
            };
            let to_port_base = to_port.split('/').next().unwrap_or(&to_port);
            let to_port_idx = to_layout
                .inputs
                .iter()
                .position(|p| p == to_port_base)
                .unwrap_or(0);
            (
                to_layout.input_port_x() - style::PORT_RADIUS,
                to_layout.input_port_y(to_port_idx),
            )
        };

        let tooltip_text = format!("{from_route} → {to_str}");

        let label = connection_label(&from_port, conn.name());
        let label_opt = if label.is_empty() { None } else { Some(label) };

        if from_node == to_node {
            let from_layout = &layouts[&from_node];
            let idx = loopback_counts.entry(from_node.clone()).or_insert(0);
            let loopback_index = *idx;
            *idx += 1;
            group = group.add(edge::loopback_edge(
                x1,
                y1,
                x2,
                y2,
                from_layout.y + from_layout.height,
                loopback_index,
                label_opt.as_deref(),
                Some(&tooltip_text),
                style::COLOR_EDGE,
            ));
        } else {
            group = group.add(edge::labeled_edge(
                x1,
                y1,
                x2,
                y2,
                label_opt.as_deref(),
                Some(&tooltip_text),
                style::COLOR_EDGE,
                None,
            ));
        }
    }

    group
}

fn render_initializers(
    pr: &ProcessReference,
    layout: &PositionedNode,
    node_loopbacks: usize,
) -> Group {
    let mut group = Group::new();

    #[allow(clippy::cast_precision_loss)]
    let loopback_clearance = node_loopbacks as f32 * 25.0;

    for (input_path, initializer) in &pr.initializations {
        let port_name = input_path.rsplit('/').next().unwrap_or(input_path);
        let port_idx = layout
            .inputs
            .iter()
            .position(|p| p == port_name)
            .unwrap_or(0);

        let px = layout.input_port_x() - style::PORT_RADIUS;
        let py = layout.input_port_y(port_idx);

        let value_str = format!("{}", initializer.get_value());
        let truncated = if value_str.len() > 12 {
            format!("{}...", &value_str[..12])
        } else {
            value_str.clone()
        };

        let init_type = match initializer {
            flowcore::model::input::InputInitializer::Always(_) => "always",
            flowcore::model::input::InputInitializer::Once(_) => "once",
        };
        let label = format!("{truncated} ({init_type})");

        let init_offset_y = 20.0 + loopback_clearance;
        let label_x = px - 70.0 - loopback_clearance;
        let label_y = py - init_offset_y;

        let dash = match initializer {
            flowcore::model::input::InputInitializer::Always(_) => None,
            flowcore::model::input::InputInitializer::Once(_) => Some("4 2"),
        };

        let mut init_group = Group::new().set("class", "edge");
        init_group = init_group.add(shapes::centered_text(
            label_x,
            label_y,
            &label,
            style::PORT_FONT_SIZE + 1.0,
            style::COLOR_INITIALIZER,
        ));
        init_group = init_group.add(edge::initializer_edge(
            label_x + 35.0,
            label_y,
            px,
            py,
            style::COLOR_INITIALIZER,
            dash,
        ));
        init_group = init_group.add(shapes::tooltip(&format!("{input_path}: {value_str}")));
        group = group.add(init_group);
    }

    group
}

fn compute_document_bounds(
    layouts: &HashMap<String, PositionedNode>,
    has_initializers: bool,
    max_loopbacks: usize,
    has_boundary: bool,
) -> (f32, f32, f32, f32) {
    let mut min_x: f32 = 0.0;
    let mut min_y: f32 = 0.0;
    let mut max_x: f32 = 0.0;
    let mut max_y: f32 = 0.0;

    for layout in layouts.values() {
        min_x = min_x.min(layout.x);
        min_y = min_y.min(layout.y);
        max_x = max_x.max(layout.x + layout.width);
        max_y = max_y.max(layout.y + layout.height);
    }

    #[allow(clippy::cast_precision_loss)]
    let loopback_extra = if max_loopbacks > 0 {
        max_loopbacks as f32 * 25.0 + 25.0
    } else {
        0.0
    };

    let boundary_extra = if has_boundary {
        style::BOUNDARY_PADDING
    } else {
        0.0
    };

    let left_pad = if has_initializers { 80.0 } else { 0.0 };
    let origin_x = min_x - left_pad - loopback_extra - boundary_extra;
    let origin_y = min_y - style::GRID_ORIGIN_Y - boundary_extra;
    let width = max_x - origin_x + style::GRID_ORIGIN_X + loopback_extra + boundary_extra;
    let height = max_y - origin_y + style::GRID_ORIGIN_Y + loopback_extra + boundary_extra;

    (origin_x, origin_y, width, height)
}
