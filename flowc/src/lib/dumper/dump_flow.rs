use std::io::Write;
use std::path::Path;

use log::info;

use provider::lib_provider::LibProvider;

use crate::dumper::dump_dot;
use crate::model::flow::Flow;
use crate::model::process::Process::FlowProcess;

use super::dump_tables;

/// Dump a human readable representation of loaded flow definition (in a `Flow` structure) to a
/// file in the specified output directory
///
/// # Example
/// ```
/// use std::env;
/// use url::Url;
/// use provider::lib_provider::LibProvider;
/// use provider::errors::Result;
/// use flowclib::model::process::Process::FlowProcess;
///
/// struct DummyProvider {}
///
/// impl LibProvider for DummyProvider {
///     fn resolve_url(&self, url: &Url, default_filename: &str, _ext: &[&str]) -> Result<(Url, Option<String>)> {
///         Ok((url.clone(), None))
///     }
///
///     fn get_contents(&self, url: &Url) -> Result<Vec<u8>> {
///         Ok("flow = \"dummy\"\n[[input]]".as_bytes().to_owned())
///     }
/// }
///
/// let dummy_provider = DummyProvider {};
/// let mut url = url::Url::from_file_path(env::current_dir().unwrap()).unwrap();
/// url = url.join("samples/hello-world/context.toml").unwrap();
///
/// if let FlowProcess(mut flow) = flowclib::compiler::loader::load(&url,
///                                                    &dummy_provider).unwrap() {
///
///     // strip off filename so output_dir is where the context.toml file resides
///     let output_dir = url.join("./").unwrap().to_file_path().unwrap();
///
///     // dump the flows compiler data and dot graph into files alongside the 'context.toml'
///     flowclib::dumper::dump_flow::dump_flow(&flow, &output_dir, &dummy_provider).unwrap();
/// }
/// ```
pub fn dump_flow(
    flow: &Flow,
    output_dir: &Path,
    provider: &dyn LibProvider,
) -> std::io::Result<()> {
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
    provider: &dyn LibProvider,
) -> std::io::Result<()> {
    let file_path = flow.source_url.to_file_path().map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            "Could not get file_stem of flow definition filename",
        )
    })?;
    let filename = file_path
        .file_stem()
        .ok_or(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Could not get file_stem of flow definition filename",
        ))?
        .to_str()
        .ok_or(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Could not convert filename to string",
        ))?;

    let mut writer = dump_tables::create_output_file(&output_dir, filename, "dump")?;
    writer.write_all(format!("\nLevel={}\n{}", level, flow).as_bytes())?;

    writer = dump_tables::create_output_file(&output_dir, filename, "dot")?;
    info!("\tGenerating {}.dot, Use \"dotty\" to view it", filename);
    dump_dot::write_flow_to_dot(flow, &mut writer, output_dir, provider)?;

    // Dump sub-flows
    for subprocess in &flow.subprocesses {
        if let FlowProcess(ref subflow) = subprocess.1 {
            _dump_flow(subflow, level + 1, output_dir, provider)?;
        }
    }

    Ok(())
}
