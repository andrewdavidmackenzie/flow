use std::fmt;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use log::{debug, info};

use flowcore::meta_provider::Provider;
use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::process::Process::FlowProcess;

use crate::compiler::compile::CompilerTables;
use crate::dumper::{dump, dump_dot};
use crate::errors::*;

/// Dump the compiler tables of a loaded flow in human readable format to a specified
/// output directory.
///
/// # Example
/// ```
/// use std::env;
/// use url::Url;
/// use flowcore::meta_provider::{Provider, MetaProvider};
/// use flowcore::errors::Result;
/// use flowcore::model::process::Process::FlowProcess;
/// use std::collections::HashSet;
/// use simpath::Simpath;
/// use std::path::PathBuf;
/// use tempdir::TempDir;
///
/// let lib_search_path = Simpath::new("FLOW_LIB_PATH");
/// let provider = MetaProvider::new(lib_search_path, PathBuf::from("/"));
///
/// let mut url = url::Url::from_file_path(env::current_dir().unwrap()).unwrap();
/// url = url.join("samples/hello-world/root.toml").unwrap();
///
/// let mut source_urls = HashSet::<(Url, Url)>::new();
/// let output_dir = TempDir::new("flow-test").expect("A temp dir").into_path();
///
/// if let Ok(FlowProcess(mut flow)) = flowclib::compiler::loader::load(&url,
///                                                           &provider,
///                                                           &mut source_urls) {
///     let tables = flowclib::compiler::compile::compile(&mut flow,
///                                                       &output_dir, true,
///                                                       #[cfg(feature = "debugger")] &mut source_urls
///                                                       ).unwrap();
///
///     flowclib::dumper::dump::dump_tables(&tables, &output_dir).unwrap();
/// }
/// ```
///
pub fn dump_tables(tables: &CompilerTables, output_dir: &Path) -> std::io::Result<()> {
    info!("\n=== Dumper: Dumping tables to '{}'", output_dir.display());

    let mut writer = create_output_file(output_dir, "connections", "dump")?;
    info!("\tGenerating connections.dump");
    writer.write_all(serde_json::to_string_pretty(&tables.connections)?.as_bytes())?;

    writer = create_output_file(output_dir, "source_routes", "dump")?;
    info!("\tGenerating source_routes.dump");
    writer.write_all(serde_json::to_string_pretty(&tables.sources)?.as_bytes())?;

    writer = create_output_file(output_dir, "destination_routes", "dump")?;
    info!("\tGenerating destination_routes.dump");
    writer.write_all(serde_json::to_string_pretty(&tables.destination_routes)?.as_bytes())?;

    writer = create_output_file(output_dir, "collapsed_connections", "dump")?;
    info!("\tGenerating collapsed_connections.dump");
    writer.write_all(serde_json::to_string_pretty(&tables.collapsed_connections)?.as_bytes())?;

    writer = create_output_file(output_dir, "libs", "dump")?;
    info!("\tGenerating libs.dump");
    writer.write_all(serde_json::to_string_pretty(&tables.libs)?.as_bytes())
}

/// Create a file at the specified `output_path`, `filename` and `extension` that output will be dumped to
pub fn create_output_file(
    output_path: &Path,
    filename: &str,
    extension: &str,
) -> std::io::Result<File> {
    let mut output_file = PathBuf::from(filename);
    output_file.set_extension(extension);
    let mut output_file_path = output_path.to_path_buf();
    output_file_path.push(&output_file);
    File::create(&output_file_path)
}

