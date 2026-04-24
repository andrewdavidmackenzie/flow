//! Node layout types and layout computation for the flow editor canvas.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::collections::HashMap;

use iced::Point;

use flowcore::model::connection::Connection;
use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::io::IO;
use flowcore::model::name::HasName;
use flowcore::model::process::Process;
use flowcore::model::process_reference::ProcessReference;

use crate::utils::{base_port_name, derive_short_name, format_value, split_route};

/// Default node width when no layout width is specified
pub(crate) const DEFAULT_WIDTH: f32 = 180.0;
/// Default node height when no layout height is specified
pub(crate) const DEFAULT_HEIGHT: f32 = 120.0;
/// Horizontal spacing between auto-laid-out nodes
const GRID_SPACING_X: f32 = 250.0;
/// Vertical spacing between auto-laid-out nodes
const GRID_SPACING_Y: f32 = 170.0;
/// Number of columns in auto-layout grid
const GRID_COLUMNS: usize = 3;
/// Starting X offset for auto-layout
const GRID_ORIGIN_X: f32 = 50.0;
/// Starting Y offset for auto-layout
const GRID_ORIGIN_Y: f32 = 50.0;
/// Port label font size
pub(crate) const PORT_FONT_SIZE: f32 = 11.0;
/// Vertical spacing between ports
pub(crate) const PORT_SPACING: f32 = 20.0;
/// Vertical offset from top of node to first port
pub(crate) const PORT_START_Y: f32 = 55.0;

static EMPTY_IO: Vec<IO> = Vec::new();

/// A positioned node derived from a [`ProcessReference`], ready for rendering.
///
/// Combines a `ProcessReference` (position, alias, source, initializations)
/// with its resolved `Process` (ports, description, type) for rendering.
/// Rebuilt every frame from `FlowDefinition` — not persisted.
#[derive(Debug, Clone)]
pub(crate) struct NodeLayout {
    pub(crate) process_ref: ProcessReference,
    pub(crate) process: Option<Process>,
}

impl Default for NodeLayout {
    fn default() -> Self {
        Self {
            process_ref: ProcessReference {
                alias: String::new(),
                source: String::new(),
                initializations: std::collections::BTreeMap::new(),
                x: Some(100.0),
                y: Some(100.0),
                width: Some(180.0),
                height: Some(120.0),
            },
            process: None,
        }
    }
}

impl NodeLayout {
    pub(crate) fn alias(&self) -> &str {
        if self.process_ref.alias.is_empty() {
            &self.process_ref.source
        } else {
            &self.process_ref.alias
        }
    }

    pub(crate) fn source(&self) -> &str {
        &self.process_ref.source
    }

    pub(crate) fn description(&self) -> String {
        match &self.process {
            Some(Process::FunctionProcess(f)) => f.description.clone(),
            Some(Process::FlowProcess(f)) => f.description.clone(),
            None => String::new(),
        }
    }

    pub(crate) fn x(&self) -> f32 {
        self.process_ref.x.unwrap_or(100.0)
    }

    pub(crate) fn y(&self) -> f32 {
        self.process_ref.y.unwrap_or(100.0)
    }

    pub(crate) fn width(&self) -> f32 {
        self.process_ref.width.unwrap_or(DEFAULT_WIDTH)
    }

    pub(crate) fn height(&self) -> f32 {
        self.process_ref.height.unwrap_or(DEFAULT_HEIGHT)
    }

    pub(crate) fn inputs(&self) -> &[IO] {
        match &self.process {
            Some(Process::FunctionProcess(f)) => &f.inputs,
            Some(Process::FlowProcess(f)) => &f.inputs,
            None => &EMPTY_IO,
        }
    }

    pub(crate) fn outputs(&self) -> &[IO] {
        match &self.process {
            Some(Process::FunctionProcess(f)) => &f.outputs,
            Some(Process::FlowProcess(f)) => &f.outputs,
            None => &EMPTY_IO,
        }
    }

    pub(crate) fn has_initializers(&self) -> bool {
        !self.process_ref.initializations.is_empty()
    }

    pub(crate) fn max_initializer_display_len(&self) -> usize {
        self.process_ref
            .initializations
            .values()
            .map(|init| {
                let s = match init {
                    flowcore::model::input::InputInitializer::Once(v) => {
                        format!("once: {}", format_value(v))
                    }
                    flowcore::model::input::InputInitializer::Always(v) => {
                        format!("always: {}", format_value(v))
                    }
                };
                s.len()
            })
            .max()
            .unwrap_or(0)
    }

