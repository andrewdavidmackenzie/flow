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

use crate::flow_canvas::TITLE_FONT_SIZE;
use crate::utils::{base_port_name, derive_short_name, format_value, split_route};

/// Default node width when no layout width is specified
pub(crate) const DEFAULT_WIDTH: f32 = 180.0;
/// Default node height when no layout height is specified
pub(crate) const DEFAULT_HEIGHT: f32 = 120.0;
/// Horizontal spacing between auto-laid-out nodes
const GRID_SPACING_X: f32 = 250.0;
/// Vertical spacing between auto-laid-out nodes
const GRID_SPACING_Y: f32 = 170.0;
/// Starting X offset for auto-layout
const GRID_ORIGIN_X: f32 = 50.0;
/// Starting Y offset for auto-layout
const GRID_ORIGIN_Y: f32 = 50.0;
/// Port label font size
pub(crate) const PORT_FONT_SIZE: f32 = 11.0;
/// Vertical spacing between ports
pub(crate) const PORT_SPACING: f32 = 20.0;

static EMPTY_IO: Vec<IO> = Vec::new();

/// A positioned node derived from a [`ProcessReference`], ready for rendering.
///
/// Combines a `ProcessReference` (position, alias, source, initializations)
/// with its resolved `Process` (ports, description, type) for rendering.
/// Rebuilt every frame from `FlowDefinition` — not persisted.
#[derive(Debug, Clone)]
pub(crate) struct NodeLayout<'a> {
    pub(crate) process_ref: &'a ProcessReference,
    pub(crate) process: Option<&'a Process>,
}

impl<'a> NodeLayout<'a> {
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
        let stored = self.process_ref.height.unwrap_or(DEFAULT_HEIGHT);
        let header = 50.0;
        let bottom_pad = 25.0;
        let max_ports = self.inputs().len().max(self.outputs().len());
        let min_for_ports = header + max_ports.saturating_sub(1) as f32 * PORT_SPACING + bottom_pad;
        stored.max(min_for_ports)
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

    fn port_start_y(&self, port_count: usize) -> f32 {
        let header_height = 50.0;
        let available = self.height() - header_height;
        let ports_height = (port_count.saturating_sub(1)) as f32 * PORT_SPACING;
        let padding = ((available - ports_height) / 2.0).max(0.0);
        self.y() + header_height + padding
    }

    pub(crate) fn output_port_position(&self, port_index: usize) -> Point {
        let count = self.outputs().len().max(1);
        Point::new(
            self.x() + self.width(),
            self.port_start_y(count) + port_index as f32 * PORT_SPACING,
        )
    }

