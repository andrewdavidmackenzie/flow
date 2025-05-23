use std::fmt::Write as _;
use std::io::Write;
use std::path::Path;

use log::info;

use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::function_definition::FunctionDefinition;
use flowcore::model::name::HasName;
use flowcore::model::process::Process::{FlowProcess, FunctionProcess};
use flowcore::model::route::{HasRoute, Route};

use crate::compiler::compile::CompilerTables;
use crate::dumper::create_output_file;
use crate::dumper::flow_to_dot::{input_initializers_to_dot, INPUT_PORTS, output_name_to_port};
use crate::errors::Result;

/// Create a directed graph named after the flow, showing all the functions of the flow after it
/// has been compiled down, grouped in sub-clusters
///
/// # Errors
///
/// Returns an error if the `FlowDefinition` cannot be dumped to file(s) for one of these reasons:
/// - The output file in `output_dir` could not be created or written to
///
/// # Example
/// ```
/// use std::env;
/// use url::Url;
/// use flowcore::provider::Provider;
/// use flowcore::meta_provider::MetaProvider;
/// use flowcore::errors::Result;
/// use flowcore::model::process::Process::FlowProcess;
/// use tempfile::tempdir;
/// use std::collections::BTreeMap;
/// use simpath::Simpath;
/// use std::path::Path;
/// use std::path::PathBuf;
///
/// // Create a lib_search_path
/// let mut lib_search_path = Simpath::new("TEST_LIBS");
/// // Add a runner's 'context root' directory, such as '$FLOW_DIR/flowr/src/bin/flowrcli/context'
/// let root_str = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
/// let runtime_parent = root_str.join("flowr/src/bin/flowrcli/context");
/// lib_search_path.add_directory(runtime_parent.to_str().unwrap());
/// let provider = MetaProvider::new(lib_search_path, PathBuf::from("/"));
///
/// let mut url = Url::from_file_path(env::current_dir().unwrap()).unwrap();
/// url = url.join("flowc/tests/test-flows/hello-world/hello-world.toml").unwrap();
///
/// let output_dir = tempdir().expect("A temp dir").keep();
///
/// if let Ok(FlowProcess(mut flow)) = flowrclib::compiler::parser::parse(&url,
///                                                    &provider) {
///     let mut source_urls = BTreeMap::<String, Url>::new();
///     let tables = flowrclib::compiler::compile::compile(&flow, &output_dir, false, false,
///                                                         &mut source_urls).unwrap();
///
///     // strip off filename so output_dir is where the root.toml file resides
///     let output_dir = tempdir().unwrap().keep();
///
///     // create a .dot format directed graph of all the functions after compiling down the flow
///     flowrclib::dumper::functions_to_dot::dump_functions(&flow, &tables, &output_dir).unwrap();
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
    let mut dot_file = create_output_file(output_dir, "functions", "dot")?;
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
        std::io::Error::other("Could not create dot content for process_refs")
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
                let _ = write!(output, "\nsubgraph cluster_{} {{",
                                 str::replace(&subflow.alias.to_string(), "-", "_"));
                let _ = write!(output, "label = \"{}\";", subflow.route());

                let _ = write!(output, "{}", &process_refs_to_dot(subflow, tables)?); // recurse

                // close cluster
                let _ = writeln!(output, "}}");
            }
            FunctionProcess(ref function) => {
                output_compiled_function(function.route(), tables, &mut output)?;
            }
        }
    }

    Ok(output)
}

// Given a Function as used in the code generation - generate a "dot" format string to draw it
fn function_to_dot(function: &FunctionDefinition, functions: &[FunctionDefinition]) -> Result<String> {
    let mut function_string = String::new();

    // modify path to point to the .html page that's built from .md to document the function
    let md_path = function
        .get_source_url()
        .to_string()
        .replace("toml", "html");

    let _ = write!(function_string,
                     "r{}[style=filled, fillcolor=coral, URL=\"{}\", label=\"{} (#{})\"];",
                     function.get_id(),
                     md_path,
                     function.alias(),
                     function.get_id()
    );

    function_string.push_str(&input_initializers_to_dot(function, &format!("r{}", function.get_id()))?);

    // Add edges for each of the outputs of this function to other ones
    for destination in function.get_output_connections() {
        let input_port = INPUT_PORTS.get(destination.destination_io_number % INPUT_PORTS.len())
            .ok_or("Could no tget Input Port")?;
        let destination_function = functions.get(destination.destination_id).ok_or("Could not get function")?;
        let source_port = output_name_to_port(&destination.source)?;
        let destination_name = destination_function
            .get_inputs()
            .get(destination.destination_io_number)
            .expect("Could not get input")
            .name()
            .to_string();
        let _ = write!(function_string,
                         "r{}:{} -> r{}:{} [taillabel = \"{}\", headlabel = \"{}\"];",
                         function.get_id(),
                         source_port,
                         destination.destination_id,
                         input_port,
                         destination.source,
                         destination_name
        );
    }

    Ok(function_string)
}

fn output_compiled_function(
    route: &Route,
    tables: &CompilerTables,
    output: &mut String,
) -> Result<()>{
    for function in &tables.functions {
        if function.route() == route {
            output.push_str(&function_to_dot(function, &tables.functions)?);
        }
    }

    Ok(())
}
