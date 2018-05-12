use model::flow::Flow;
use std::fmt;
use generator::code_gen::CodeGenTables;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::PathBuf;
use ::dumper::dump_dot;

/// dump a flow definition that has been loaded to a file in the specified output directory
///
///
/// # Example
/// ```
/// extern crate url;
/// extern crate flowclib;
/// extern crate tempdir;
///
/// use std::env;
///
/// fn main() {
///     let mut url = url::Url::from_file_path(env::current_dir().unwrap()).unwrap();
///     url = url.join("samples/hello-world-simple/context.toml").unwrap();
///     let flow = flowclib::loader::loader::load(&"hello-world-simple".to_string(), &url).unwrap();
///     let output_dir = tempdir::TempDir::new("dumper").unwrap().into_path();
///
///     flowclib::dumper::dumper::dump_flow(&flow, &output_dir).unwrap();
/// }
/// ```
///
pub fn dump_flow(flow: &Flow, output_dir: &PathBuf) -> io::Result<String> {
    _dump_flow(flow, 0, output_dir)
}

/*
    dump the flow definition recursively, tracking what lever we are at as we go down
*/
fn _dump_flow(flow: &Flow, level: usize, output_dir: &PathBuf) -> io::Result<String> {
    let mut writer = create_output_file(&output_dir, &flow.alias, "txt")?;
    writer.write_all(format!("\nLevel={}\n{}", level, flow).as_bytes())?;

    writer = create_output_file(&output_dir, &flow.alias, "dot")?;
    dump_dot::dump_flow_dot(flow, level, &mut writer)?;

    // Dump sub-flows
    if let Some(ref flow_refs) = flow.flow_refs {
        for flow_ref in flow_refs {
            _dump_flow(&flow_ref.flow, level + 1, output_dir)?;
        }
    }

    Ok("All flows dumped".to_string())
}

/// dump a flow's compiled tables constructed for use in code generation
///
///
/// # Example
/// ```
/// extern crate url;
/// extern crate flowclib;
/// extern crate tempdir;
///
/// use std::env;
///
/// fn main() {
///     let mut url = url::Url::from_file_path(env::current_dir().unwrap()).unwrap();
///     println!("url = {:?}", url);
///     url = url.join("samples/hello-world-simple/context.toml").unwrap();
///     let mut flow = flowclib::loader::loader::load(&"hello-world-simple".to_string(), &url).unwrap();
///     let tables = flowclib::compiler::compile::compile(&mut flow).unwrap();
///     let output_dir = tempdir::TempDir::new("dumper").unwrap().into_path();
///
///     flowclib::dumper::dumper::dump_tables(&tables, &output_dir).unwrap();
/// }
/// ```
///
pub fn dump_tables(tables: &CodeGenTables, output_dir: &PathBuf) -> io::Result<String> {
    let mut writer = create_output_file(&output_dir, "tables", "txt")?;
    writer.write_all(format!("{}:\n{:#?}\n", "Original Connections", tables.connections).as_bytes())?;
    writer.write_all(format!("{}:\n{:#?}\n", "Source Routes", tables.source_routes).as_bytes())?;
    writer.write_all(format!("{}:\n{:#?}\n", "Destination Routes", tables.destination_routes).as_bytes())?;
    writer.write_all(format!("{}:\n{:#?}\n", "Collapsed Connections", tables.collapsed_connections).as_bytes())?;
    writer.write_all(format!("{}:\n{:#?}\n", "Libraries", tables.libs).as_bytes())?;
    writer.write_all(format!("{}:\n{:#?}\n", "Library references", tables.lib_references).as_bytes())?;
    Ok("All tables dumped".to_string())
}

/// dump a flow's runnables graph as a .dot file to visualize dependencies
///
///
/// # Example
/// ```
/// extern crate url;
/// extern crate flowclib;
/// extern crate tempdir;
///
/// use std::env;
///
/// fn main() {
///     let mut url = url::Url::from_file_path(env::current_dir().unwrap()).unwrap();
///     println!("url = {:?}", url);
///     url = url.join("samples/hello-world-simple/context.toml").unwrap();
///     let mut flow = flowclib::loader::loader::load(&"hello-world-simple".to_string(), &url).unwrap();
///     let tables = flowclib::compiler::compile::compile(&mut flow).unwrap();
///     let output_dir = tempdir::TempDir::new("flow").unwrap().into_path();
///
///     flowclib::dumper::dumper::dump_runnables(&flow, &tables, &output_dir).unwrap();
/// }
/// ```
///
pub fn dump_runnables(flow: &Flow, tables: &CodeGenTables, output_dir: &PathBuf) -> io::Result<String> {
    let mut writer = create_output_file(&output_dir, "runnables", "dot")?;
    info!("Generating Runnables dot file {}, Use \"dotty\" to view it", output_dir.display());
    dump_dot::runnables_to_dot(&flow.alias, tables, &mut writer)?;

    writer = create_output_file(&output_dir, "runnables", "txt")?;
    dump_table(tables.runnables.iter(), "Runnables", &mut writer)
}

fn dump_table<C: Iterator>(table: C, title: &str, writer: &mut Write) -> io::Result<String>
    where <C as Iterator>::Item: fmt::Display {
    writer.write_all(format!("{}:\n", title).as_bytes())?;
    for e in table.into_iter() {
        writer.write_all(format!("\t{}\n", e).as_bytes())?;
    }
    writer.write_all(b"\n")?;
    Ok("printed".to_string())
}

fn create_output_file(output_path: &PathBuf, filename: &str, extension: &str) -> io::Result<File> {
    let mut output_file = PathBuf::from(filename);
    output_file.set_extension(extension);
    let mut output_file_path = output_path.clone();
    output_file_path.push(&output_file);
    File::create(&output_file_path)
}