    pub(crate) fn input_port_position(&self, port_index: usize) -> Point {
        let count = self.inputs().len().max(1);
        Point::new(
            self.x(),
            self.port_start_y(count) + port_index as f32 * PORT_SPACING,
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

    /// Check whether `screen_pos` is within the hit radius of any resize handle
    /// on this node. Returns the handle variant and the given `node_index` if hit.
    ///
    /// The hit test is performed in screen space so the grab area is constant
    /// regardless of zoom.
    pub(crate) fn hit_test_resize_handle(
        &self,
        node_index: usize,
        screen_pos: Point,
        zoom: f32,
        offset: Point,
    ) -> Option<(usize, crate::flow_canvas::ResizeHandle)> {
        use crate::flow_canvas::{transform_point, RESIZE_HANDLE_HIT};
        for (handle, world_pt) in &self.resize_handle_positions() {
            let screen_pt = transform_point(*world_pt, zoom, offset);
            let dx = (screen_pos.x - screen_pt.x).abs();
            let dy = (screen_pos.y - screen_pt.y).abs();
            if dx <= RESIZE_HANDLE_HIT && dy <= RESIZE_HANDLE_HIT {
                return Some((node_index, *handle));
            }
        }
        None
    }

    pub(crate) fn is_in_title_zone(&self, point: Point) -> bool {
        let text_center_x = self.x() + self.width() / 2.0;
        let text_top_y = self.y() + 6.0;
        let text_height = TITLE_FONT_SIZE + 8.0;
        let text_half_width = self.width() * 0.45;

        point.x >= text_center_x - text_half_width
            && point.x <= text_center_x + text_half_width
            && point.y >= text_top_y
            && point.y <= text_top_y + text_height
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
    pub(crate) fn build_from_flow(flow_def: &'a FlowDefinition) -> Vec<NodeLayout<'a>> {
        let mut nodes = Vec::with_capacity(flow_def.process_refs.len());

        for pref in &flow_def.process_refs {
            let alias = if pref.alias.is_empty() {
                derive_short_name(&pref.source)
            } else {
                pref.alias.clone()
            };

            let resolved = flow_def.subprocesses.get(&alias);

            nodes.push(NodeLayout {
                process_ref: pref,
                process: resolved,
            });
        }

        nodes
    }
}

/// Compute topology-based positions for nodes without saved layout.
///
/// Assigns each node a column based on its depth from source nodes (nodes with no
/// incoming connections). Nodes are spread vertically within each column.
pub(crate) fn compute_topological_layout(
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

    fn test_pref(alias: &str, source: &str) -> ProcessReference {
        ProcessReference {
            alias: alias.into(),
            source: source.into(),
            initializations: std::collections::BTreeMap::new(),
            x: Some(100.0),
            y: Some(100.0),
            width: Some(180.0),
            height: Some(120.0),
        }
    }

    fn test_node(
        alias: &str,
        source: &str,
        process: Option<Process>,
    ) -> (ProcessReference, Option<Process>) {
        (test_pref(alias, source), process)
    }

    fn as_layout(data: &(ProcessReference, Option<Process>)) -> NodeLayout<'_> {
        NodeLayout {
            process_ref: &data.0,
            process: data.1.as_ref(),
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
        let data = test_node("n", "", Some(Process::FunctionProcess(lib_function())));
        let node = as_layout(&data);
        assert!(!node.is_openable());
    }

    #[test]
    fn is_openable_context() {
        let data = test_node("n", "", Some(Process::FunctionProcess(context_function())));
        let node = as_layout(&data);
        assert!(!node.is_openable());
    }

    #[test]
    fn is_openable_local() {
        let data = test_node(
            "n",
            "",
            Some(Process::FlowProcess(FlowDefinition::default())),
        );
        let node = as_layout(&data);
        assert!(node.is_openable());
    }

    #[test]
    fn is_openable_provided_impl() {
        let data = test_node(
            "n",
            "",
            Some(Process::FunctionProcess(FunctionDefinition::default())),
        );
        let node = as_layout(&data);
        assert!(node.is_openable());
    }

    #[test]
    fn fill_color_by_process() {
        let lib_data = test_node("n", "", Some(Process::FunctionProcess(lib_function())));
        let lib = as_layout(&lib_data);
        let ctx_data = test_node("n", "", Some(Process::FunctionProcess(context_function())));
        let ctx = as_layout(&ctx_data);
        let prov_data = test_node(
            "n",
            "",
            Some(Process::FunctionProcess(FunctionDefinition::default())),
        );
        let prov = as_layout(&prov_data);
        let flow_data = test_node(
            "n",
            "",
            Some(Process::FlowProcess(FlowDefinition::default())),
        );
        let flow = as_layout(&flow_data);
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
        let data = test_node("test", "lib://test", Some(Process::FunctionProcess(f)));
        let node = as_layout(&data);
        let ip0 = node.input_port_position(0);
        let ip1 = node.input_port_position(1);
        let op0 = node.output_port_position(0);
        assert!((ip0.x - 100.0).abs() < 0.01);
        assert!((ip1.x - 100.0).abs() < 0.01);
        assert!((op0.x - 280.0).abs() < 0.01);
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
        let data = test_node("get", "", Some(Process::FunctionProcess(f)));
        let node = as_layout(&data);
        let string_pos = node.find_output_pos_inline("string/1");
        let json_pos = node.find_output_pos_inline("json/2");
        assert!((json_pos.y - string_pos.y).abs() > 1.0);
    }

    #[test]
    fn hit_test_source_text_zone() {
        let data = test_node("test", "lib://flowstdlib/math/add", None);
        let node = as_layout(&data);
        let source_center = Point::new(190.0, 134.0);
        assert!(node.is_in_source_text_zone(source_center));
        let node_body = Point::new(110.0, 200.0);
        assert!(!node.is_in_source_text_zone(node_body));
    }
}