    pub(crate) fn initializer_display(&self, port_name: &str) -> Option<String> {
        self.process_ref
            .initializations
            .get(port_name)
            .map(|init| match init {
                flowcore::model::input::InputInitializer::Once(v) => {
                    format!("once: {}", format_value(v))
                }
                flowcore::model::input::InputInitializer::Always(v) => {
                    format!("always: {}", format_value(v))
                }
            })
    }

    pub(crate) fn fill_color(&self) -> iced::Color {
        match &self.process {
            Some(Process::FlowProcess(_)) => iced::Color::from_rgb(0.9, 0.6, 0.2),
            Some(Process::FunctionProcess(f)) => {
                if f.get_lib_reference().is_some() {
                    iced::Color::from_rgb(0.3, 0.5, 0.9)
                } else if f.get_context_reference().is_some() {
                    iced::Color::from_rgb(0.3, 0.75, 0.45)
                } else {
                    iced::Color::from_rgb(0.6, 0.3, 0.8)
                }
            }
            None => {
                if self.source().starts_with("lib://") {
                    iced::Color::from_rgb(0.3, 0.5, 0.9)
                } else if self.source().starts_with("context://") {
                    iced::Color::from_rgb(0.3, 0.75, 0.45)
                } else {
                    iced::Color::from_rgb(0.9, 0.6, 0.2)
                }
            }
        }
    }

    pub(crate) fn is_openable(&self) -> bool {
        match &self.process {
            Some(Process::FlowProcess(_)) => true,
            Some(Process::FunctionProcess(f)) => {
                f.get_lib_reference().is_none() && f.get_context_reference().is_none()
            }
            None => {
                !self.source().starts_with("lib://") && !self.source().starts_with("context://")
            }
        }
    }

    pub(crate) fn output_port_position(&self, port_index: usize) -> Point {
        Point::new(
            self.x() + self.width(),
            self.y() + PORT_START_Y + port_index as f32 * PORT_SPACING,
        )
    }

    pub(crate) fn input_port_position(&self, port_index: usize) -> Point {
        Point::new(
            self.x(),
            self.y() + PORT_START_Y + port_index as f32 * PORT_SPACING,
        )
    }

    pub(crate) fn resize_handle_positions(&self) -> [(crate::flow_canvas::ResizeHandle, Point); 8] {
        use crate::flow_canvas::ResizeHandle;
        let mid_x = self.x() + self.width() / 2.0;
        let mid_y = self.y() + self.height() / 2.0;
        let right = self.x() + self.width();
        let bottom = self.y() + self.height();
        [
            (ResizeHandle::TopLeft, Point::new(self.x(), self.y())),
            (ResizeHandle::Top, Point::new(mid_x, self.y())),
            (ResizeHandle::TopRight, Point::new(right, self.y())),
            (ResizeHandle::Left, Point::new(self.x(), mid_y)),
            (ResizeHandle::Right, Point::new(right, mid_y)),
            (ResizeHandle::BottomLeft, Point::new(self.x(), bottom)),
            (ResizeHandle::Bottom, Point::new(mid_x, bottom)),
            (ResizeHandle::BottomRight, Point::new(right, bottom)),
        ]
    }

    pub(crate) fn is_in_source_text_zone(&self, point: Point) -> bool {
        use crate::flow_canvas::SOURCE_FONT_SIZE;
        let text_center_x = self.x() + self.width() / 2.0;
        let text_top_y = self.y() + 34.0;
        let text_height = SOURCE_FONT_SIZE + 4.0;
        let text_half_width = self.width() * 0.4;

        point.x >= text_center_x - text_half_width
            && point.x <= text_center_x + text_half_width
            && point.y >= text_top_y
            && point.y <= text_top_y + text_height
    }

    pub(crate) fn find_output_pos_inline(&self, port: &str) -> Point {
        if port.is_empty() {
            self.output_port_position(0)
        } else {
            let base = base_port_name(port);
            let idx = self
                .outputs()
                .iter()
                .position(|p| p.name() == base)
                .unwrap_or(0);
            self.output_port_position(idx)
        }
    }

    pub(crate) fn find_input_pos_inline(&self, port: &str) -> Point {
        if port.is_empty() {
            self.input_port_position(0)
        } else {
            let base = base_port_name(port);
            let idx = self
                .inputs()
                .iter()
                .position(|p| p.name() == base)
                .unwrap_or(0);
            self.input_port_position(idx)
        }
    }

