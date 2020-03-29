use std::io;
use std::io::Write;
use std::path::PathBuf;

use log::info;

use crate::dumper::dump_dot;
use crate::dumper::helper;
use crate::model::flow::Flow;
use crate::model::process::Process::FlowProcess;

/// dump a flow definition that has been loaded to a file in the specified output directory
///
///
/// # Example
/// ```
/// use std::env;
/// use url::Url;
/// use flowrlib::provider::Provider;
/// use flowrlib::errors::*;
/// use flowclib::model::process::Process::FlowProcess;
///
/// struct DummyProvider {}
///
/// impl Provider for DummyProvider {
///     fn resolve_url(&self, url: &str, default_filename: &str, _ext: &[&str]) -> Result<(String, Option<String>)> {
///         Ok((url.to_string(), None))
///     }
///
///     fn get_contents(&self, url: &str) -> Result<Vec<u8>> {
///         Ok("flow = \"dummy\"\n[[input]]".as_bytes().to_owned())
///     }
/// }
///
/// fn main() {
///
///     let dummy_provider = DummyProvider {};
///     let mut url = url::Url::from_file_path(env::current_dir().unwrap()).unwrap();
///     url = url.join("samples/hello-world-simple/context.toml").unwrap();
///
///     if let FlowProcess(mut flow) = flowclib::compiler::loader::load(&url.to_string(),
///                                                       &dummy_provider).unwrap() {
///         let output_dir = tempdir::TempDir::new("dumper").unwrap().into_path();
///
///         flowclib::dumper::dump_flow::dump_flow(&flow, &output_dir).unwrap();
///     }
/// }
/// ```
pub fn dump_flow(flow: &Flow, output_dir: &PathBuf) -> io::Result<String> {
    info!("==== Dumper: Dumping flow hierarchy to '{}'", output_dir.display());
    _dump_flow(flow, 0, output_dir)
}

/*
    dump the flow definition recursively, tracking what level we are at as we go down
*/
fn _dump_flow(flow: &Flow, level: usize, output_dir: &PathBuf) -> io::Result<String> {
    let mut writer = helper::create_output_file(&output_dir, &flow.alias, "dump")?;
    writer.write_all(format!("\nLevel={}\n{}", level, flow).as_bytes())?;

    writer = helper::create_output_file(&output_dir, &flow.alias, "dot")?;
    dump_dot::flow_to_dot(flow, &mut writer)?;

    // Dump sub-flows
    if let Some(ref flow_refs) = flow.process_refs {
        for flow_ref in flow_refs {
            match flow_ref.process {
                FlowProcess(ref subflow) => {
                    _dump_flow(subflow, level + 1, output_dir)?;
                }
                _ => {}
            }
        }
    }

    Ok("All flows dumped".to_string())
}
