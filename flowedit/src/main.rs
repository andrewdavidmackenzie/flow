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

use std::collections::BTreeSet;
use std::path::PathBuf;

use clap::{Arg, ArgAction, Command as ClapCommand};
use log::info;
use simpath::Simpath;
use url::Url;

use flowcore::model::flow_definition::FlowDefinition;

mod file_ops;
mod flow_canvas;
mod flow_edit;
mod hierarchy_panel;
mod history;
mod initializer;
mod library_mgmt;
mod library_panel;
mod node_layout;
mod utils;
mod window_state;

pub(crate) use flow_edit::{toolbar_btn, FlowEdit, FlowEditMessage, Message, ViewMessage};

#[cfg(test)]
pub(crate) use flow_edit::FunctionEditMessage;
pub(crate) use window_state::FunctionViewer;
#[cfg(test)]
pub(crate) use window_state::{InitializerEditor, WindowKind, WindowState};

#[cfg(test)]
mod ui_test;

/// Entry point for the `flowedit` application.
///
/// Parses CLI arguments, loads the flow definition, and launches the iced GUI.
fn main() -> iced::Result {
    env_logger::init();
    iced::daemon(FlowEdit::new, FlowEdit::update, FlowEdit::view)
        .title(FlowEdit::title)
        .subscription(FlowEdit::subscription)
        .antialiasing(true)
        .run()
}

pub(crate) struct CliArgs {
    pub(crate) lib_dirs: Vec<String>,
    pub(crate) flow_file: Option<String>,
    pub(crate) auto_build: bool,
    pub(crate) auto_run: bool,
}

pub(crate) fn parse_cli_args() -> CliArgs {
    let matches = ClapCommand::new("flowedit")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Visual editor for flow definition files")
        .arg(
            Arg::new("flow-file")
                .required(false)
                .help("Path to the flow definition file (.toml, .yaml, or .json)"),
        )
        .arg(
            Arg::new("lib_dir")
                .short('L')
                .long("libdir")
                .num_args(1)
                .action(ArgAction::Append)
                .value_name("LIB_DIR")
                .help("Add a directory to the Library Search path"),
        )
        .arg(
            Arg::new("auto-build")
                .long("auto-build")
                .action(ArgAction::SetTrue)
                .help("Automatically build the flow on startup"),
        )
        .arg(
            Arg::new("auto-run")
                .long("auto-run")
                .action(ArgAction::SetTrue)
                .help("Automatically build and run the flow on startup"),
        )
        .get_matches();

    let lib_dirs: Vec<String> = if matches.contains_id("lib_dir") {
        matches
            .get_many::<String>("lib_dir")
            .map(|dirs| dirs.map(std::string::ToString::to_string).collect())
            .unwrap_or_default()
    } else {
        vec![]
    };

    let flow_file = matches.get_one::<String>("flow-file").cloned();
    let auto_run = matches.get_flag("auto-run");
    let auto_build = auto_run || matches.get_flag("auto-build");
    CliArgs {
        lib_dirs,
        flow_file,
        auto_build,
        auto_run,
    }
}

pub(crate) fn setup_lib_search_path(lib_dirs: &[String]) {
    let mut lib_search_path = Simpath::new_with_separator("FLOW_LIB_PATH", ',');
    for addition in lib_dirs {
        lib_search_path.add(addition);
        info!("'{addition}' added to the Library Search Path");
    }
    if lib_search_path.is_empty() {
        if let Ok(home) = std::env::var("HOME") {
            let default_lib = format!("{home}/.flow/lib");
            lib_search_path.add(&default_lib);
            std::env::set_var("FLOW_LIB_PATH", &default_lib);
        }
    } else if !lib_dirs.is_empty() {
        let current = std::env::var("FLOW_LIB_PATH").unwrap_or_default();
        let additions = lib_dirs.join(",");
        if current.is_empty() {
            std::env::set_var("FLOW_LIB_PATH", additions);
        } else {
            std::env::set_var("FLOW_LIB_PATH", format!("{current},{additions}"));
        }
    }
}

pub(crate) fn load_initial_flow(
    flow_file: Option<&str>,
) -> (String, FlowDefinition, BTreeSet<Url>) {
    if let Some(flow_path_str) = flow_file {
        let flow_path = PathBuf::from(flow_path_str);
        match file_ops::load_flow(&flow_path) {
            Ok(mut fd) => {
                let nc = fd.process_refs.len();
                let ec = fd.connections.len();
                let lib_refs = fd.lib_references.clone();
                if let Ok(url) = Url::from_file_path(&flow_path) {
                    fd.source_url = url;
                }
                (
                    format!("Ready - {nc} nodes, {ec} connections"),
                    fd,
                    lib_refs,
                )
            }
            Err(e) => {
                let fd = FlowDefinition {
                    name: String::from("(error)"),
                    ..FlowDefinition::default()
                };
                (format!("Error loading flow: {e}"), fd, BTreeSet::new())
            }
        }
    } else {
        let fd = FlowDefinition {
            name: String::from("(new flow)"),
            ..FlowDefinition::default()
        };
        (String::from("Ready"), fd, BTreeSet::new())
    }
}
