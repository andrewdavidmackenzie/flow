//! Topological graph layout algorithm shared between flowc and flowedit.
//!
//! Computes node positions using BFS-based column assignment and vertical stacking.
//! Framework-agnostic — works with simple node specs, not rendering types.

#![allow(clippy::cast_precision_loss, clippy::implicit_hasher)]

use std::collections::{HashMap, HashSet, VecDeque};

use super::style;

/// A positioned node returned by [`compute_layout`].
#[derive(Debug, Clone)]
pub struct PositionedNode {
    /// Node identifier (alias or derived short name)
    pub alias: String,
    /// X position
    pub x: f32,
    /// Y position
    pub y: f32,
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
    /// Column index (depth from source nodes)
    pub column: usize,
    /// Input port names
    pub inputs: Vec<String>,
    /// Output port names
    pub outputs: Vec<String>,
}

impl PositionedNode {
    /// Y position of the nth input port.
    #[must_use]
    pub fn input_port_y(&self, index: usize) -> f32 {
        let count = self.inputs.len().max(1);
        port_y(self.y, self.height, index, count)
    }

    /// Y position of the nth output port.
    #[must_use]
    pub fn output_port_y(&self, index: usize) -> f32 {
        let count = self.outputs.len().max(1);
        port_y(self.y, self.height, index, count)
    }

    /// X position of input ports (left edge).
    #[must_use]
    pub fn input_port_x(&self) -> f32 {
        self.x
    }

    /// X position of output ports (right edge).
    #[must_use]
    pub fn output_port_x(&self) -> f32 {
        self.x + self.width
    }
}

/// Compute the Y position of a port given node geometry.
fn port_y(node_y: f32, node_height: f32, index: usize, count: usize) -> f32 {
    let ports_height = (count.saturating_sub(1)) as f32 * style::PORT_SPACING;
    let available = node_height - style::HEADER_HEIGHT;
    let padding = ((available - ports_height) / 2.0).max(0.0);
    node_y + style::HEADER_HEIGHT + padding + index as f32 * style::PORT_SPACING
}

/// Compute the required height for a node based on its port count.
#[must_use]
pub fn compute_node_height(input_count: usize, output_count: usize) -> f32 {
    let max_ports = input_count.max(output_count).max(1);
    let ports_height = (max_ports - 1) as f32 * style::PORT_SPACING;
    (style::HEADER_HEIGHT + ports_height + style::NODE_PADDING * 2.0).max(style::NODE_HEIGHT)
}

/// Compute the Bézier control point horizontal offset for a connection.
#[must_use]
pub fn bezier_control_offset(dx: f32) -> f32 {
    dx.abs().max(60.0) * 0.5
}

/// Extract a short name from a source path (e.g., `lib://flowstdlib/math/add` → `add`).
#[must_use]
pub fn derive_short_name(source: &str) -> String {
    source.rsplit('/').next().unwrap_or(source).to_string()
}

/// Split a route like `"sequence/number"` into `("sequence", "number")`.
#[must_use]
pub fn split_route(route: &str) -> (String, String) {
    let route = route.trim_start_matches('/');
    if let Some(pos) = route.find('/') {
        (route[..pos].to_string(), route[pos + 1..].to_string())
    } else {
        (route.to_string(), String::new())
    }
}

/// Build a label for a connection from its output port name and connection name.
/// Simple port names (already visible on the node) are omitted; only sub-routes
/// (containing `/`) are included.
#[must_use]
pub fn connection_label(from_port: &str, conn_name: &str) -> String {
    let is_subroute = from_port.contains('/');
    match (is_subroute, from_port, conn_name) {
        (_, "", "") => String::new(),
        (_, "", name) => name.to_string(),
        (true, port, "") => format!("/{port}"),
        (true, port, name) => format!("{name} /{port}"),
        (false, _, "") => String::new(),
        (false, _, name) => name.to_string(),
    }
}

