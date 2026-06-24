//! SVG renderer for flow diagrams.
//!
//! Renders flow definitions as interactive SVG documents with clickable
//! navigation, tooltips, and styled nodes/edges.

use std::collections::HashMap;

use svg::node::element::{Group, Style};
use svg::Document;

use flowcore::graph::layout::split_route;
use flowcore::graph::style::NodeCategory;
use flowcore::model::connection::Connection;
use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::name::HasName;
use flowcore::model::process::Process::{FlowProcess, FunctionProcess};
use flowcore::model::process_reference::ProcessReference;

use super::layout::{self, process_alias, PositionedNode};
use super::{edge, shapes, style};

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

    for pr in &flow.process_refs {
        let alias = process_alias(pr);
        if let (Some(layout), Some(info)) = (layouts.get(&alias), process_info.get(&alias)) {
            svg_group = svg_group.add(render_node(layout, info, &alias));
        }
    }

    for conn in &flow.connections {
        svg_group = svg_group.add(render_connection(conn, &layouts));
    }

    for pr in &flow.process_refs {
        let alias = process_alias(pr);
        if let Some(layout) = layouts.get(&alias) {
            svg_group = svg_group.add(render_initializers(pr, layout));
        }
    }

    let has_initializers = flow
        .process_refs
        .iter()
        .any(|pr| !pr.initializations.is_empty());
    let (vb_x, vb_y, doc_width, doc_height) = compute_document_bounds(&layouts, has_initializers);

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

fn render_connection(conn: &Connection, layouts: &HashMap<String, PositionedNode>) -> Group {
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

fn render_initializers(pr: &ProcessReference, layout: &PositionedNode) -> Group {
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

        let init_type = match initializer {
            flowcore::model::input::InputInitializer::Always(_) => "always",
            flowcore::model::input::InputInitializer::Once(_) => "once",
        };
        let label = format!("{truncated} ({init_type})");

        let label_x = px - 70.0;
        let label_y = py;

        group = group.add(shapes::centered_text(
            label_x,
            label_y,
            &label,
            style::PORT_FONT_SIZE + 1.0,
            style::COLOR_INITIALIZER,
        ));

        let dash = match initializer {
            flowcore::model::input::InputInitializer::Always(_) => None,
            flowcore::model::input::InputInitializer::Once(_) => Some("4 2"),
        };

        group = group.add(edge::bezier_edge(
            label_x + 35.0,
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

fn compute_document_bounds(
    layouts: &HashMap<String, PositionedNode>,
    has_initializers: bool,
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

    let left_pad = if has_initializers { 80.0 } else { 0.0 };
    let origin_x = min_x - left_pad;
    let origin_y = min_y - style::GRID_ORIGIN_Y;
    let width = max_x - origin_x + style::GRID_ORIGIN_X;
    let height = max_y - origin_y + style::GRID_ORIGIN_Y;

    (origin_x, origin_y, width, height)
}
