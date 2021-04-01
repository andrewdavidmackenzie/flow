use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::prelude::*;
use std::path::Path;

use serde_json::Value;
use url::Url;

use flowrstructs::input::InputInitializer::{Always, Once};
use provider::lib_provider::LibProvider;

use crate::deserializers::deserializer_helper::get_file_extension;
use crate::errors::*;
use crate::generator::generate::GenerationTables;
use crate::model::connection::Connection;
use crate::model::flow::Flow;
use crate::model::function::Function;
use crate::model::io::{Find, IOSet};
use crate::model::name::{HasName, Name};
use crate::model::process::Process::{FlowProcess, FunctionProcess};
use crate::model::route::{HasRoute, Route};

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

pub fn write_flow_to_dot(
    flow: &Flow,
    dot_file: &mut dyn Write,
    output_dir: &Path,
    provider: &dyn LibProvider,
) -> std::io::Result<()> {
    dot_file.write_all(digraph_wrapper_start(flow).as_bytes())?;

    let mut contents = String::new();

    // Inputs
    contents.push_str(&add_input_set(&flow.inputs(), flow.route(), false));

    // Outputs
    contents.push_str(&add_output_set(&flow.outputs(), flow.route(), false));

    // Process References
    contents.push_str("\n\t// Process References\n");
    if let Some(process_refs) = &flow.process_refs {
        for process_ref in process_refs {
            let process = flow.subprocesses.get(process_ref.alias()).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Could not find process named in process_ref",
                )
            })?;
            match process {
                FlowProcess(ref flow) => {
                    let (flow_source, _) = provider
                        .resolve_url(
                            &Url::parse(&process_ref.source).map_err(|_| {
                                std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    "Could not parse Url from flow_source",
                                )
                            })?,
                            "",
                            &[""],
                        )
                        .map_err(|_| {
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "Could not resolve Url of flow_source",
                            )
                        })?;

                    // remove file extension when forming URL as is of form {file_stem}.dot.svg
                    let mut flow_source_str = flow_source.to_string();
                    if let Some(extension) = get_file_extension(&flow_source) {
                        flow_source_str.truncate(flow_source_str.len() - (extension.len() + 1))
                    }

                    let relative_path = absolute_to_relative(&flow_source_str, output_dir)
                        .map_err(|_| {
                            std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "Could not absolute flow_source to a relative path",
                            )
                        })?;
                    let flow = format!("\t\"{}\" [label=\"{}\", style=filled, fillcolor=aquamarine, width=2, height=2, URL=\"{}.dot.svg\"];\n",
                                       flow.route(), process_ref.alias, relative_path);
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
    }

    // Connections
    if let Some(connections) = &flow.connections {
        contents.push_str("\n\t// Connections");
        for connection in connections {
            contents.push_str(&connection_to_dot(
                &connection,
                &flow.inputs(),
                &flow.outputs(),
            ));
        }
    }

    dot_file.write_all(contents.as_bytes())?;

    dot_file.write_all(&digraph_wrapper_end().as_bytes())
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

