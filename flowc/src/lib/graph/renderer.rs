//! SVG renderer for flow diagrams.
//!
//! Renders flow definitions as interactive SVG documents with clickable
//! navigation, tooltips, and styled nodes/edges.

use std::collections::HashMap;

use svg::node::element::{Group, Style};
use svg::Document;

use flowcore::model::connection::Connection;
use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::name::HasName;
use flowcore::model::process::Process::{FlowProcess, FunctionProcess};
use flowcore::model::process_reference::ProcessReference;

use super::layout::{self, process_alias, split_route, NodeLayout};
use super::{edge, shapes, style};

/// Information about a process needed for rendering.
struct ProcessInfo {
    is_subflow: bool,
    is_impure: bool,
    has_inputs: bool,
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

    // Render nodes
    for pr in &flow.process_refs {
        let alias = process_alias(pr);
        if let (Some(layout), Some(info)) = (layouts.get(&alias), process_info.get(&alias)) {
            svg_group = svg_group.add(render_node(layout, info, &alias));
        }
    }

    // Render connections
    for conn in &flow.connections {
        svg_group = svg_group.add(render_connection(conn, &layouts));
    }

    // Render initializers
    for pr in &flow.process_refs {
        let alias = process_alias(pr);
        if let Some(layout) = layouts.get(&alias) {
            svg_group = svg_group.add(render_initializers(pr, layout));
        }
    }

    let (doc_width, doc_height) = compute_document_size(&layouts);