/// Sort nodes within each column by the median index of their neighbors
/// in adjacent columns, reducing edge crossings (Sugiyama barycenter heuristic).
fn median_ordering(
    columns: &mut HashMap<usize, Vec<String>>,
    max_col: usize,
    incoming: &HashMap<String, Vec<String>>,
    outgoing: &HashMap<String, Vec<String>>,
) {
    for _pass in 0..3 {
        // Forward pass: sort each column by median position of incoming neighbors
        for col in 1..=max_col {
            let prev_order: HashMap<String, usize> = columns
                .get(&(col - 1))
                .map(|nodes| {
                    nodes
                        .iter()
                        .enumerate()
                        .map(|(i, a)| (a.clone(), i))
                        .collect()
                })
                .unwrap_or_default();

            if let Some(col_nodes) = columns.get_mut(&col) {
                col_nodes.sort_by(|a, b| {
                    let ma = median_neighbor_pos(a, incoming, &prev_order);
                    let mb = median_neighbor_pos(b, incoming, &prev_order);
                    ma.partial_cmp(&mb).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }

        // Backward pass: sort each column by median position of outgoing neighbors
        for col in (0..max_col).rev() {
            let next_order: HashMap<String, usize> = columns
                .get(&(col + 1))
                .map(|nodes| {
                    nodes
                        .iter()
                        .enumerate()
                        .map(|(i, a)| (a.clone(), i))
                        .collect()
                })
                .unwrap_or_default();

            if let Some(col_nodes) = columns.get_mut(&col) {
                col_nodes.sort_by(|a, b| {
                    let ma = median_neighbor_pos(a, outgoing, &next_order);
                    let mb = median_neighbor_pos(b, outgoing, &next_order);
                    ma.partial_cmp(&mb).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }
    }
}

/// Compute the median position of a node's neighbors in an adjacent column.
fn median_neighbor_pos(
    node: &str,
    adjacency: &HashMap<String, Vec<String>>,
    order: &HashMap<String, usize>,
) -> f32 {
    let mut positions: Vec<usize> = adjacency
        .get(node)
        .map(|neighbors| {
            neighbors
                .iter()
                .filter_map(|n| order.get(n).copied())
                .collect()
        })
        .unwrap_or_default();

    if positions.is_empty() {
        return f32::MAX;
    }

    positions.sort_unstable();
    let mid = positions.len() / 2;
    if positions.len().is_multiple_of(2) {
        let Some(&a) = positions.get(mid.wrapping_sub(1)) else {
            return f32::MAX;
        };
        let Some(&b) = positions.get(mid) else {
            return f32::MAX;
        };
        (a + b) as f32 / 2.0
    } else {
        positions.get(mid).map_or(f32::MAX, |&p| p as f32)
    }
}

/// Compute topological layout for a set of nodes and connections.
///
/// `node_specs` maps alias → (input names, output names).
/// `connections` is a list of `(from_route, to_route)` pairs where routes
/// are in `"node/port"` format.
///
/// Returns a map from alias to [`PositionedNode`] with computed positions.
#[must_use]
pub fn compute_layout(
    node_specs: &[(String, Vec<String>, Vec<String>)],
    connections: &[(String, String)],
) -> HashMap<String, PositionedNode> {
    let aliases: Vec<String> = node_specs
        .iter()
        .map(|(alias, _, _)| alias.clone())
        .collect();
    let alias_set: HashSet<&str> = aliases.iter().map(String::as_str).collect();

    let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    for alias in &aliases {
        incoming.entry(alias.clone()).or_default();
        outgoing.entry(alias.clone()).or_default();
    }

    for (from_route, to_route) in connections {
        let (from_node, _) = split_route(from_route);
        if !alias_set.contains(from_node.as_str()) {
            continue;
        }
        let (to_node, _) = split_route(to_route);
        if from_node != to_node && alias_set.contains(to_node.as_str()) {
            outgoing
                .entry(from_node.clone())
                .or_default()
                .push(to_node.clone());
            incoming.entry(to_node).or_default().push(from_node.clone());
        }
    }

    // BFS column assignment (longest path from sources)
    let mut depth: HashMap<String, usize> = HashMap::new();
    let mut queue = VecDeque::new();
    for alias in &aliases {
        if incoming.get(alias).is_none_or(Vec::is_empty) {
            depth.insert(alias.clone(), 0);
            queue.push_back(alias.clone());
        }
    }

    let max_depth = aliases.len().saturating_sub(1);
    while let Some(node) = queue.pop_front() {
        let node_depth = depth.get(&node).copied().unwrap_or(0);
        if let Some(neighbors) = outgoing.get(&node) {
            for neighbor in neighbors {
                let new_depth = (node_depth + 1).min(max_depth);
                let current = depth.get(neighbor).copied().unwrap_or(0);
                if new_depth > current {
                    depth.insert(neighbor.clone(), new_depth);
                    queue.push_back(neighbor.clone());
                }
            }
        }
    }

    for alias in &aliases {
        depth.entry(alias.clone()).or_insert(0);
    }

    // Group by column
    let mut columns: HashMap<usize, Vec<String>> = HashMap::new();
    for alias in &aliases {
        let col = depth.get(alias).copied().unwrap_or(0);
        columns.entry(col).or_default().push(alias.clone());
    }

    // Build a lookup for port info
    let spec_map: HashMap<&str, (&[String], &[String])> = node_specs
        .iter()
        .map(|(alias, inputs, outputs)| (alias.as_str(), (inputs.as_slice(), outputs.as_slice())))
        .collect();

    // Median ordering: reduce edge crossings by sorting nodes within each column
    // by the median position of their connected neighbors in adjacent columns.
    let max_col = columns.keys().copied().max().unwrap_or(0);
    median_ordering(&mut columns, max_col, &incoming, &outgoing);

    // Compute positions
    let mut layouts = HashMap::new();
    for (col, col_nodes) in &columns {
        let x = style::GRID_ORIGIN_X + *col as f32 * style::GRID_SPACING_X;
        let mut y = style::GRID_ORIGIN_Y;
        for alias in col_nodes {
            let (inputs, outputs) = spec_map.get(alias.as_str()).copied().unwrap_or((&[], &[]));
            let height = compute_node_height(inputs.len(), outputs.len());
            layouts.insert(
                alias.clone(),
                PositionedNode {
                    alias: alias.clone(),
                    x,
                    y,
                    width: style::NODE_WIDTH,
                    height,
                    column: *col,
                    inputs: inputs.to_vec(),
                    outputs: outputs.to_vec(),
                },
            );
            y += height + style::NODE_GAP_Y;
        }
    }

    layouts
}

/// Compute waypoints for a backward edge (feedback connection) that routes
/// below the graph to avoid crossing over nodes.
///
/// `edge_index` offsets multiple backward edges vertically to prevent overlap.
#[must_use]
pub fn backward_edge_waypoints(
    from_x: f32,
    from_y: f32,
    to_x: f32,
    to_y: f32,
    graph_bottom: f32,
    edge_index: usize,
) -> Vec<(f32, f32)> {
    let margin = 20.0;
    let offset = graph_bottom + margin + edge_index as f32 * 15.0;
    vec![
        (from_x, from_y),
        (from_x + 30.0, from_y),
        (from_x + 30.0, offset),
        (to_x - 30.0, offset),
        (to_x - 30.0, to_y),
        (to_x, to_y),
    ]
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod test {
    use super::*;

    #[test]
    fn split_route_with_port() {
        let (node, port) = split_route("sequence/number");
        assert_eq!(node, "sequence");
        assert_eq!(port, "number");
    }

    #[test]
    fn split_route_no_port() {
        let (node, port) = split_route("value");
        assert_eq!(node, "value");
        assert_eq!(port, "");
    }

    #[test]
    fn split_route_leading_slash() {
        let (node, port) = split_route("/add/a");
        assert_eq!(node, "add");
        assert_eq!(port, "a");
    }

    #[test]
    fn derive_short_name_lib_url() {
        assert_eq!(derive_short_name("lib://flowstdlib/math/add"), "add");
    }

    #[test]
    fn derive_short_name_plain() {
        assert_eq!(derive_short_name("my_func"), "my_func");
    }

    #[test]
    fn compute_height_single_port() {
        let h = compute_node_height(1, 1);
        assert!((h - style::NODE_HEIGHT).abs() < 0.01);
    }

    #[test]
    fn compute_height_many_ports() {
        let h = compute_node_height(8, 2);
        assert!(h > style::NODE_HEIGHT);
    }

    #[test]
    fn layout_chain() {
        let specs = vec![
            ("a".into(), vec![], vec!["out".into()]),
            ("b".into(), vec!["in".into()], vec!["out".into()]),
            ("c".into(), vec!["in".into()], vec![]),
        ];
        let conns = vec![
            ("a/out".into(), "b/in".into()),
            ("b/out".into(), "c/in".into()),
        ];
        let result = compute_layout(&specs, &conns);
        assert_eq!(result.len(), 3);
        assert!(result["a"].x < result["b"].x);
        assert!(result["b"].x < result["c"].x);
        assert_eq!(result["a"].column, 0);
        assert_eq!(result["b"].column, 1);
        assert_eq!(result["c"].column, 2);
    }

    #[test]
    fn layout_diamond() {
        let specs = vec![
            ("src".into(), vec![], vec!["out".into()]),
            ("left".into(), vec!["in".into()], vec!["out".into()]),
            ("right".into(), vec!["in".into()], vec!["out".into()]),
            ("sink".into(), vec!["a".into(), "b".into()], vec![]),
        ];
        let conns = vec![
            ("src/out".into(), "left/in".into()),
            ("src/out".into(), "right/in".into()),
            ("left/out".into(), "sink/a".into()),
            ("right/out".into(), "sink/b".into()),
        ];
        let result = compute_layout(&specs, &conns);
        assert_eq!(result["src"].column, 0);
        assert_eq!(result["left"].column, 1);
        assert_eq!(result["right"].column, 1);
        assert_eq!(result["sink"].column, 2);
    }

    #[test]
    fn bezier_offset_small_dx() {
        let offset = bezier_control_offset(20.0);
        assert!((offset - 30.0).abs() < 0.01);
    }

    #[test]
    fn bezier_offset_large_dx() {
        let offset = bezier_control_offset(200.0);
        assert!((offset - 100.0).abs() < 0.01);
    }

    #[test]
    fn backward_waypoints_go_below() {
        let wp = backward_edge_waypoints(300.0, 100.0, 50.0, 80.0, 400.0, 0);
        assert_eq!(wp.len(), 6);
        for &(_, y) in &wp[2..4] {
            assert!(y > 400.0);
        }
    }

    #[test]
    fn backward_waypoints_stack_vertically() {
        let wp0 = backward_edge_waypoints(300.0, 100.0, 50.0, 80.0, 400.0, 0);
        let wp1 = backward_edge_waypoints(300.0, 100.0, 50.0, 80.0, 400.0, 1);
        assert!(wp1[2].1 > wp0[2].1);
    }

    #[test]
    fn layout_isolated_nodes() {
        let specs = vec![
            ("a".into(), vec!["in".into()], vec!["out".into()]),
            ("b".into(), vec!["in".into()], vec!["out".into()]),
        ];
        let conns: Vec<(String, String)> = vec![];
        let result = compute_layout(&specs, &conns);
        assert_eq!(result.len(), 2);
        assert_eq!(result["a"].column, 0);
        assert_eq!(result["b"].column, 0);
        assert!(result["b"].y > result["a"].y);
    }

    #[test]
    fn layout_cycle_does_not_loop_forever() {
        let specs = vec![
            ("a".into(), vec!["in".into()], vec!["out".into()]),
            ("b".into(), vec!["in".into()], vec!["out".into()]),
        ];
        let conns = vec![
            ("a/out".into(), "b/in".into()),
            ("b/out".into(), "a/in".into()),
        ];
        let result = compute_layout(&specs, &conns);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn port_y_positions_centered() {
        let specs = vec![("n".into(), vec!["a".into(), "b".into(), "c".into()], vec![])];
        let result = compute_layout(&specs, &[]);
        let node = &result["n"];
        let y0 = node.input_port_y(0);
        let y1 = node.input_port_y(1);
        let y2 = node.input_port_y(2);
        assert!((y1 - y0 - style::PORT_SPACING).abs() < 0.01);
        assert!((y2 - y1 - style::PORT_SPACING).abs() < 0.01);
        assert!(y0 > node.y + style::HEADER_HEIGHT - 1.0);
    }

    #[test]
    fn output_port_x_is_right_edge() {
        let specs = vec![("n".into(), vec![], vec!["out".into()])];
        let result = compute_layout(&specs, &[]);
        let node = &result["n"];
        assert!((node.output_port_x() - (node.x + node.width)).abs() < 0.01);
    }

    #[test]
    fn input_port_x_is_left_edge() {
        let specs = vec![("n".into(), vec!["in".into()], vec![])];
        let result = compute_layout(&specs, &[]);
        let node = &result["n"];
        assert!((node.input_port_x() - node.x).abs() < 0.01);
    }

    #[test]
    fn node_height_grows_with_ports() {
        let h_few = compute_node_height(2, 1);
        let h_many = compute_node_height(10, 1);
        assert!(h_many > h_few);
    }

    #[test]
    fn bezier_offset_negative_dx() {
        let offset = bezier_control_offset(-200.0);
        assert!((offset - 100.0).abs() < 0.01);
    }

    #[test]
    fn connection_label_empty() {
        assert_eq!(connection_label("", ""), "");
    }

    #[test]
    fn connection_label_name_only() {
        assert_eq!(connection_label("", "feedback-step"), "feedback-step");
    }

    #[test]
    fn connection_label_simple_port_no_name() {
        assert_eq!(connection_label("right-lte", ""), "");
    }

    #[test]
    fn connection_label_simple_port_with_name() {
        assert_eq!(connection_label("i2", "feedback-step"), "feedback-step");
    }

    #[test]
    fn connection_label_subroute_no_name() {
        assert_eq!(connection_label("string/1", ""), "/string/1");
    }

    #[test]
    fn connection_label_subroute_with_name() {
        assert_eq!(connection_label("string/1", "first"), "first /string/1");
    }
}
