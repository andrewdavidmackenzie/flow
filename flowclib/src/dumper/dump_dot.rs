use model::flow::Flow;
use generator::generate::GenerationTables;
use std::io;
use std::io::prelude::*;
use std::path::PathBuf;
use model::runnable::Runnable;
use model::process_reference::ProcessReference;
use model::io::IOSet;
use model::route::Route;
use model::route::Router;
use model::route::HasRoute;
use model::route::FindRoute;
use model::connection::Connection;
use model::name::HasName;
use model::process::Process::FlowProcess;
use model::process::Process::FunctionProcess;
use ::dumper::helper;

static INPUT_PORTS: &[&str] = &["n", "ne", "nw"];
//static OUTPUT_PORTS: &[&str] = &["s", "se", "sw"];

pub fn dump_flow_dot(flow: &Flow, dot_file: &mut Write) -> io::Result<String> {
    dot_file.write_all(digraph_wrapper_start(flow).as_bytes())?;

    let mut contents = String::new();
    // Inputs
    contents.push_str(&add_input_set(&flow.inputs, flow.route(), false));

    // Outputs
    contents.push_str(&add_output_set(&flow.outputs, flow.route(), false));

    // Values
    if let Some(values) = &flow.values {
        for value in values {
            contents.push_str(&run_to_dot(value));
        }
    }

    // Process References
    if let Some(process_refs) = &flow.process_refs {
        for flow_ref in process_refs {
            match flow_ref.process {
                FunctionProcess(ref function) => {
                    contents.push_str(&run_to_dot(function as &Runnable));
                }
                FlowProcess(ref _flow) => {
                    contents.push_str("\n\t// Sub-Flows\n");
                    contents.push_str(&process_reference_to_dot(flow_ref));
                }
            }
        }
    }

    // Connections
    if let Some(connections) = &flow.connections {
        contents.push_str("\n\t// Connections");
        for connection in connections {
            contents.push_str(&connection_to_dot(&connection, &flow.inputs, &flow.outputs));
        }
    }

    dot_file.write_all(contents.as_bytes())?;

    dot_file.write_all(&digraph_wrapper_end().as_bytes())?;

    Ok("Dot file written".to_string())
}

fn connection_to_dot(connection: &Connection, input_set: &IOSet, output_set: &IOSet) -> String {
    let (from_route, number, array_index) = Router::without_trailing_array_index(&connection.from_io.route());

    let (from_node, from_label) = node_from_io_route(&from_route.to_string(), &connection.from_io.name(), input_set);
    let (to_node, to_label) = node_from_io_route(&connection.to_io.route(), &connection.to_io.name(), output_set);
    if array_index {
        format!("\n\t\"{}\" -> \"{}\" [labeldistance=\"3\", taillabel=\"{}[{}]\", headlabel=\"{}\"];",
                from_node, to_node, from_label, number, to_label)
    } else {
        format!("\n\t\"{}\" -> \"{}\" [labeldistance=\"3\", taillabel=\"{}\", headlabel=\"{}\"];",
                from_node, to_node, from_label, to_label)
    }
}

/*
    Return the route to a node (value, function, flow) for a given route, by:

    If the input or output name IS the default one ("" empty string), then just return the route.

    If the input or output IS NOT the default one ("" empty string) then remove the IO name from the
    route and return that.
*/
fn node_from_io_route(route: &Route, name: &str, io_set: &IOSet) -> (String, String) {
    let mut label = "".to_string();
    if !io_set.find(route) {
        label = name.to_string();
    }

    if name.is_empty() || io_set.find(route) {
        return (route.clone(), label);
    } else {
        let length_without_io_name = route.len() - name.len() - 1; // 1 for '/'
        return (route.clone()[..length_without_io_name].to_string(), label);
    }
}

fn digraph_wrapper_start(flow: &Flow) -> String {
    let mut wrapper = String::new();

    // Create a directed graph named after the flow
    wrapper.push_str(&format!("digraph {} {{\n", str::replace(&flow.alias, "-", "_")));
    wrapper.push_str(&format!("\tlabel=\"{}\";\n", flow.alias));
    wrapper.push_str("\tlabelloc=t;\n");
    wrapper.push_str("\tmargin=0.4;\n");
    wrapper.push_str("\tcompound=true;\n");
    wrapper.push_str("\tmodel=mds;\n");
    wrapper.push_str("\tnodesep=1.5;\n");

    wrapper
}

fn digraph_wrapper_end() -> String {
    "
} // close digraph\n".to_string()
}

fn run_to_dot(runnable: &Runnable) -> String {
    let mut dot_string = String::new();

    let name = if runnable.name() == runnable.alias() {
        "".to_string()
    } else {
        format!("\\n({})", runnable.name()).to_string()
    };

    let mut initial_value = if let Some(iv) = runnable.get_initial_value() {
        format!("\\ninit={}", iv).to_string()
    } else {
        "".to_string()
    };

    // Escape any quotes in intial value - as it might be a string value
    initial_value = str::replace(&initial_value, "\"", "\\\"");


    dot_string.push_str(&format!("\t\"{}\" [{} label=\"{}{}{}\"]; // runnable @ route, label = runnable name \n",
                                 runnable.route(),
                                 runnable_style(runnable),
                                 runnable.alias(), name, initial_value));

    dot_string
}