    let document = Document::new()
        .set(
            "viewBox",
            format!("0 0 {} {}", doc_width.round(), doc_height.round()),
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
.node-function:hover rect, .node-function:hover path { stroke-width: 3; }
.port-label { pointer-events: none; }
.edge path { transition: stroke-width 0.15s; }
.edge:hover path { stroke-width: 3; }
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

        if let Some(process) = flow.subprocesses.get(&alias) {
            match process {
                FlowProcess(sub_flow) => {
                    let inputs: Vec<(String, String)> =
                        sub_flow.inputs.iter().map(io_name_and_type).collect();
                    let outputs: Vec<(String, String)> =
                        sub_flow.outputs.iter().map(io_name_and_type).collect();
                    info.insert(
                        alias,
                        ProcessInfo {
                            is_subflow: true,
                            is_impure: false,
                            has_inputs: !inputs.is_empty(),
                            inputs,
                            outputs,
                            source: pr.source.clone(),
                        },
                    );
                }
                FunctionProcess(func) => {
                    let inputs: Vec<(String, String)> =
                        func.inputs.iter().map(io_name_and_type).collect();
                    let outputs: Vec<(String, String)> =
                        func.outputs.iter().map(io_name_and_type).collect();
                    info.insert(
                        alias,
                        ProcessInfo {
                            is_subflow: false,
                            is_impure: func.is_impure(),
                            has_inputs: !inputs.is_empty(),
                            inputs,
                            outputs,
                            source: pr.source.clone(),
                        },
                    );
                }
            }
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

fn render_node(layout: &NodeLayout, info: &ProcessInfo, alias: &str) -> Group {
    let mut group = Group::new();

    let (fill, text_color, css_class) = if info.is_subflow {
        (style::COLOR_SUBFLOW, style::COLOR_TEXT, "node-subflow")
    } else if info.is_impure && !info.has_inputs {
        (style::COLOR_SOURCE, style::COLOR_TEXT, "node-source")
    } else if info.is_impure {
        (style::COLOR_SINK, style::COLOR_TEXT_LIGHT, "node-sink")
    } else {
        (style::COLOR_FUNCTION, style::COLOR_TEXT, "node-function")
    };

    group = group.set("class", css_class);

    // Node shape
    if info.is_impure && !info.has_inputs {
        group = group.add(shapes::inverted_house(
            layout.x,
            layout.y,
            layout.width,
            layout.height,
            fill,
            style::COLOR_BORDER,
        ));
    } else if info.is_impure && info.has_inputs {
        group = group.add(shapes::house(
            layout.x,
            layout.y,
            layout.width,
            layout.height,
            fill,
            style::COLOR_BORDER,
        ));
    } else {
        group = group.add(shapes::rounded_rect(
            layout.x,
            layout.y,
            layout.width,
            layout.height,
            fill,
            style::COLOR_BORDER,
        ));
    }

    // Node title
    group = group.add(shapes::centered_text(
        layout.x + layout.width / 2.0,
        layout.y + style::HEADER_HEIGHT / 2.0,
        alias,
        style::FONT_SIZE,
        text_color,
    ));

    // Tooltip
    group = group.add(shapes::tooltip(&format!("{} ({})", alias, info.source)));

    // Input ports with type tooltips
    for (i, (port_name, port_type)) in info.inputs.iter().enumerate() {
        let px = layout.input_port_x();
        let py = layout.input_port_y(i);
        let mut port_group = Group::new();
        port_group = port_group.add(shapes::port_circle(px, py));
        port_group = port_group.add(shapes::tooltip(&format!("{port_name}: {port_type}")));
        group = group.add(port_group);
        group = group.add(shapes::port_label(
            px + style::PORT_RADIUS + 3.0,
            py,
            port_name,
            "start",
            text_color,
        ));
    }

    // Output ports with type tooltips
    for (i, (port_name, port_type)) in info.outputs.iter().enumerate() {
        let px = layout.output_port_x();
        let py = layout.output_port_y(i);
        let mut port_group = Group::new();
        port_group = port_group.add(shapes::port_circle(px, py));
        port_group = port_group.add(shapes::tooltip(&format!("{port_name}: {port_type}")));
        group = group.add(port_group);
        group = group.add(shapes::port_label(
            px - style::PORT_RADIUS - 3.0,
            py,
            port_name,
            "end",
            text_color,
        ));
    }

    if info.is_subflow {
        let href = format!("{alias}.svg");
        return Group::new().add(shapes::link(&href, group));
    }

    group
}

fn render_connection(conn: &Connection, layouts: &HashMap<String, NodeLayout>) -> Group {
    let mut group = Group::new().set("class", "edge");

    let from_route = conn.from().to_string();
    let (from_node, from_port) = split_route(&from_route);

    let Some(from_layout) = layouts.get(&from_node) else {
        return group;
    };

    let from_port_idx = from_layout
        .outputs
        .iter()
        .position(|p| p == &from_port)
        .unwrap_or(0);

    let x1 = from_layout.output_port_x();
    let y1 = from_layout.output_port_y(from_port_idx);

    for to_route in conn.to() {
        let to_str = to_route.to_string();
        let (to_node, to_port) = split_route(&to_str);

        let Some(to_layout) = layouts.get(&to_node) else {
            continue;
        };

        let to_port_idx = to_layout
            .inputs
            .iter()
            .position(|p| p == &to_port)
            .unwrap_or(0);

        let x2 = to_layout.input_port_x();
        let y2 = to_layout.input_port_y(to_port_idx);

        let tooltip_text = format!("{from_route} → {to_str}");

        if from_node == to_node {
            // Loopback
            group = group.add(edge::loopback_edge(
                x1,
                y1,
                x2,
                y2,
                from_layout.y + from_layout.height,
                style::COLOR_EDGE,
            ));
        } else {
            group = group.add(edge::labeled_edge(
                x1,
                y1,
                x2,
                y2,
                if conn.name().is_empty() {
                    None
                } else {
                    Some(conn.name().as_str())
                },
                Some(&tooltip_text),
                style::COLOR_EDGE,
                None,
            ));
        }
    }

    group
}

fn render_initializers(pr: &ProcessReference, layout: &NodeLayout) -> Group {
    let mut group = Group::new();

    for (input_path, initializer) in &pr.initializations {
        let port_name = input_path.rsplit('/').next().unwrap_or(input_path);
        let port_idx = layout
            .inputs
            .iter()
            .position(|p| p == port_name)
            .unwrap_or(0);

        let px = layout.input_port_x();
        let py = layout.input_port_y(port_idx);

        let value_str = format!("{}", initializer.get_value());
        let truncated = if value_str.len() > 12 {
            format!("{}...", &value_str[..12])
        } else {
            value_str.clone()
        };

        let label_x = px - 60.0;
        let label_y = py;

        group = group.add(shapes::centered_text(
            label_x,
            label_y,
            &truncated,
            style::PORT_FONT_SIZE,
            style::COLOR_INITIALIZER,
        ));

        let dash = match initializer {
            flowcore::model::input::InputInitializer::Always(_) => None,
            flowcore::model::input::InputInitializer::Once(_) => Some("4 2"),
        };

        group = group.add(edge::bezier_edge(
            label_x + 25.0,
            label_y,
            px,
            py,
            style::COLOR_INITIALIZER,
            dash,
        ));

        group = group.add(shapes::tooltip(&format!("{input_path}: {value_str}")));
    }

    group
}

fn compute_document_size(layouts: &HashMap<String, NodeLayout>) -> (f32, f32) {
    let mut max_x: f32 = 0.0;
    let mut max_y: f32 = 0.0;

    for layout in layouts.values() {
        max_x = max_x.max(layout.x + layout.width);
        max_y = max_y.max(layout.y + layout.height);
    }

    (
        max_x + style::GRID_ORIGIN_X * 2.0,
        max_y + style::GRID_ORIGIN_Y * 2.0,
    )
}