    /// Build render-only [`NodeLayout`] list directly from a [`FlowDefinition`].
    ///
    /// This is the single entry point for converting process references and their
    /// resolved subprocess definitions into the rendering representation.
    /// The returned layouts are ephemeral and must not be stored in persistent state.
    pub(crate) fn build_from_flow(flow_def: &FlowDefinition) -> Vec<NodeLayout> {
        let topo_positions =
            compute_topological_layout(&flow_def.process_refs, &flow_def.connections);

        let mut nodes = Vec::with_capacity(flow_def.process_refs.len());

        for (i, pref) in flow_def.process_refs.iter().enumerate() {
            let alias = if pref.alias.is_empty() {
                derive_short_name(&pref.source)
            } else {
                pref.alias.clone()
            };

            let resolved = flow_def.subprocesses.get(&alias);

            let input_count = resolved.map_or(0, |p| match p {
                Process::FunctionProcess(f) => f.inputs.len(),
                Process::FlowProcess(f) => f.inputs.len(),
            });
            let output_count = resolved.map_or(0, |p| match p {
                Process::FunctionProcess(f) => f.outputs.len(),
                Process::FlowProcess(f) => f.outputs.len(),
            });
            let min_ports = input_count.max(output_count);
            let min_height = PORT_START_Y + (min_ports as f32 + 1.0) * PORT_SPACING;

            let (default_x, default_y) = if let Some((tx, ty)) = topo_positions.get(&alias) {
                (*tx, *ty)
            } else {
                let col = i % GRID_COLUMNS;
                let row = i / GRID_COLUMNS;
                (
                    GRID_ORIGIN_X + col as f32 * GRID_SPACING_X,
                    GRID_ORIGIN_Y + row as f32 * GRID_SPACING_Y,
                )
            };

            let mut process_ref = pref.clone();
            if process_ref.alias.is_empty() {
                process_ref.alias = alias;
            }
            process_ref.x = Some(pref.x.unwrap_or(default_x));
            process_ref.y = Some(pref.y.unwrap_or(default_y));
            process_ref.width = Some(pref.width.unwrap_or(DEFAULT_WIDTH));
            process_ref.height = Some(pref.height.unwrap_or(DEFAULT_HEIGHT.max(min_height)));

            nodes.push(NodeLayout {
                process_ref,
                process: resolved.cloned(),
            });
        }

        nodes
    }
}

