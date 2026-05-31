//! Topological graph layout for flow diagrams.
//!
//! Computes node positions using BFS-based column assignment and vertical stacking.
//! Adapted from flowedit's `node_layout.rs`.

#![allow(clippy::cast_precision_loss)]

use std::collections::{HashMap, HashSet, VecDeque};

use flowcore::model::connection::Connection;
use flowcore::model::process_reference::ProcessReference;

use crate::style;

/// A positioned node with its dimensions and port info.
#[derive(Debug, Clone)]
pub struct NodeLayout {
    /// Node identifier (alias or derived short name)
    pub name: String,
    /// X position
    pub x: f32,
    /// Y position
    pub y: f32,
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
    /// Input port names
    pub inputs: Vec<String>,
    /// Output port names
    pub outputs: Vec<String>,
}

impl NodeLayout {
    /// Y position of the nth input port
    #[must_use]
    pub fn input_port_y(&self, index: usize) -> f32 {
        let count = self.inputs.len().max(1);
        let ports_height = (count - 1) as f32 * style::PORT_SPACING;
        let available = self.height - style::HEADER_HEIGHT;
        let padding = ((available - ports_height) / 2.0).max(0.0);
        self.y + style::HEADER_HEIGHT + padding + index as f32 * style::PORT_SPACING
    }

    /// Y position of the nth output port
    #[must_use]
    pub fn output_port_y(&self, index: usize) -> f32 {
        let count = self.outputs.len().max(1);
        let ports_height = (count - 1) as f32 * style::PORT_SPACING;
        let available = self.height - style::HEADER_HEIGHT;
        let padding = ((available - ports_height) / 2.0).max(0.0);
        self.y + style::HEADER_HEIGHT + padding + index as f32 * style::PORT_SPACING
    }

    /// X position of input ports (left edge)
    #[must_use]
    pub fn input_port_x(&self) -> f32 {
        self.x
    }

    /// X position of output ports (right edge)
    #[must_use]
    pub fn output_port_x(&self) -> f32 {
        self.x + self.width
    }
}

/// Extract a short name from a source path (e.g., "lib://flowstdlib/math/add" → "add")
fn derive_short_name(source: &str) -> String {
    source.rsplit('/').next().unwrap_or(source).to_string()
}

/// Split a route like "sequence/number" into ("sequence", "number")
fn split_route(route: &str) -> (String, String) {
    let route = route.trim_start_matches('/');
    if let Some(pos) = route.find('/') {
        (route[..pos].to_string(), route[pos + 1..].to_string())
    } else {
        (route.to_string(), String::new())
    }
}

/// Compute the required height for a node based on its port count.
fn compute_node_height(input_count: usize, output_count: usize) -> f32 {
    let max_ports = input_count.max(output_count).max(1);
    let ports_height = (max_ports - 1) as f32 * style::PORT_SPACING;
    (style::HEADER_HEIGHT + ports_height + style::NODE_PADDING * 2.0).max(style::NODE_HEIGHT)
}

/// Get the alias for a process reference.
fn process_alias(p: &ProcessReference) -> String {
    if p.alias.is_empty() {
        derive_short_name(&p.source)
    } else {
        p.alias.clone()
    }
}

/// Compute topological layout for a set of processes and connections.
///
/// Returns a map from alias to `NodeLayout` with computed positions.
#[must_use]
pub fn compute_layout(
    process_refs: &[ProcessReference],
    connections: &[Connection],
    node_info: &HashMap<String, (Vec<String>, Vec<String>)>,
) -> HashMap<String, NodeLayout> {
    let aliases: Vec<String> = process_refs.iter().map(process_alias).collect();
    let alias_set: HashSet<&str> = aliases.iter().map(String::as_str).collect();

    let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    for alias in &aliases {
        incoming.entry(alias.clone()).or_default();
        outgoing.entry(alias.clone()).or_default();
    }

    for conn in connections {
        let from_route = conn.from().to_string();
        let (from_node, _) = split_route(&from_route);
        if !alias_set.contains(from_node.as_str()) {
            continue;
        }
        for to_route in conn.to() {
            let to_str = to_route.to_string();
            let (to_node, _) = split_route(&to_str);
            if from_node != to_node && alias_set.contains(to_node.as_str()) {
                outgoing
                    .entry(from_node.clone())
                    .or_default()
                    .push(to_node.clone());
                incoming.entry(to_node).or_default().push(from_node.clone());
            }
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

    // Compute positions and build NodeLayout
    let mut layouts = HashMap::new();
    for (col, col_nodes) in &columns {
        let x = style::GRID_ORIGIN_X + *col as f32 * style::GRID_SPACING_X;
        let mut y = style::GRID_ORIGIN_Y;
        for alias in col_nodes {
            let (inputs, outputs) = node_info.get(alias).cloned().unwrap_or_default();
            let height = compute_node_height(inputs.len(), outputs.len());
            layouts.insert(
                alias.clone(),
                NodeLayout {
                    name: alias.clone(),
                    x,
                    y,
                    width: style::NODE_WIDTH,
                    height,
                    inputs,
                    outputs,
                },
            );
            y += height + style::NODE_GAP_Y;
        }
    }

    layouts
}