// Given a Runnable as used in the code generation - generate a "dot" format string to draw it
fn runnable_to_dot(runnable: &Box<Runnable>, runnables: &Vec<Box<Runnable>>) -> String {
    let mut runnable_string = String::new();

    let style = runnable_style(&**runnable);

    runnable_string.push_str(&format!("r{}[{} label=\"{} (#{})\"];\n",
                                      runnable.get_id(),
                                      style,
                                      runnable.alias(),
                                      runnable.get_id()));

    if let Some(iv) = runnable.get_initial_value() {
        // Add an extra graph entry for the initial value
        runnable_string.push_str(&format!("iv{}[style=invis];\n", runnable.get_id()));
        // with a connection to the runnable
        if iv.is_string() {
            // escape the quotes in the value when converted to string
            runnable_string.push_str(&format!("iv{} -> r{} [style=dotted] [color=blue] [label=\"'{}'\"];\n",
                                              runnable.get_id(), runnable.get_id(), iv.as_str().unwrap()));
        } else {
            runnable_string.push_str(&format!("iv{} -> r{} [style=dotted] [color=blue] [label=\"{}\"];\n",
                                              runnable.get_id(), runnable.get_id(), iv));
        }
    }

    // Add edges for each of the outputs of this runnable to other ones
    for &(ref output_route, destination_index, destination_input_index) in runnable.get_output_routes() {
        let input_port = INPUT_PORTS[destination_input_index % INPUT_PORTS.len()];
        let destination_runnable = &runnables[destination_index];
        let input_name = destination_runnable.get_inputs().unwrap()[destination_input_index].name().to_string();
        runnable_string.push_str(&format!("r{}:s -> r{}:{} [taillabel = \"{}\", headlabel = \"{}\"];\n",
                                          runnable.get_id(), destination_index, input_port,
                                          output_route, input_name));
    }

    runnable_string
}

fn runnable_style(runnable: &Runnable) -> &'static str {
    if runnable.get_type() == "Value" {
        if runnable.is_static_value() {
            return "shape=cylinder, style=filled, fillcolor=gray40,"; // static value
        } else {
            return "shape=cylinder, style=filled, fillcolor=dodgerblue,"; // normal value
        }
    } else {
        return "style=filled, fillcolor=coral,";
    }
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
            // Avoid creating extra points to connect to for default input (e.g. on a value)
            if input.route() != to {
                // Add an entry for each input using it's route
                string.push_str(&format!("\t\"{}\" [label=\"{}\", shape=house, style=filled, fillcolor=white];\n",
                                         input.route(), input.name()));

                if connect_subflow {
                    // and connect the input to the sub-flow
                    string.push_str(&format!("\t\"{}\" -> \"{}\":n [style=invis, headtooltip=\"{}\"];\n",
                                             input.route(), to, input.name()));
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
            // Only add output if it's not got the same route as it's runnable i.e. it's not the default output
            if output.route() != from {
                // Add an entry for each output using it's route
                string.push_str(&format!("\t\"{}\" [label=\"{}\", rank=sink, shape=invhouse, style=filled, fillcolor=black, fontcolor=white];\n",
                                         output.route(), output.name()));

                if connect_subflow {
                    // and connect the output to the sub-flow
                    string.push_str(&format!("\t\"{}\":s -> \"{}\"[style=invis, headtooltip=\"{}\"];\n",
                                             from, output.route(), output.name()));
                }
            }
        }
        string.push_str("\t}\n");
    }
    string
}

fn process_reference_to_dot(process_ref: &ProcessReference) -> String {
    let mut dot_string = String::new();

    match process_ref.process {
        FlowProcess(ref flow) => {
            dot_string.push_str(&format!("\t\"{}\" [label=\"{}\", style=filled, fillcolor=aquamarine, width=2, height=2, URL=\"{}.dot\"];\n",
                                         flow.route(), process_ref.alias, process_ref.source_url));
        },
        FunctionProcess(ref function) => {
            dot_string.push_str(&format!("\t\"{}\" [label=\"{}\", style=filled, fillcolor=aquamarine, width=2, height=2, URL=\"{}.dot\"];\n",
                                         function.route(), process_ref.alias, process_ref.source_url));
        }
    }
    dot_string
}

pub fn runnables_to_dot(flow_alias: &str, tables: &GenerationTables, output_dir: &PathBuf) -> io::Result<String> {
    let mut dot_file = helper::create_output_file(&output_dir, "runnables", "dot")?;
    info!("Generating Runnables dot file {}, Use \"dotty\" to view it", output_dir.display());

    // Create a directed graph named after the flow
    dot_file.write_all(format!("digraph {} {{\nnodesep=1.5\n", str::replace(flow_alias, "-", "_")).as_bytes())?;

    let mut runnables = String::new();
    for runnable in &tables.runnables {
        runnables.push_str(&runnable_to_dot(runnable, &tables.runnables));
    }
    dot_file.write_all(runnables.as_bytes())?;

    dot_file.write_all("}".as_bytes())?;

    Ok("Dot file written".to_string())
}