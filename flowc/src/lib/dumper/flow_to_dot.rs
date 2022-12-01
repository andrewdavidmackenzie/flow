use std::collections::hash_map::DefaultHasher;
use std::fmt::Write as FormatWrite;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::process::Command;

use log::{debug, info};
use serde_json::Value;
use simpath::{FileType, FoundType, Simpath};
use wax::Glob;

use flowcore::model::connection::Connection;
use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::function_definition::FunctionDefinition;
use flowcore::model::input::InputInitializer::{Always, Once};
use flowcore::model::io::IOSet;
use flowcore::model::name::HasName;
use flowcore::model::process::Process::{FlowProcess, FunctionProcess};
use flowcore::model::route::{HasRoute, Route};
use flowcore::provider::Provider;

use crate::dumper::dump;
use crate::errors::*;

pub(crate) static INPUT_PORTS: &[&str] = &["n", "ne", "nw", "w"];
pub(crate) static OUTPUT_PORTS: &[&str] = &["s", "se", "sw", "e"];

/// Create a .dot format directed graph of a loaded flow definition
///
/// # Example
/// ```
/// use std::env;
/// use url::Url;
/// use flowcore::provider::Provider;
/// use flowcore::meta_provider::MetaProvider;
/// use flowcore::errors::Result;
/// use flowcore::model::process::Process::FlowProcess;
/// use tempdir::TempDir;
/// use std::collections::BTreeSet;
/// use simpath::Simpath;
/// use std::path::PathBuf;
///
/// let lib_search_path = Simpath::new("FLOW_LIB_PATH");
/// let provider = MetaProvider::new(lib_search_path, PathBuf::from("/"));
///
/// let mut url = Url::from_file_path(env::current_dir().unwrap()).unwrap();
/// url = url.join("samples/hello-world/root.toml").unwrap();
///
/// let mut source_urls = BTreeSet::<(Url, Url)>::new();
///
/// if let Ok(FlowProcess(mut flow)) = flowclib::compiler::parser::parse(&url,
///                                                    &provider,
///                                                    &mut source_urls) {
///
///     // strip off filename so output_dir is where the root.toml file resides
///     let output_dir = TempDir::new("flow").unwrap().into_path();
///
///     // dump the flows compiler data and dot graph into files alongside the 'root.toml'
///     flowclib::dumper::flow_to_dot::dump_flow(&flow, &output_dir, &provider).unwrap();
/// }
/// ```
pub fn dump_flow(
    flow: &FlowDefinition,
    output_dir: &Path,
    provider: &dyn Provider,
) -> Result<()> {
    info!(
        "\n=== Dumper: Dumping flow hierarchy to '{}'",
        output_dir.display()
    );
    _dump_flow(flow, 0, output_dir, provider)?;
    Ok(())
}

