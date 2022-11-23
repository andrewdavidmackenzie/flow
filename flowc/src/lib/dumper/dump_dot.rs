use std::collections::hash_map::DefaultHasher;
use std::fmt::Write as FormatWrite;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::Path;
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
use flowcore::model::name::{HasName, Name};
use flowcore::model::process::Process::{FlowProcess, FunctionProcess};
use flowcore::model::route::{HasRoute, Route};
use flowcore::provider::Provider;

use crate::compiler::compile::CompilerTables;
use crate::dumper::dump;
use crate::errors::*;

/// Create a directed graph named after the flow, showing all the functions of the flow after it
/// has been compiled down, grouped in sub-clusters
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
/// use std::path::Path;
/// use std::path::PathBuf;
///
/// // Create a lib_search_path including 'context' which is in flowr/src
/// let mut lib_search_path = Simpath::new("TEST_LIBS");
/// let root_str = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
/// let runtime_parent = root_str.join("flowr/src");
/// lib_search_path.add_directory(runtime_parent.to_str().unwrap());
/// let provider = MetaProvider::new(lib_search_path, PathBuf::from("/"));
///
/// let mut url = Url::from_file_path(env::current_dir().unwrap()).unwrap();
/// url = url.join("flowc/tests/test-flows/hello-world/hello-world.toml").unwrap();
///
/// let mut source_urls = BTreeSet::<(Url, Url)>::new();
/// let output_dir = TempDir::new("flow-test").expect("A temp dir").into_path();
///
/// if let Ok(FlowProcess(mut flow)) = flowclib::compiler::parser::parse(&url,
///                                                    &provider,
///                                                    &mut source_urls) {
///     let tables = flowclib::compiler::compile::compile(&flow, &output_dir, false, false,
///                                                       #[cfg(feature = "debugger")] &mut source_urls
///                                                      ).unwrap();
///
///     // strip off filename so output_dir is where the root.toml file resides
///     let output_dir = TempDir::new("flow").unwrap().into_path();
///
///     // create a .dot format directed graph of all the functions after compiling down the flow
///     flowclib::dumper::dump_dot::dump_functions(&flow, &tables, &output_dir).unwrap();
/// }
/// ```
pub fn dump_functions(
    flow: &FlowDefinition,
    tables: &CompilerTables,
    output_dir: &Path,
) -> std::io::Result<()> {
    info!(
        "\n=== Dumper: Dumping functions in dot format to '{}'",
        output_dir.display()
    );
    let mut dot_file = dump::create_output_file(output_dir, "functions", "dot")?;
    info!("\tGenerating functions.dot, Use \"dotty\" to view it");
    dot_file.write_all(
        format!(
            "digraph {} {{\nnodesep=1.0\n",
            str::replace(&flow.alias.to_string(), "-", "_")
        )
            .as_bytes(),
    )?;
    dot_file.write_all(format!("labelloc=t;\nlabel = \"{}\";\n", flow.route()).as_bytes())?;

    let functions = process_refs_to_dot(flow, tables, output_dir).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            "Could not create dot content for process_refs",
        )
    })?;

    dot_file.write_all(functions.as_bytes())?;

    dot_file.write_all(b"}")
}

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
///     flowclib::dumper::dump_dot::dump_flow(&flow, &output_dir, &provider).unwrap();
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

    let mut writer = dump::create_output_file(target_dir, filename, "dot")?;
    info!("\tGenerating {}.dot, Use \"dotty\" to view it", filename);
    write_flow_to_dot(flow, &mut writer, target_dir)?;

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

static INPUT_PORTS: &[&str] = &["n", "ne", "nw", "w"];
static OUTPUT_PORTS: &[&str] = &["s", "se", "sw", "e"];

fn absolute_to_relative(absolute: &str, current_dir: &Path) -> Result<String> {
    let root_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("Could not get parent directory of manifest dir")?;
    let mut current_path = current_dir.to_path_buf();
    let mut path_to_root = String::new();
    // figure out relative path to get to root from output_dir
    while current_path != root_path {
        path_to_root.push_str("../");
        current_path.pop();
    }
    Ok(absolute.replace(&format!("file://{}/", root_path.display()), &path_to_root))
}

fn remove_file_extension(file_path: &str) -> String {
    let splits: Vec<&str> = file_path.split('.').collect();
    if splits.len() > 1 {
        splits[0..splits.len() - 1].join(".")
    } else {
        file_path.to_owned()
    }
}

