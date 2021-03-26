use std::io;
use std::io::Write;
use std::io::{Error, ErrorKind};
use std::path::Path;

use log::info;

use provider::content::provider::Provider;

use crate::dumper::dump_dot;
use crate::dumper::helper;
use crate::model::flow::Flow;
use crate::model::process::Process::FlowProcess;

/// dump a flow definition that has been loaded to a file in the specified output directory
///
/// # Example
/// ```
/// use std::env;
/// use url::Url;
/// use provider::content::provider::Provider;
/// use provider::errors::Result;
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
/// let dummy_provider = DummyProvider {};
/// let mut url = url::Url::from_file_path(env::current_dir().unwrap()).unwrap();
/// url = url.join("samples/hello-world/context.toml").unwrap();
///
/// if let FlowProcess(mut flow) = flowclib::compiler::loader::load(&url.to_string(),
///                                                    &dummy_provider).unwrap() {
///
///     // strip off filename so output_dir is where the context.toml file resides
///     let output_dir = url.join("./").unwrap().to_file_path().unwrap();
///
///     // dump the flows compiler data and dot graph into files alongside the 'context.toml'
///     flowclib::dumper::dump_flow::dump_flow(&flow, &output_dir, &dummy_provider).unwrap();
/// }
/// ```
pub fn dump_flow(flow: &Flow, output_dir: &Path, provider: &dyn Provider) -> io::Result<String> {
    info!(
        "=== Dumper: Dumping flow hierarchy to '{}'",
        output_dir.display()
    );
    _dump_flow(flow, 0, output_dir, provider)
}

/*
    dump the flow definition recursively, tracking what level we are at as we go down
*/
#[allow(clippy::or_fun_call)]
fn _dump_flow(
    flow: &Flow,
    level: usize,
    output_dir: &Path,
    provider: &dyn Provider,
) -> io::Result<String> {
    let filename = Path::new(&flow.source_url)
        .file_stem()
        .ok_or(Error::new(
            ErrorKind::Other,
            "Could not get file_stem of flow definition filename",
        ))?
        .to_str()
        .ok_or(Error::new(
            ErrorKind::Other,
            "Could not convert filename to string",
        ))?;

    let mut writer = helper::create_output_file(&output_dir, filename, "dump")?;
    writer.write_all(format!("\nLevel={}\n{}", level, flow).as_bytes())?;

    writer = helper::create_output_file(&output_dir, filename, "dot")?;
    info!("\tGenerating {}.dot, Use \"dotty\" to view it", filename);
    dump_dot::write_flow_to_dot(flow, &mut writer, output_dir, provider)?;

    // Dump sub-flows
    for subprocess in &flow.subprocesses {
        if let FlowProcess(ref subflow) = subprocess.1 {
            _dump_flow(subflow, level + 1, output_dir, provider)?;
        }
    }

    Ok("All flows dumped".to_string())
}
