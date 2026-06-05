//! `flowgraph` provides graph layout and SVG rendering for flow programs.
//!
//! It computes topological layouts for flow diagrams and renders them as
//! interactive SVG files with clickable navigation and tooltips.

use std::fs;
use std::path::Path;

use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::process::Process::FlowProcess;

mod edge;
mod layout;
mod renderer;
mod shapes;
mod style;

/// Render a flow and all its sub-flows as SVG files in the output directory.
///
/// # Errors
///
/// Returns an error if SVG files cannot be written.
pub fn dump_flow_svgs(flow: &FlowDefinition, output_dir: &Path) -> Result<(), String> {
    let svg_content = renderer::render_flow(flow);
    let name = if flow.alias.is_empty() {
        "root".to_string()
    } else {
        flow.alias.replace('-', "_")
    };
    let filename = format!("{name}.svg");
    let path = output_dir.join(&filename);
    fs::write(&path, svg_content)
        .map_err(|e| format!("Could not write SVG to {}: {e}", path.display()))?;

    for process in flow.subprocesses.values() {
        if let FlowProcess(sub_flow) = process {
            dump_flow_svgs(sub_flow, output_dir)?;
        }
    }

    Ok(())
}