/// Compute topology-based positions for nodes without saved layout.
///
/// Assigns each node a column based on its depth from source nodes (nodes with no
/// incoming connections). Nodes are spread vertically within each column.
fn compute_topological_layout(
    process_refs: &[ProcessReference],
    connections: &[Connection],
) -> HashMap<String, (f32, f32)> {
    // Build alias list
    let aliases: Vec<String> = process_refs
        .iter()
        .map(|p| {
            if p.alias.is_empty() {
                derive_short_name(&p.source)
            } else {
                p.alias.clone()
            }
        })
        .collect();

    // Build adjacency: which nodes feed which
    let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    for alias in &aliases {
        incoming.entry(alias.clone()).or_default();
        outgoing.entry(alias.clone()).or_default();
    }

    let alias_set: std::collections::HashSet<&str> = aliases.iter().map(String::as_str).collect();
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

    // Assign column depth using BFS from source nodes (no incoming edges)
    let mut depth: HashMap<String, usize> = HashMap::new();
    let mut queue: std::collections::VecDeque<String> = std::collections::VecDeque::new();

    for alias in &aliases {
        if incoming.get(alias).is_none_or(std::vec::Vec::is_empty) {
            depth.insert(alias.clone(), 0);
            queue.push_back(alias.clone());
        }
    }

    // BFS to assign max depth (longest path from any source).
    // Cap depth to prevent infinite loops on cyclic flows (e.g., fibonacci feedback).
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

    // Assign any unvisited nodes depth 0
    for alias in &aliases {
        depth.entry(alias.clone()).or_insert(0);
    }

    // Group nodes by column
    let mut columns: HashMap<usize, Vec<String>> = HashMap::new();
    for alias in &aliases {
        let col = depth.get(alias).copied().unwrap_or(0);
        columns.entry(col).or_default().push(alias.clone());
    }

    // Compute positions: spread columns horizontally, nodes vertically within each column
    let mut positions = HashMap::new();
    for (col, col_nodes) in &columns {
        let x = GRID_ORIGIN_X + *col as f32 * GRID_SPACING_X;
        let total_height = col_nodes.len() as f32 * GRID_SPACING_Y;
        let start_y = GRID_ORIGIN_Y + (GRID_SPACING_Y - total_height) / 2.0;

        for (row, alias) in col_nodes.iter().enumerate() {
            let y = start_y.max(GRID_ORIGIN_Y) + row as f32 * GRID_SPACING_Y;
            positions.insert(alias.clone(), (x, y));
        }
    }

    positions
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod test {
    use super::*;
    use flowcore::model::function_definition::FunctionDefinition;
    use flowcore::model::io::IO;
    use flowcore::model::route::Route;
    use iced::Point;
    use url::Url;

    fn test_node(alias: &str, source: &str, process: Option<Process>) -> NodeLayout {
        NodeLayout {
            process_ref: ProcessReference {
                alias: alias.into(),
                source: source.into(),
                initializations: std::collections::BTreeMap::new(),
                x: Some(100.0),
                y: Some(100.0),
                width: Some(180.0),
                height: Some(120.0),
            },
            process,
        }
    }

    fn lib_function() -> FunctionDefinition {
        let mut f = FunctionDefinition::default();
        f.lib_reference = Some(Url::parse("lib://test").expect("valid url"));
        f
    }

    fn context_function() -> FunctionDefinition {
        let mut f = FunctionDefinition::default();
        f.context_reference = Some(Url::parse("context://stdio/stdout").expect("valid url"));
        f
    }

    fn function_with_io(inputs: Vec<IO>, outputs: Vec<IO>) -> FunctionDefinition {
        let mut f = FunctionDefinition::default();
        f.inputs = inputs;
        f.outputs = outputs;
        f
    }

    #[test]
    fn is_openable_lib() {
        let node = test_node("n", "", Some(Process::FunctionProcess(lib_function())));
        assert!(!node.is_openable());
    }

    #[test]
    fn is_openable_context() {
        let node = test_node("n", "", Some(Process::FunctionProcess(context_function())));
        assert!(!node.is_openable());
    }

    #[test]
    fn is_openable_local() {
        let node = test_node(
            "n",
            "",
            Some(Process::FlowProcess(FlowDefinition::default())),
        );
        assert!(node.is_openable());
    }

    #[test]
    fn is_openable_provided_impl() {
        let node = test_node(
            "n",
            "",
            Some(Process::FunctionProcess(FunctionDefinition::default())),
        );
        assert!(node.is_openable());
    }

    #[test]
    fn fill_color_by_process() {
        let lib = test_node("n", "", Some(Process::FunctionProcess(lib_function())));
        let ctx = test_node("n", "", Some(Process::FunctionProcess(context_function())));
        let prov = test_node(
            "n",
            "",
            Some(Process::FunctionProcess(FunctionDefinition::default())),
        );
        let flow = test_node(
            "n",
            "",
            Some(Process::FlowProcess(FlowDefinition::default())),
        );
        assert_ne!(lib.fill_color(), ctx.fill_color());
        assert_ne!(lib.fill_color(), prov.fill_color());
        assert_ne!(lib.fill_color(), flow.fill_color());
        assert_ne!(ctx.fill_color(), prov.fill_color());
        assert_ne!(ctx.fill_color(), flow.fill_color());
        assert_ne!(prov.fill_color(), flow.fill_color());
    }

    #[test]
    fn node_layout_port_positions() {
        let f = function_with_io(
            vec![
                IO::new_named(vec![], Route::default(), "i1"),
                IO::new_named(vec![], Route::default(), "i2"),
            ],
            vec![IO::new_named(vec![], Route::default(), "out")],
        );
        let node = test_node("test", "lib://test", Some(Process::FunctionProcess(f)));
        let ip0 = node.input_port_position(0);
        let ip1 = node.input_port_position(1);
        let op0 = node.output_port_position(0);

        // Input ports on left edge
        assert!((ip0.x - 100.0).abs() < 0.01);
        assert!((ip1.x - 100.0).abs() < 0.01);
        // Output ports on right edge
        assert!((op0.x - 280.0).abs() < 0.01);
        // Ports vertically spaced
        assert!(ip1.y > ip0.y);
    }

    #[test]
    fn find_node_output_inline_with_subroute() {
        let f = function_with_io(
            vec![],
            vec![
                IO::new_named(vec![], Route::default(), "string"),
                IO::new_named(vec![], Route::default(), "json"),
            ],
        );
        let node = test_node("get", "", Some(Process::FunctionProcess(f)));
        let string_pos = node.find_output_pos_inline("string/1");
        let json_pos = node.find_output_pos_inline("json/2");
        // string is output 0, json is output 1 — different y positions
        assert!((json_pos.y - string_pos.y).abs() > 1.0);
    }

    #[test]
    fn hit_test_source_text_zone() {
        let node = test_node("test", "lib://flowstdlib/math/add", None);
        // Source text is centered at (node.x() + width/2, node.y() + 34.0)
        let source_center = Point::new(190.0, 134.0);
        assert!(node.is_in_source_text_zone(source_center));
        // Point clearly outside source text zone but inside node
        let node_body = Point::new(110.0, 200.0);
        assert!(!node.is_in_source_text_zone(node_body));
    }
}
