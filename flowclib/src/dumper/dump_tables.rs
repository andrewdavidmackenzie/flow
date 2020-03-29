use std::fmt;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::PathBuf;

use log::info;

use crate::dumper::dump_dot;
use crate::generator::generate::GenerationTables;
use crate::model::flow::Flow;

/// dump a flow's compiler tables that were constructed for use in code generation
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
///         Ok("flow = \"dummy\"".as_bytes().to_owned())
///     }
/// }
///
/// fn main() {
///     let dummy_provider = DummyProvider {};
///     let mut url = url::Url::from_file_path(env::current_dir().unwrap()).unwrap();
///     println!("url = {:?}", url);
///     url = url.join("samples/hello-world-simple/context.toml").unwrap();
///
///     if let FlowProcess(mut flow) = flowclib::compiler::loader::load_root(&url.to_string(),
///                                                           &dummy_provider).unwrap() {
///         let tables = flowclib::compiler::compile::compile(&mut flow).unwrap();
///         let output_dir = tempdir::TempDir::new("flow").unwrap().into_path();
///
///         let tables = flowclib::compiler::compile::compile(&mut flow).unwrap();
///         let output_dir = tempdir::TempDir::new("dumper").unwrap().into_path();
///
///         flowclib::dumper::dump_tables::dump_tables(&tables, &output_dir).unwrap();
///     }
/// }
/// ```
///
pub fn dump_tables(tables: &GenerationTables, output_dir: &PathBuf) -> io::Result<String> {
    info!("==== Dumper: Dumping tables to '{}'", output_dir.display());

    let mut writer = create_output_file(&output_dir, "connections", "dump")?;
    writer.write_all(format!("{}",
                             serde_json::to_string_pretty(&tables.connections)
                                 .unwrap()).as_bytes())?;

    writer = create_output_file(&output_dir, "source_routes", "dump")?;
    writer.write_all(format!("{}",
                             serde_json::to_string_pretty(&tables.source_routes)
                                 .unwrap()).as_bytes())?;

    writer = create_output_file(&output_dir, "destination_routes", "dump")?;
    writer.write_all(format!("{}",
                             serde_json::to_string_pretty(&tables.destination_routes)
                                 .unwrap()).as_bytes())?;

    writer = create_output_file(&output_dir, "collapsed_connections", "dump")?;
    writer.write_all(format!("{}",
                             serde_json::to_string_pretty(&tables.collapsed_connections)
                                 .unwrap()).as_bytes())?;

    writer = create_output_file(&output_dir, "libs", "dump")?;
    writer.write_all(format!("{}",
                             serde_json::to_string_pretty(&tables.libs)
                                 .unwrap()).as_bytes())?;
    Ok("All tables dumped".to_string())
}

/// dump a flow's functions graph as a .dot file to visualize dependencies
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
///     // Return a flow definition for the content for the example
///     fn get_contents(&self, url: &str) -> Result<Vec<u8>> {
///         Ok("flow = \"dummy\"".as_bytes().to_owned())
///     }
/// }
///
/// fn main() {
///     let dummy_provider = DummyProvider {};
///     let mut url = url::Url::from_file_path(env::current_dir().unwrap()).unwrap();
///     println!("url = {:?}", url);
///     url = url.join("samples/hello-world-simple/context.toml").unwrap();
///
///     if let FlowProcess(mut flow) = flowclib::compiler::loader::load_root(&url.to_string(),
///                                                           &dummy_provider).unwrap() {
///         let tables = flowclib::compiler::compile::compile(&mut flow).unwrap();
///         let output_dir = tempdir::TempDir::new("flow").unwrap().into_path();
///
///         flowclib::dumper::dump_tables::dump_functions(&flow, &tables, &output_dir).unwrap();
///     }
/// }
/// ```
pub fn dump_functions(flow: &Flow, tables: &GenerationTables, output_dir: &PathBuf) -> io::Result<String> {
    dump_dot::functions_to_dot(flow, tables, output_dir)?;

    let mut writer = create_output_file(&output_dir, "functions", "dump")?;
    info!("==== Dumper: Dumping functions to functions.dump file in '{}'", output_dir.display());
    dump_table(tables.functions.iter(), &mut writer)?;
    Ok("Functions dumped".to_string())
}

// TODO I can't get output of functions as JSON to work with serde
fn dump_table<C: Iterator>(table: C, writer: &mut dyn Write) -> io::Result<String>
    where <C as Iterator>::Item: fmt::Display {
    for function in table.into_iter() {
        writer.write_all(format!("{}\n", function).as_bytes())?;
    }
    writer.write_all(b"\n")?;
    Ok("table dumped".to_string())
}

fn create_output_file(output_path: &PathBuf, filename: &str, extension: &str) -> io::Result<File> {
    let mut output_file = PathBuf::from(filename);
    output_file.set_extension(extension);
    let mut output_file_path = output_path.clone();
    output_file_path.push(&output_file);
    File::create(&output_file_path)
}