fn write_flow_to_dot(
    flow: &FlowDefinition,
    dot_file: &mut dyn Write,
    output_dir: &Path,
) -> std::io::Result<()> {
    dot_file.write_all(digraph_wrapper_start(flow).as_bytes())?;

    let mut contents = String::new();

    // Inputs
    contents.push_str(&add_input_set(flow.inputs(), flow.route(), false));

    // Outputs
    contents.push_str(&add_output_set(flow.outputs(), flow.route(), false));

    // Process References
    contents.push_str("\n\t// Process References\n");
    for process_ref in &flow.process_refs {
        let process = flow.subprocesses.get(process_ref.alias()).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Could not find process named in process_ref",
            )
        })?;
        match process {
            FlowProcess(ref flow) => {
                // TODO convert lib reference to a file path or url reference to the actual resource

                let flow_source_str = remove_file_extension(&process_ref.source);

                let relative_path =
                    absolute_to_relative(&flow_source_str, output_dir).map_err(|_| {
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Could not absolute flow_source to a relative path",
                        )
                    })?;
                let flow = format!("\t\"{}\" [label=\"{}\", style=filled, fillcolor=aquamarine, width=2, height=2, URL=\"{relative_path}.dot.svg\"];\n",
                                   flow.route(), process_ref.alias);
                contents.push_str(&flow);
            }
            FunctionProcess(ref function) => {
                contents.push_str(&fn_to_dot(function, output_dir).map_err(|_| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Could not generate dot code for function",
                    )
                })?);
            }
        }
    }

    // Connections
    contents.push_str("\n\t// Connections");
    for connection in &flow.connections {
        contents.push_str(&connection_to_dot(connection));
    }

    dot_file.write_all(contents.as_bytes())?;

    dot_file.write_all(digraph_wrapper_end().as_bytes())
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

fn output_name_to_port<T: Hash>(t: &T) -> &str {
    OUTPUT_PORTS[index_from_name(t, OUTPUT_PORTS.len())]
}

// Return the route to a node (function, flow) from an IO route by stripping off any IO Name at the end
fn node_from_io_route(route: &Route, name: &Name) -> (String, String) {
    (route.to_string().strip_suffix(&name.to_string()).unwrap_or(route).to_string(), name.to_string())
}

#[allow(clippy::ptr_arg)]
fn connection_to_dot(connection: &Connection) -> String {
    let (from_route, number, array_index) =
        connection.from_io().route().without_trailing_array_index();

    let (from_node, from_label) =
        node_from_io_route(&from_route, connection.from_io().name());
    let (to_node, to_label) = node_from_io_route(
        connection.to_io().route(),
        connection.to_io().name(),
    );

    let from_port = if connection.from_io().flow_io() {
        "s"
    } else {
        output_name_to_port(connection.from_io().name())
    };

    let to_port = if connection.to_io().flow_io() {
        "n"
    } else {
        input_name_to_port(connection.to_io().name())
    };

    if array_index {
        format!(
            "\n\t\"{}\":{} -> \"{}\":{} [xlabel=\"{}[{}]\", headlabel=\"{}\"];",
            from_node, from_port, to_node, to_port, from_label, number, to_label
        )
    } else {
        format!(
            "\n\t\"{}\":{} -> \"{}\":{} [xlabel=\"{}\", headlabel=\"{}\"];",
            from_node, from_port, to_node, to_port, from_label, to_label
        )
    }
}

fn digraph_wrapper_start(flow: &FlowDefinition) -> String {
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

fn digraph_wrapper_end() -> String {
    "
} // close digraph\n"
        .to_string()
}

fn fn_to_dot(function: &FunctionDefinition, output_dir: &Path) -> Result<String> {
    let mut dot_string = String::new();

    let name = if function.name() == function.alias() {
        "".to_string()
    } else {
        format!("\\n({})", function.name())
    };

    let relative_path = absolute_to_relative(function.get_source_url().as_ref(), output_dir)?;

    // modify path to point to the .html page that's built from .md to document the function
    let md_path = relative_path.replace("toml", "html");
    let _ = writeln!(dot_string,
                    "\t\"{}\" [style=filled, fillcolor=coral, URL=\"{}\", label=\"{}{}\"]; // function @ route, label = function name",
                    function.route(),
                    md_path,
                    function.alias(),
                    name);

    dot_string.push_str(&input_initializers(function, function.route().as_ref()));

    Ok(dot_string)
}