/// dump a flow's functions graph as a .dot file to visualize dependencies
///
///
/// # Example
/// ```
/// use std::env;
/// use url::Url;
/// use flowcore::meta_provider::{Provider, MetaProvider};
/// use flowcore::errors::Result;
/// use flowcore::model::process::Process::FlowProcess;
/// use std::collections::HashSet;
/// use simpath::Simpath;
/// use std::path::PathBuf;
///
/// let lib_search_path = Simpath::new("FLOW_LIB_PATH");
/// let provider = MetaProvider::new(lib_search_path, PathBuf::from("/"));
/// let mut url = url::Url::from_file_path(env::current_dir().unwrap()).unwrap();
/// url = url.join("samples/hello-world/root.toml").unwrap();
///
/// let mut source_urls = HashSet::<(Url, Url)>::new();
/// let output_dir = tempdir::TempDir::new("flow").unwrap().into_path();
///
/// if let Ok(FlowProcess(mut flow)) = flowclib::compiler::loader::load(&url,
///                                                           &provider,
///                                                           &mut source_urls) {
///     let tables = flowclib::compiler::compile::compile(&mut flow,
///                                                       &output_dir, true,
///                                                       #[cfg(feature = "debugger")] &mut source_urls
///                                                      ).unwrap();
///
///     flowclib::dumper::dump::dump_functions(&flow, &tables, &output_dir).unwrap();
/// }
/// ```
pub fn dump_functions(
    flow: &FlowDefinition,
    tables: &CompilerTables,
    output_dir: &Path,
) -> std::io::Result<()> {
    info!("\n=== Dumper: Generating {}/functions.dump", output_dir.display());
    dump_dot::dump_functions(flow, tables, output_dir)?;

    let mut writer = create_output_file(output_dir, "functions", "dump")?;
    dump_table(tables.functions.iter(), &mut writer)
}

fn dump_table<C: Iterator>(table: C, writer: &mut dyn Write) -> std::io::Result<()>
    where
        <C as Iterator>::Item: fmt::Display,
{
    for function in table {
        writer.write_all(format!("{}\n", function).as_bytes())?;
    }
    writer.write_all(b"\n")
}

/// Dump a human readable representation of loaded flow definition to a file in `output_dir`
///
/// # Example
/// ```
/// use std::env;
/// use url::Url;
/// use flowcore::meta_provider::{Provider, MetaProvider};
/// use flowcore::errors::Result;
/// use flowcore::model::process::Process::FlowProcess;
/// use tempdir::TempDir;
/// use std::collections::HashSet;
/// use simpath::Simpath;
/// use std::path::PathBuf;
///
/// let lib_search_path = Simpath::new("FLOW_LIB_PATH");
/// let provider = MetaProvider::new(lib_search_path, PathBuf::from("/"));
///
/// let mut url = url::Url::from_file_path(env::current_dir().unwrap()).unwrap();
/// url = url.join("samples/hello-world/root.toml").unwrap();
///
/// let mut source_urls = HashSet::<(Url, Url)>::new();
/// if let Ok(FlowProcess(mut flow)) = flowclib::compiler::loader::load(&url,
///                                                    &provider,
///                                                    &mut source_urls) {
///
///     // strip off filename so output_dir is where the root.toml file resides
///     let output_dir = TempDir::new("flow").unwrap().into_path();
///
///     // dump the flows compiler data and dot graph into files alongside the 'root.toml'
///     flowclib::dumper::dump::dump_flow(&flow, &output_dir, &provider).unwrap();
/// }
/// ```
pub fn dump_flow(
    flow: &FlowDefinition,
    output_dir: &Path,
    provider: &dyn Provider
) -> Result<()> {
    info!(
        "\n=== Dumper: Dumping flow hierarchy to '{}' folder",
        output_dir.display()
    );
    _dump_flow(flow, 0, output_dir, provider)?;
    Ok(())
}

/*
    dump the flow definition recursively, tracking what level we are at as we go down
*/
#[allow(clippy::only_used_in_recursion)]
fn _dump_flow(
    flow: &FlowDefinition,
    level: usize,
    target_dir: &Path,
    provider: &dyn Provider
) -> Result<()> {
    let file_path = flow.source_url.to_file_path().map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            "Could not get file_stem of flow definition filename",
        )
    })?;
    let filename = file_path
        .file_stem()
        .ok_or_else(|| std::io::Error::new(
            std::io::ErrorKind::Other,
            "Could not get file_stem of flow definition filename",
        ))?
        .to_str()
        .ok_or_else(|| std::io::Error::new(
            std::io::ErrorKind::Other,
            "Could not convert filename to string",
        ))?;

    debug!("Dumping tables to {}", filename);
    let mut writer = dump::create_output_file(target_dir, filename, "dump")?;
    writer.write_all(format!("\nLevel={}\n{}", level, flow).as_bytes())?;

    // Dump sub-flows
    for subprocess in &flow.subprocesses {
        if let FlowProcess(ref subflow) = subprocess.1 {
            _dump_flow(
                subflow,
                level + 1,
                target_dir,
                provider
            )?;
        }
    }

    Ok(())
}