fn connection_to_dot(connection: &Connection, input_set: &IOSet, output_set: &IOSet) -> String {
    let (from_route, number, array_index) =
        connection.from_io.route().without_trailing_array_index();

    let (from_node, from_label) =
        node_from_io_route(&from_route, connection.from_io.name(), input_set);
    let (to_node, to_label) = node_from_io_route(
        &connection.to_io.route(),
        &connection.to_io.name(),
        output_set,
    );

    let from_port = if connection.from_io.flow_io() {
        "s"
    } else {
        output_name_to_port(connection.from_io.name())
    };

    let to_port = if connection.to_io.flow_io() {
        "n"
    } else {
        input_name_to_port(connection.to_io.name())
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

/*
    Return the route to a node (value, function, flow) for a given route, by:

    If the input or output name IS the default one ("" empty string), then just return the route.

    If the input or output IS NOT the default one ("" empty string) then remove the IO name from the
    route and return that.
*/
fn node_from_io_route(route: &Route, name: &Name, io_set: &IOSet) -> (String, String) {
    let label = if !io_set.find(route) {
        name.to_string()
    } else {
        "".to_string()
    };

    if name.is_empty() || io_set.find(route) {
        (route.to_string(), label)
    } else {
        let length_without_io_name = route.len() - name.len() - 1; // 1 for '/'
        (
            route.to_string()[..length_without_io_name].to_string(),
            label,
        )
    }
}

fn digraph_wrapper_start(flow: &Flow) -> String {
    let mut wrapper = String::new();

    // Create a directed graph named after the flow
    wrapper.push_str(&format!(
        "digraph {} {{\n",
        str::replace(&flow.alias.to_string(), "-", "_")
    ));
    wrapper.push_str(&format!("\tlabel=\"{}\";\n", flow.alias));
    wrapper.push_str("\tlabelloc=t;\n");
    wrapper.push_str("\tmargin=0.4;\n");

    wrapper
}

fn digraph_wrapper_end() -> String {
    "
} // close digraph\n"
        .to_string()
}

fn fn_to_dot(function: &Function, output_dir: &Path) -> Result<String> {
    let mut dot_string = String::new();

    let name = if function.name() == function.alias() {
        "".to_string()
    } else {
        format!("\\n({})", function.name())
    };

    let relative_path = absolute_to_relative(function.get_source_url(), output_dir)?;

    // modify path to point to the .html page that's built from .md to document the function
    let md_path = relative_path.replace("toml", "html");
    dot_string.push_str(&format!("\t\"{}\" [style=filled, fillcolor=coral, URL=\"{}\", label=\"{}{}\"]; // function @ route, label = function name \n",
                                 function.route(),
                                 md_path,
                                 function.alias(),
                                 name));

    dot_string.push_str(&input_initializers(function, &function.route().to_string()));

    Ok(dot_string)
}

// Given a Function as used in the code generation - generate a "dot" format string to draw it
fn function_to_dot(function: &Function, functions: &[Function], _output_dir: &Path) -> String {
    let mut function_string = String::new();

    // modify path to point to the .html page that's built from .md to document the function
    let md_path = function.get_source_url().replace("toml", "html");

    function_string.push_str(&format!(
        "r{}[style=filled, fillcolor=coral, URL=\"{}\", label=\"{} (#{})\"];\n",
        function.get_id(),
        md_path,
        function.alias(),
        function.get_id()
    ));

    function_string.push_str(&input_initializers(
        function,
        &format!("r{}", function.get_id()),
    ));

    // Add edges for each of the outputs of this function to other ones
    for destination in function.get_output_connections() {
        let input_port = INPUT_PORTS[destination.io_number % INPUT_PORTS.len()];
        let destination_function = &functions[destination.function_id];
        let output_port = output_name_to_port(&destination.subroute);
        if let Some(inputs) = destination_function.get_inputs() {
            let input_name = inputs
                .get(destination.io_number)
                .unwrap()
                .name()
                .to_string();
            function_string.push_str(&format!(
                "r{}:{} -> r{}:{} [taillabel = \"{}\", headlabel = \"{}\"];\n",
                function.get_id(),
                output_port,
                destination.function_id,
                input_port,
                destination.subroute,
                input_name
            ));
        }
    }

    function_string
}

fn input_initializers(function: &Function, function_identifier: &str) -> String {
    let mut initializers = String::new();

    if let Some(inputs) = function.get_inputs() {
        for (input_number, input) in inputs.iter().enumerate() {
            if let Some(initializer) = input.get_initializer() {
                // Add an extra (hidden) graph entry for the initializer
                initializers.push_str(&format!(
                    "\"initializer{}_{}\"[style=invis];\n",
                    function_identifier, input_number
                ));
                let (value, is_constant) = match initializer {
                    Always(value) => (value.clone(), true),
                    Once(value) => (value.clone(), false),
                };

                let value_string = if let Value::String(value_str) = value {
                    format!("\\\"{}\\\"", value_str)
                } else {
                    format!("{}", value)
                };

                let line_style = if is_constant { "solid" } else { "dotted" };

                let input_port = input_name_to_port(input.name());
                // escape the quotes in the value when converted to string
                initializers.push_str(&format!("\"initializer{}_{}\" -> \"{}\":{} [style={}] [len=0.1] [color=blue] [label=\"{}\"];\n",
                                               function_identifier, input_number, function_identifier, input_port, line_style, value_string));
            }
        }
    }

    initializers
}

/*
    Rotate through the 3 top 'ports' on the sub-flow bubble to try and make inputs separate out
    visually - but this breaks down if we have more than 3 inputs
*/
fn add_input_set(input_set: &IOSet, to: &Route, connect_subflow: bool) -> String {
    let mut string = String::new();

    if let Some(inputs) = input_set {
        string.push_str("\n\t// Inputs\n\t{ rank=source\n");
        for input in inputs {
            // Avoid creating extra points to connect to for default input
            if input.route() != to {
                // Add an entry for each input using it's route
                string.push_str(&format!(
                    "\t\"{}\" [label=\"{}\", shape=house, style=filled, fillcolor=white];\n",
                    input.route(),
                    input.name()
                ));

                if connect_subflow {
                    // and connect the input to the sub-flow
                    string.push_str(&format!(
                        "\t\"{}\" -> \"{}\":n [style=invis, headtooltip=\"{}\"];\n",
                        input.route(),
                        to.to_string(),
                        input.name()
                    ));
                }
            }
        }
        string.push_str("\t}\n");
    }
    string
}

/*
    Add the outputs from a flow to add points to connect to
*/
fn add_output_set(output_set: &IOSet, from: &Route, connect_subflow: bool) -> String {
    let mut string = String::new();

    if let Some(outputs) = output_set {
        string.push_str("\n\t// Outputs\n\t{ rank=sink\n");
        for output in outputs {
            // Only add output if it's not got the same route as it's function i.e. it's not the default output
            if output.route() != from {
                // Add an entry for each output using it's route
                string.push_str(&format!("\t\"{}\" [label=\"{}\", shape=invhouse, style=filled, fillcolor=black, fontcolor=white];\n",
                                         output.route(), output.name()));

                if connect_subflow {
                    // and connect the output to the sub-flow
                    let output_port = output_name_to_port(output.name());
                    string.push_str(&format!(
                        "\t\"{}\":{} -> \"{}\"[style=invis, headtooltip=\"{}\"];\n",
                        from.to_string(),
                        output_port,
                        output.route(),
                        output.name()
                    ));
                }
            }
        }
        string.push_str("\t}\n");
    }
    string
}

fn output_compiled_function(
    route: &Route,
    tables: &GenerationTables,
    output: &mut String,
    output_dir: &Path,
) {
    for function in &tables.functions {
        if function.route() == route {
            output.push_str(&function_to_dot(function, &tables.functions, output_dir));
        }
    }
}

pub fn process_refs_to_dot(
    flow: &Flow,
    tables: &GenerationTables,
    output_dir: &Path,
) -> Result<String> {
    let mut output = String::new();

    // Do the same for all subprocesses referenced from this one
    if let Some(ref process_refs) = flow.process_refs {
        for process_ref in process_refs {
            let process = flow
                .subprocesses
                .get(process_ref.alias())
                .ok_or("Could not find process named in process_ref")?;
            match process {
                FlowProcess(ref subflow) => {
                    // create cluster sub graph
                    output.push_str(&format!(
                        "\nsubgraph cluster_{} {{\n",
                        str::replace(&subflow.alias.to_string(), "-", "_")
                    ));
                    output.push_str(&format!("label = \"{}\";", subflow.route()));

                    output.push_str(&process_refs_to_dot(subflow, tables, output_dir)?); // recurse

                    // close cluster
                    output.push_str("}\n");
                }
                FunctionProcess(ref function) => {
                    output_compiled_function(function.route(), tables, &mut output, output_dir);
                }
            }
        }
    }

    Ok(output)
}