/// Generate SVG files from any .dot file found below the `root_dir` using the `dot` graphviz
/// executable, if it is found installed on the system within the `$PATH` variable of the user
pub fn generate_svgs(root_dir: &Path, delete_dots: bool) -> Result<()> {
    if let Ok(FoundType::File(dot)) = Simpath::new("PATH").find_type("dot", FileType::File) {
        info!("\n=== Dumper: Generating .dot.svg files from .dot files, using 'dot' command from $PATH");

        let glob = Glob::new("**/*.dot").map_err(|_| "Globbing error")?;
        for entry in glob.walk(root_dir) {
            let entry = entry?;
            let path = entry.path();
            let path_name = path.to_string_lossy();
            let mut output_file = path.to_path_buf();
            output_file.set_extension("dot.svg");
            #[allow(clippy::needless_borrow)]
            if Command::new(&dot)
                .args(vec!["-Tsvg", &format!("-o{}", output_file.display()), &path_name])
                .status()?.success() {
                debug!(".dot.svg successfully generated from {path_name}");
                if delete_dots {
//                    std::fs::remove_file(path)?;
                    debug!("Source file {path_name} was removed after SVG generation")
                }
            } else {
                bail!("Error executing 'dot'");
            }
        }
    } else {
        info!("Could not find 'dot' command in $PATH so SVG generation skipped");
    }

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
    let file_path = flow.source_url.to_file_path()
        .map_err(|_| "Could not get file_stem of flow definition filename")?;
    let filename = file_path
        .file_stem()
        .ok_or("Could not get file_stem of flow definition filename")?
        .to_str()
        .ok_or("Could not convert filename to string")?;

    let mut writer = dump::create_output_file(target_dir, filename, "dot")?;
    info!("\tGenerating {}.dot, Use \"dotty\" to view it", filename);
    write_flow_to_dot(flow, &mut writer)?;

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

fn write_flow_to_dot(
    flow: &FlowDefinition,
    dot_file: &mut dyn Write,
) -> Result<()> {
    dot_file.write_all(digraph_start(flow).as_bytes())?;

    let mut contents = String::new();

    // Inputs
    contents.push_str(&input_set_to_dot(flow.inputs(), flow.route(), false));

    // Outputs
    contents.push_str(&output_set_to_dot(flow.outputs(), flow.route(), false));

    // Process References
    contents.push_str(&process_references_to_dot(flow)?);

    // Connections
    contents.push_str("\n\t// Connections");
    for connection in &flow.connections {
        contents.push_str(&connection_to_dot(connection));
    }

    dot_file.write_all(contents.as_bytes())?;

    dot_file.write_all(digraph_end().as_bytes())?;

    Ok(())
}

/*
    Rotate through the 3 top 'ports' on the sub-flow bubble to try and make inputs separate out
    visually - but this breaks down if we have more than 3 inputs
*/
fn input_set_to_dot(input_set: &IOSet, to: &Route, connect_subflow: bool) -> String {
    let mut string = "\n\t// Inputs\n\t{ rank=source\n".to_string();

    for input in input_set {
        // Avoid creating extra points to connect to for default input
        if input.route() != to {
            // Add an entry for each input using it's route
            let _ = writeln!(string,
                             "\t\"{}\" [label=\"{}\", shape=house, style=filled, fillcolor=white];",
                             input.route(),
                             input.name()
            );

            if connect_subflow {
                // and connect the input to the sub-flow
                let _ = writeln!(string,
                                 "\t\"{}\" -> \"{}\":n [style=invis, headtooltip=\"{}\"];",
                                 input.route(),
                                 to,
                                 input.name()
                );
            }
        }
    }
    string.push_str("\t}\n");

    string
}

/*
    Add the outputs from a flow to add points to connect to
*/
fn output_set_to_dot(output_set: &IOSet, from: &Route, connect_subflow: bool) -> String {
    let mut string = String::new();

    string.push_str("\n\t// Outputs\n\t{ rank=sink\n");
    for output in output_set {
        // Only add output if it's not got the same route as it's function i.e. it's not the default output
        if output.route() != from {
            // Add an entry for each output using it's route
            let _ = writeln!(string, "\t\"{}\" [label=\"{}\", shape=invhouse, style=filled, fillcolor=black, fontcolor=white];",
                             output.route(), output.name());

            if connect_subflow {
                // and connect the output to the sub-flow
                let output_port = output_name_to_port(output.name());
                let _ = writeln!(string,
                                 "\t\"{}\":{} -> \"{}\"[style=invis, headtooltip=\"{}\"];",
                                 from,
                                 output_port,
                                 output.route(),
                                 output.name()
                );
            }
        }
    }
    string.push_str("\t}\n");

    string
}

fn process_references_to_dot(flow: &FlowDefinition) -> Result<String> {
    let mut contents = "\n\t// Process References\n".to_string();
    let file_path = flow.source_url.to_file_path().map_err(|_| "Could not convert Url to file path")?;

    for process_ref in &flow.process_refs {
        let process = flow.subprocesses.get(process_ref.alias())
            .ok_or("Could not find process named in process_ref")?;
        match process {
            FlowProcess(ref subflow) =>
                contents.push_str(&subflow_to_dot(subflow, file_path.clone(),
                                                  subflow.route())?),
            FunctionProcess(ref function) =>
                contents.push_str(&subfunction_to_dot(function, file_path.clone())?),
        }
    }

    Ok(contents)
}

fn subflow_to_dot(flow: &FlowDefinition, parent: PathBuf, flow_route: &str) -> Result<String> {
    let flow_source_path = flow.source_url.to_file_path()
        .map_err(|_| "Could not convert flow's source_url to a File Path")?;
    let relative_path = absolute_to_relative(&flow_source_path, parent)?;
    Ok(format!("\t\"{}\" [label=\"{}\", style=filled, fillcolor=aquamarine, width=2, height=2, URL=\"{relative_path}.dot.svg\"];\n",
               flow_route, flow.alias))
}

fn subfunction_to_dot(function: &FunctionDefinition, parent: PathBuf) -> Result<String> {
    let mut dot_string = String::new();

    let name = if function.name() == function.alias() {
        "".to_string()
    } else {
        format!("\\n({})", function.name())
    };

    let function_source_path = function.get_source_url().to_file_path()
        .map_err(|_| "Could not convert function's source_url to a File Path")?;
    let relative_path = absolute_to_relative(&function_source_path, parent)?;

    // modify path to point to the .html page that's built from .md to document the function
    let md_path = relative_path.replace("toml", "html");
    let _ = writeln!(dot_string,
                     "\t\"{}\" [style=filled, fillcolor=coral, URL=\"{}\", label=\"{}{}\"];",
                     function.route(),
                     md_path,
                     function.alias(),
                     name);

    dot_string.push_str(&input_initializers_to_dot(function, function.route().as_ref()));

    Ok(dot_string)
}

pub (crate) fn input_initializers_to_dot(function: &FunctionDefinition, function_identifier: &str) -> String {
    let mut initializers = "\n\t// Initializers\n".to_string();

    // TODO add initializers for sub-flows also

    for (input_number, input) in function.get_inputs().iter().enumerate() {
        if let Some(initializer) = input.get_initializer() {
            let (value, line_style) = match initializer {
                Always(value) => (value.clone(), "solid"),
                Once(value) => (value.clone(), "dotted"),
            };

            // escape the quotes in the value when converted to string
            let value_string = if let Value::String(value_str) = value {
                format!("\\\"{value_str}\\\"")
            } else {
                format!("{value}")
            };

            // Add a node for the source of the initializer
            let _ = writeln!(initializers,
                             "\t\"initializer{}_{}\"  [style=invis];",
                             function_identifier, input_number
            );

            let input_port = input_name_to_port(input.name());

            // Add connection from hidden node to the input being initialized
            let _ = writeln!(initializers,
                             "\t\"initializer{}_{}\" -> \"{}\":{} [style={}]  [taillabel=\"{}\"] [len=0.1] [color=blue];",
                             function_identifier, input_number,
                             function_identifier, input_port, line_style, value_string);
        }
    }

    initializers
}

fn connection_to_dot(connection: &Connection) -> String {
    // ensure no array index included in the source - just get the input route
    let (from_route, number, array_index) =
        connection.from_io().route().without_trailing_array_index();

    let (from_port, from_name, from_node) = if connection.from_io().flow_io() {
        ("s",
         "", // connect from the "tip" of the flow input pentagon, no need for name
         from_route.to_string())
    } else {
        (output_name_to_port(connection.from_io().name()),
         connection.from_io().name().as_str(),
         strip_io_name(&from_route, connection.from_io().name().as_str()))
    };

    let (to_port, to_name, to_node) = if connection.to_io().flow_io() {
        ("n",
         "", // connect to the tip of the flow output pentagon, no need for name
            connection.to_io().route().to_string()
        )
    } else {
        (input_name_to_port(connection.to_io().name()),
         connection.to_io().name().as_str(),
         strip_io_name(connection.to_io().route(), connection.to_io().name().as_str())
        )
    };

    if array_index {
        format!(
            "\n\t\"{}\":{} -> \"{}\":{} [xlabel=\"{}[{}]\", headlabel=\"{}\"];",
            from_node, from_port,
            to_node, to_port,
            from_name, number, to_name)
    } else {
        format!(
            "\n\t\"{}\":{} -> \"{}\":{} [xlabel=\"{}\", headlabel=\"{}\"];",
            from_node, from_port,
            to_node, to_port,
            from_name, to_name)
    }
}

fn digraph_start(flow: &FlowDefinition) -> String {
    let mut wrapper = String::new();

    // Create a directed graph named after the flow
    let _ = writeln!(wrapper,
        "digraph {} {{",
        str::replace(&flow.alias.to_string(), "-", "_")
    );
    let _ = writeln!(wrapper, "\tlabel=\"{}\";", flow.alias);
    let _ = writeln!(wrapper, "\tlabelloc=t;");
    let _ = writeln!(wrapper, "\tmargin=0.4;");

    wrapper
}

fn digraph_end() -> String {
    "
} // close digraph\n"
        .to_string()
}

