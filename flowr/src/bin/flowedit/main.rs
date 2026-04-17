//! `flowedit` is a visual editor for flow definition files.
//!
//! Phase 1 provides a read-only viewer that renders the process nodes and connections
//! from a flow definition file onto an iced [`Canvas`][iced::widget::canvas::Canvas].
//!
//! Usage:
//! ```text
//! flowedit [flow-definition-file]
//! ```
//!
//! The flow file (TOML, YAML, or JSON) is parsed using flowcore's deserializer
//! and each [`ProcessReference`][flowcore::model::process_reference::ProcessReference]
//! is displayed as a colored, rounded rectangle on the canvas, with connections
//! drawn as bezier curves between nodes.

use std::path::PathBuf;

use clap::{Arg, Command as ClapCommand};
use iced::widget::{Column, Row, Text};
use iced::{Element, Fill, Task};
use url::Url;

use flowcore::deserializers::deserializer::get;
use flowcore::model::process::Process;

mod canvas_view;
use canvas_view::{
    build_edge_layouts, build_node_layouts, EdgeLayout, FlowCanvasState, NodeLayout,
};

/// Messages handled by the flowedit application
#[derive(Debug, Clone)]
enum Message {}

/// Top-level application state
struct FlowEdit {
    /// The name of the flow being viewed
    flow_name: String,
    /// Positioned nodes derived from the flow's process references
    nodes: Vec<NodeLayout>,
    /// Connection edges between nodes
    edges: Vec<EdgeLayout>,
    /// Canvas state for caching rendered geometry
    canvas_state: FlowCanvasState,
    /// Status message displayed in the bottom bar
    status: String,
}

/// Main entry point for the flowedit binary.
///
/// Parses CLI arguments, loads the flow definition, and launches the iced GUI.
fn main() -> iced::Result {
    iced::application(FlowEdit::new, FlowEdit::update, FlowEdit::view)
        .title(FlowEdit::title)
        .antialiasing(true)
        .run()
}

impl FlowEdit {
    /// Create the application, parsing CLI args and optionally loading a flow file.
    fn new() -> (Self, Task<Message>) {
        let matches = ClapCommand::new("flowedit")
            .version(env!("CARGO_PKG_VERSION"))
            .about("Visual editor for flow definition files")
            .arg(
                Arg::new("flow-file")
                    .required(false)
                    .help("Path to the flow definition file (.toml, .yaml, or .json)"),
            )
            .get_matches();

        let (flow_name, nodes, edges, status) =
            if let Some(flow_path_str) = matches.get_one::<String>("flow-file") {
                let flow_path = PathBuf::from(flow_path_str);
                match load_flow(&flow_path) {
                    Ok((name, node_list, edge_list)) => {
                        let nc = node_list.len();
                        let ec = edge_list.len();
                        (
                            name,
                            node_list,
                            edge_list,
                            format!("Ready - {nc} nodes, {ec} connections"),
                        )
                    }
                    Err(e) => (
                        String::from("(error)"),
                        Vec::new(),
                        Vec::new(),
                        format!("Error loading flow: {e}"),
                    ),
                }
            } else {
                (
                    String::from("(new flow)"),
                    Vec::new(),
                    Vec::new(),
                    String::from("Ready"),
                )
            };

        let app = FlowEdit {
            flow_name,
            nodes,
            edges,
            canvas_state: FlowCanvasState::default(),
            status,
        };

        (app, Task::none())
    }

    /// Return the window title.
    fn title(&self) -> String {
        format!("flowedit - {}", self.flow_name)
    }

    /// Handle messages (none in Phase 1).
    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    /// Build the view: a canvas area and a status bar at the bottom.
    fn view(&self) -> Element<'_, Message> {
        let canvas = self
            .canvas_state
            .view(&self.nodes, &self.edges)
            .map(|()| unreachable!());

        let status_bar: Row<'_, Message> = Row::new().push(Text::new(self.status.clone()).size(14));

        Column::new()
            .push(iced::widget::container(canvas).width(Fill).height(Fill))
            .push(iced::widget::container(status_bar).width(Fill).padding(5))
            .into()
    }
}

/// Load a flow definition file and return the flow name, node layouts, and edge layouts.
fn load_flow(path: &PathBuf) -> Result<(String, Vec<NodeLayout>, Vec<EdgeLayout>), String> {
    let abs_path = if path.is_absolute() {
        path.clone()
    } else {
        std::env::current_dir()
            .map_err(|e| format!("Could not get current directory: {e}"))?
            .join(path)
    };

    let url =
        Url::from_file_path(&abs_path).map_err(|()| format!("Invalid file path: {abs_path:?}"))?;

    let contents =
        std::fs::read_to_string(&abs_path).map_err(|e| format!("Could not read file: {e}"))?;

    let deserializer =
        get::<Process>(&url).map_err(|e| format!("Could not get deserializer: {e}"))?;

    let process = deserializer
        .deserialize(&contents, Some(&url))
        .map_err(|e| format!("Could not parse flow definition: {e}"))?;

    match process {
        Process::FlowProcess(flow) => {
            let edges = build_edge_layouts(&flow.connections);
            let nodes = build_node_layouts(&flow.process_refs, &flow.connections);
            Ok((flow.name, nodes, edges))
        }
        Process::FunctionProcess(_) => Err(
            "The specified file defines a Function, not a Flow. flowedit requires a flow definition."
                .to_string(),
        ),
    }
}