// Given a Function as used in the code generation - generate a "dot" format string to draw it
fn function_to_dot(function: &FunctionDefinition, functions: &[FunctionDefinition], _output_dir: &Path) -> String {
    let mut function_string = String::new();

    // modify path to point to the .html page that's built from .md to document the function
    let md_path = function
        .get_source_url()
        .to_string()
        .replace("toml", "html");

    let _ = writeln!(function_string,
        "r{}[style=filled, fillcolor=coral, URL=\"{}\", label=\"{} (#{})\"];",
        function.get_id(),
        md_path,
        function.alias(),
        function.get_id()
    );

    function_string.push_str(&input_initializers(
        function,
        &format!("r{}", function.get_id()),
    ));

    // Add edges for each of the outputs of this function to other ones
    for destination in function.get_output_connections() {
        let input_port = INPUT_PORTS[destination.destination_io_number % INPUT_PORTS.len()];
        let destination_function = &functions[destination.destination_id];
        let source_port = output_name_to_port(&destination.source);
        let destination_name = destination_function
            .get_inputs()
            .get(destination.destination_io_number)
            .expect("Could not get input")
            .name()
            .to_string();
        let _ = writeln!(function_string,
                         "r{}:{} -> r{}:{} [taillabel = \"{}\", headlabel = \"{}\"];",
                         function.get_id(),
                         source_port,
                         destination.destination_id,
                         input_port,
                         destination.source,
                         destination_name
        );
    }

    function_string
}

fn input_initializers(function: &FunctionDefinition, function_identifier: &str) -> String {
    let mut initializers = String::new();

    for (input_number, input) in function.get_inputs().iter().enumerate() {
        if let Some(initializer) = input.get_initializer() {
            // Add an extra (hidden) graph entry for the initializer
            let _ = writeln!(initializers,
                "\"initializer{}_{}\"[style=invis];",
                function_identifier, input_number
            );
            let (value, is_constant) = match initializer {
                Always(value) => (value.clone(), true),
                Once(value) => (value.clone(), false),
            };

            let value_string = if let Value::String(value_str) = value {
                format!("\\\"{value_str}\\\"")
            } else {
                format!("{value}")
            };

            let line_style = if is_constant { "solid" } else { "dotted" };

            let input_port = input_name_to_port(input.name());
            // escape the quotes in the value when converted to string
            let _ = writeln!(initializers, "\"initializer{}_{}\" -> \"{}\":{} [style={}] [len=0.1] [color=blue] [label=\"{}\"];",
                                               function_identifier, input_number, function_identifier, input_port, line_style, value_string);
        }
    }

    initializers
}

/*
    Rotate through the 3 top 'ports' on the sub-flow bubble to try and make inputs separate out
    visually - but this breaks down if we have more than 3 inputs
*/
#[allow(clippy::ptr_arg)]
fn add_input_set(input_set: &IOSet, to: &Route, connect_subflow: bool) -> String {
    let mut string = String::new();

    string.push_str("\n\t// Inputs\n\t{ rank=source\n");
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
#[allow(clippy::ptr_arg)]
fn add_output_set(output_set: &IOSet, from: &Route, connect_subflow: bool) -> String {
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

fn output_compiled_function(
    route: &Route,
    tables: &CompilerTables,
    output: &mut String,
    output_dir: &Path,
) {
    for function in &tables.functions {
        if function.route() == route {
            output.push_str(&function_to_dot(function, &tables.functions, output_dir));
        }
    }
}

fn process_refs_to_dot(
    flow: &FlowDefinition,
    tables: &CompilerTables,
    output_dir: &Path,
) -> Result<String> {
    let mut output = String::new();

    // Do the same for all subprocesses referenced from this one
    for process_ref in &flow.process_refs {
        let process = flow
            .subprocesses
            .get(process_ref.alias())
            .ok_or("Could not find process named in process_ref")?;
        match process {
            FlowProcess(ref subflow) => {
                // create cluster sub graph
                let _ = writeln!(output, "\nsubgraph cluster_{} {{",
                    str::replace(&subflow.alias.to_string(), "-", "_")
                );
                let _ = write!(output, "label = \"{}\";", subflow.route());

                output.push_str(&process_refs_to_dot(subflow, tables, output_dir)?); // recurse

                // close cluster
                output.push_str("}\n");
            }
            FunctionProcess(ref function) => {
                output_compiled_function(function.route(), tables, &mut output, output_dir);
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
mod test {
    use super::remove_file_extension;

    #[test]
    fn strip_extension() {
        assert_eq!("file", remove_file_extension("file.toml"));
    }

    #[test]
    fn strip_last_extension_only() {
        assert_eq!("file.my.file", remove_file_extension("file.my.file.toml"));
    }

    #[test]
    fn strip_extension_in_path() {
        assert_eq!(
            "/root/home/file",
            remove_file_extension("/root/home/file.toml")
        );
    }

    #[test]
    fn strip_no_extension() {
        assert_eq!("file", remove_file_extension("file"));
    }
}
