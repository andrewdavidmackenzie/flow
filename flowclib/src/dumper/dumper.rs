use model::flow::Flow;
use std::fmt;
use generator::code_gen::CodeGenTables;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::PathBuf;

/// dump a flow definition that has been loaded to stdout
///
///
/// # Example
/// ```
/// extern crate url;
/// extern crate flowclib;
/// use std::env;
///
/// let mut url = url::Url::from_file_path(env::current_dir().unwrap()).unwrap();
/// println!("url = {:?}", url);
/// url = url.join("samples/hello-world-simple/context.toml").unwrap();
/// let flow = flowclib::loader::loader::load(&url).unwrap();
/// flowclib::dumper::dumper::dump_flow(&flow)
/// ```
///
pub fn dump_flow(flow: &Flow) {
    _dump(flow, 0);
}

/// dump a flow's compiled tables constructed for use in code generation
///
///
/// # Example
/// ```
/// extern crate url;
/// extern crate flowclib;
/// use std::env;
/// use flowclib::compiler::compile;
/// use flowclib::dumper::dumper;
///
/// let mut url = url::Url::from_file_path(env::current_dir().unwrap()).unwrap();
/// println!("url = {:?}", url);
/// url = url.join("samples/hello-world-simple/context.toml").unwrap();
/// let mut flow = flowclib::loader::loader::load(&url).unwrap();
/// let tables = compile::compile(&mut flow).unwrap();
/// dumper::dump_tables(&tables)
/// ```
///
pub fn dump_tables(tables: &CodeGenTables) {
    print(tables.collapsed_connections.iter(), "Collapsed Connections");
    print(tables.runnables.iter(), "Runnables");
    print(tables.libs.iter(), "Libraries");
    print(tables.lib_references.iter(), "Library references");
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
/// use flowclib::loader::loader;
/// use flowclib::compiler::compile;
/// use flowclib::dumper::dumper;
/// use tempdir::TempDir;
///
/// let mut url = url::Url::from_file_path(env::current_dir().unwrap()).unwrap();
/// println!("url = {:?}", url);
/// url = url.join("samples/hello-world-simple/context.toml").unwrap();
/// let mut flow = loader::load(&url).unwrap();
/// let tables = compile::compile(&mut flow).unwrap();
/// let output_dir = TempDir::new("flow").unwrap().into_path();
/// dumper::dump_dot(&flow, &tables, &output_dir).unwrap();
/// ```
///
pub fn dump_dot(flow: &Flow, tables: &CodeGenTables, output_dir: &PathBuf) -> io::Result<String> {
    let mut output_path = output_dir.clone();
    output_path.pop();
    let mut output_file = PathBuf::from(&flow.name);
    output_file.set_extension("dot");
    output_path.push(&output_file);
    info!("Generating dot file at '{}'", output_path.display());

    let mut dot_file = File::create(&output_path)?;
    // Create a directed graph named after the flow
    dot_file.write_all(format!("digraph {} {{\n", str::replace(&flow.name, "-", "_")).as_bytes())?;

    let mut runnables = String::new();
    for (index, ref runnable) in tables.runnables.iter().enumerate() {
        runnables.push_str(&format!("r{}[label=\"{} (#{})\"];\n", index, runnable.name(), runnable.get_id()));

        if let Some(iv) = runnable.get_initial_value() {
            // Add an extra graph entry for the initial value
            runnables.push_str(&format!("iv{}[style=invis] ;\n", index));
            // with a connection to the runnable
            runnables.push_str(&format!("iv{} -> r{} [style=dotted] [color=blue] [label=\"{}\"];\n",
                                        index, index, iv));
        }

        for &(ref output_route, destination_index, _) in runnable.get_output_routes() {
            runnables.push_str(&format!("r{} -> r{} [label = \"{}\"];\n", index, destination_index, output_route));
        }
    }
    dot_file.write_all(runnables.as_bytes())?;

    dot_file.write_all("}".as_bytes())?;

    let mut png_file = output_file.clone();
    png_file.set_extension("png");
    info!("Use \"dot -T png -o {} {}\" to create a PNG file", png_file.display(), output_file.display());
    info!("Use \"dotty {}\" to view it", output_file.display());

    Ok(format!("Dot file written to file '{}'", output_file.display()))
}

fn print<C: Iterator>(table: C, title: &str) where <C as Iterator>::Item: fmt::Display {
    println!("\n{}:", title);
    for e in table.into_iter() {
        println!("{}", e);
    }
}

/*
    dump the flow definition recursively, tracking what leverl we are at as we go down
*/
fn _dump(flow: &Flow, level: usize) {
    println!("\nLevel={}\n{}", level, flow);

    // Dump sub-flows
    if let Some(ref flow_refs) = flow.flow_refs {
        for flow_ref in flow_refs {
            _dump(&flow_ref.flow, level + 1);
        }
    }
}