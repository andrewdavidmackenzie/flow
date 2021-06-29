use std::fmt;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use log::info;

use crate::dumper::dump_dot;
use crate::generator::generate::GenerationTables;
use crate::model::flow::Flow;
use crate::model::route::HasRoute;

/// Dump the compiler tables of a loaded flow in human readable format to a specified
/// output directory.
///
/// # Example
/// ```
/// use std::env;
/// use url::Url;
/// use flowcore::lib_provider::{Provider, MetaProvider};
/// use flowcore::errors::Result;
/// use flowclib::model::process::Process::FlowProcess;
/// use flowcore::lib_provider::LibProvider;
/// use std::collections::HashSet;
/// use simpath::Simpath;
///
/// let lib_search_path = Simpath::new("FLOW_LIB_PATH");
/// let provider = MetaProvider::new(lib_search_path);
///
/// let mut url = url::Url::from_file_path(env::current_dir().unwrap()).unwrap();
/// url = url.join("samples/hello-world/context.toml").unwrap();
///
/// let mut source_urls = HashSet::<(Url, Url)>::new();
///
/// if let Ok(FlowProcess(mut flow)) = flowclib::compiler::loader::load(&url,
///                                                           &provider,
///                                                           &mut source_urls) {
///     let tables = flowclib::compiler::compile::compile(&mut flow).unwrap();
///     let output_dir = tempdir::TempDir::new("flow").unwrap().into_path();
///
///     let tables = flowclib::compiler::compile::compile(&mut flow).unwrap();
///     let output_dir = tempdir::TempDir::new("dumper").unwrap().into_path();
///
///     flowclib::dumper::dump_tables::dump_tables(&tables, &output_dir).unwrap();
/// }
/// ```
///
pub fn dump_tables(tables: &GenerationTables, output_dir: &Path) -> std::io::Result<()> {
    info!("=== Dumper: Dumping tables to '{}'", output_dir.display());

    let mut writer = create_output_file(&output_dir, "connections", "dump")?;
    info!("\tGenerating connections.dump");
    writer.write_all(serde_json::to_string_pretty(&tables.connections)?.as_bytes())?;

    writer = create_output_file(&output_dir, "source_routes", "dump")?;
    info!("\tGenerating source_routes.dump");
    writer.write_all(serde_json::to_string_pretty(&tables.source_routes)?.as_bytes())?;

    writer = create_output_file(&output_dir, "destination_routes", "dump")?;
    info!("\tGenerating destination_routes.dump");
    writer.write_all(serde_json::to_string_pretty(&tables.destination_routes)?.as_bytes())?;

    writer = create_output_file(&output_dir, "collapsed_connections", "dump")?;
    info!("\tGenerating collapsed_connections.dump");
    writer.write_all(serde_json::to_string_pretty(&tables.collapsed_connections)?.as_bytes())?;

    writer = create_output_file(&output_dir, "libs", "dump")?;
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

/*
    Create a directed graph named after the flow, adding functions grouped in sub-clusters
*/
fn functions_to_dot(
    flow: &Flow,
    tables: &GenerationTables,
    output_dir: &Path,
) -> std::io::Result<()> {
    info!(
        "=== Dumper: Dumping functions to '{}'",
        output_dir.display()
    );
    let mut dot_file = create_output_file(&output_dir, "functions", "dot")?;
    info!("\tGenerating functions.dot, Use \"dotty\" to view it");
    dot_file.write_all(
        format!(
            "digraph {} {{\nnodesep=1.0\n",
            str::replace(&flow.alias.to_string(), "-", "_")
        )
        .as_bytes(),
    )?;
    dot_file.write_all(&format!("labelloc=t;\nlabel = \"{}\";\n", flow.route()).as_bytes())?;

    let functions = dump_dot::process_refs_to_dot(flow, tables, output_dir).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            "Could not create dot content for process_refs",
        )
    })?;

    dot_file.write_all(functions.as_bytes())?;

    dot_file.write_all(b"}")
}

/// dump a flow's functions graph as a .dot file to visualize dependencies
///
///
/// # Example
/// ```
/// use std::env;
/// use url::Url;
/// use flowcore::lib_provider::{Provider, MetaProvider};
/// use flowcore::errors::Result;
/// use flowclib::model::process::Process::FlowProcess;
/// use flowcore::lib_provider::LibProvider;
/// use std::collections::HashSet;
/// use simpath::Simpath;
///
/// let lib_search_path = Simpath::new("FLOW_LIB_PATH");
/// let provider = MetaProvider::new(lib_search_path);
/// let mut url = url::Url::from_file_path(env::current_dir().unwrap()).unwrap();
/// url = url.join("samples/hello-world/context.toml").unwrap();
///
/// let mut source_urls = HashSet::<(Url, Url)>::new();
///
/// if let Ok(FlowProcess(mut flow)) = flowclib::compiler::loader::load(&url,
///                                                           &provider,
///                                                           &mut source_urls) {
///     let tables = flowclib::compiler::compile::compile(&mut flow).unwrap();
///     let output_dir = tempdir::TempDir::new("flow").unwrap().into_path();
///
///     flowclib::dumper::dump_tables::dump_functions(&flow, &tables, &output_dir).unwrap();
/// }
/// ```
pub fn dump_functions(
    flow: &Flow,
    tables: &GenerationTables,
    output_dir: &Path,
) -> std::io::Result<()> {
    functions_to_dot(flow, tables, output_dir)?;

    let mut writer = create_output_file(&output_dir, "functions", "dump")?;
    info!("\tGenerating functions.dump");
    dump_table(tables.functions.iter(), &mut writer)
}

// TODO I can't get output of functions as JSON to work with serde
fn dump_table<C: Iterator>(table: C, writer: &mut dyn Write) -> std::io::Result<()>
where
    <C as Iterator>::Item: fmt::Display,
{
    for function in table.into_iter() {
        writer.write_all(format!("{}\n", function).as_bytes())?;
    }
    writer.write_all(b"\n")
}