fn index_from_name<T: Hash>(t: &T, length: usize) -> usize {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    let index = s.finish() % length as u64;
    index as usize
}

fn input_name_to_port<T: Hash>(t: &T) -> &str {
    INPUT_PORTS[index_from_name(t, INPUT_PORTS.len())]
}

pub(crate) fn output_name_to_port<T: Hash>(t: &T) -> &str {
    OUTPUT_PORTS[index_from_name(t, OUTPUT_PORTS.len())]
}

// Return the route to a node (function, flow) from an IO route by stripping off any IO Name at the end
// TODO Make this a parent() method of Route!!!
fn strip_io_name(route: &Route, name: &str) -> String {
    route.to_string().strip_suffix(&format!("/{name}")).unwrap_or(route).to_string()
}

// figure out a relative path to get to target from source
fn absolute_to_relative(target: &Path, source: PathBuf) -> Result<String> {
//    println!("cargo:warning=source: {}", source.display());
//    println!("cargo:warning=target: {}", target.display());
    let mut current_path = source.parent()
        .ok_or("Could not get directory containing source")?.to_path_buf();
    let mut relative_path_to_root = String::new();
    while !target.starts_with(&current_path) {
        relative_path_to_root.push_str("../");
        if !current_path.pop() {
            bail!("Could not find a common directory to calculate a relative path")
        }
    }
    let sub_path_from_common_point = target.strip_prefix(current_path.as_path())
        .map_err(|_| "Could not calculate sub-path")?;
    relative_path_to_root = relative_path_to_root
        .add(&sub_path_from_common_point.to_string_lossy());
//    println!("cargo:warning=relative: {}", relative_path_to_root);
    Ok(relative_path_to_root)
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use url::Url;

    use crate::dumper::flow_to_dot::absolute_to_relative;

    #[test]
    fn sub_dir_relative_path() {
        let target = Path::new("/Users/andrew/workspace/flow/target/flowsamples/mandlebrot/escapes/escapes.html");
        let parent = Path::new("/Users/andrew/workspace/flow/target/flowsamples/mandlebrot/render.dot.svg");

        let relative = absolute_to_relative(target, parent.to_path_buf())
            .expect("Could not form a relative path");

        assert_eq!(relative, "escapes/escapes.html");
    }

    #[test]
    fn sub_dir_mixed_schemes_relative_path() {
        let target_url = Url::parse("file:///Users/andrew/workspace/flow/target/flowsamples/mandlebrot/escapes/escapes.html")
            .expect("Could not parse Url");
        let target = target_url.to_file_path().expect("Could not convert to file path");
        let parent = Path::new("/Users/andrew/workspace/flow/target/flowsamples/mandlebrot/render.dot.svg");

        let relative = absolute_to_relative(&target, parent.to_path_buf())
            .expect("Could not form a relative path");

        assert_eq!(relative, "escapes/escapes.html");
    }

    #[test]
    fn other_branch_relative_path() {
        let target = Path::new("file:///Users/andrew/workspace/flow/target/flowstdlib/control/index_f.html");
        let parent = Path::new("file:///Users/andrew/workspace/flow/target/flowsamples/mandlebrot/render.dot.svg");

        let relative = absolute_to_relative(target, parent.to_path_buf())
            .expect("Could not form a relative path");

        assert_eq!(relative, "../../flowstdlib/control/index_f.html");
    }
}
