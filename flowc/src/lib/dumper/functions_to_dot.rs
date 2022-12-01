use std::io::Write;
use std::path::Path;

use log::info;

use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::function_definition::FunctionDefinition;
use flowcore::model::name::HasName;
use flowcore::model::process::Process::{FlowProcess, FunctionProcess};
use flowcore::model::route::{HasRoute, Route};

use crate::compiler::compile::CompilerTables;
use crate::dumper::dump;
use crate::dumper::flow_to_dot::{input_initializers_to_dot, INPUT_PORTS, output_name_to_port};
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
///     flowclib::dumper::functions_to_dot::dump_functions(&flow, &tables, &output_dir).unwrap();
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

    let functions = process_refs_to_dot(flow, tables).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            "Could not create dot content for process_refs",
        )
    })?;

    dot_file.write_all(functions.as_bytes())?;

    dot_file.write_all(b"}")
}

fn process_refs_to_dot(
    flow: &FlowDefinition,
    tables: &CompilerTables,
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
                output.push_str(&format!("\nsubgraph cluster_{} {{",
                                 str::replace(&subflow.alias.to_string(), "-", "_"))
                );
                output.push_str(&format!("label = \"{}\";", subflow.route()));

                output.push_str(&process_refs_to_dot(subflow, tables)?); // recurse

                // close cluster
                output.push_str("}\n");
            }
            FunctionProcess(ref function) => {
                output_compiled_function(function.route(), tables, &mut output);
            }
        }
    }

    Ok(output)
}

// Given a Function as used in the code generation - generate a "dot" format string to draw it
fn function_to_dot(function: &FunctionDefinition, functions: &[FunctionDefinition]) -> String {
    let mut function_string = String::new();

    // modify path to point to the .html page that's built from .md to document the function
    let md_path = function
        .get_source_url()
        .to_string()
        .replace("toml", "html");

    function_string.push_str(&format!(
                     "r{}[style=filled, fillcolor=coral, URL=\"{}\", label=\"{} (#{})\"];",
                     function.get_id(),
                     md_path,
                     function.alias(),
                     function.get_id()
    ));

    function_string.push_str(&input_initializers_to_dot(
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
        function_string.push_str(&format!(
                         "r{}:{} -> r{}:{} [taillabel = \"{}\", headlabel = \"{}\"];",
                         function.get_id(),
                         source_port,
                         destination.destination_id,
                         input_port,
                         destination.source,
                         destination_name
        ));
    }

    function_string
}

fn output_compiled_function(
    route: &Route,
    tables: &CompilerTables,
    output: &mut String,
) {
    for function in &tables.functions {
        if function.route() == route {
            output.push_str(&function_to_dot(function, &tables.functions));
        }
    }
}
