use model::flow::Flow;
use generator::code_gen::CodeGenTables;
use std::io;
use std::io::prelude::*;
use model::runnable::Runnable;
use model::flow_reference::FlowReference;
use model::io::IOSet;
use model::connection::Route;
use model::connection;

static RUNNABLES_INPUT_PORTS: &[&str] = &["n", "ne", "nw"];

pub fn dump_flow_dot(flow: &Flow, level: usize, dot_file: &mut Write) -> io::Result<String> {
    let mut contents = String::new();
    // Inputs
    contents.push_str(&add_input_set(&flow.inputs, &flow.route, false));

    // Outputs
    contents.push_str(&add_output_set(&flow.outputs, &flow.route, false));

    // Values
    if let &Some(ref values) = &flow.values {
        for value in values {
            contents.push_str(&run_to_dot(value));
        }
    }

    // Function References
    if let &Some(ref function_refs) = &flow.function_refs {
        for function_ref in function_refs {
            contents.push_str(&run_to_dot(&function_ref.function as &Runnable));
        }
    }

    // Flow References
    if let &Some(ref flow_refs) = &flow.flow_refs {
        contents.push_str("\n\t\tsubgraph cluster_sub_flows {\n\n");
        for flow_ref in flow_refs {
            contents.push_str(&flow_reference_to_dot(&flow_ref));
        }
        contents.push_str("\t\t} // close cluster_sub_flows\n\n"); // subgraph cluster_sub_flows
    }

    // Connections inside this flows
    if let &Some(ref connections) = &flow.connections {
        contents.push_str("\n\t// Connections");
        for connection in connections {
            let (from_route, number, array_index) = connection::name_without_trailing_number(&connection.from_io.route);

            if array_index {
                contents.push_str(&format!("\n\t\"{}\" -> \"{}\" [label=\"{}\"];",
                                           from_route, connection.to_io.route, number));
            } else {
                contents.push_str(&format!("\n\t\"{}\" -> \"{}\";",
                                           from_route, connection.to_io.route));
            }
        }
    }

    dot_file.write_all(digraph_wrapper_start(flow, level).as_bytes())?;
    dot_file.write_all(contents.as_bytes())?;
    dot_file.write_all(&digraph_wrapper_end().as_bytes())?;

    Ok("Dot file written".to_string())
}

fn digraph_wrapper_start(flow: &Flow, level: usize) -> String {
    let mut wrapper = String::new();

    // Create a directed graph named after the flow
    wrapper.push_str(&format!("digraph {} {{\n", str::replace(&flow.alias, "-", "_")));
    wrapper.push_str(&format!("\tlabel=\"{}\";\n", flow.alias));
    wrapper.push_str("\tlabelloc=t;\n");
    wrapper.push_str("\tmargin=0.4;\n");
    wrapper.push_str("\tcompound=true;\n");
    wrapper.push_str("\tmodel=mds;\n");

    if level == 0 { // Context
        wrapper.push_str("\n\tsubgraph cluster_context {\n\t\tshape=square;\n");
    } else {
        wrapper.push_str("\n\tsubgraph cluster_flow {\n\t\tshape=regular;	\n");
    }

    wrapper.push_str("\t\tmargin=50;\n\t\tlabel=\"\";\n");

    wrapper
}

fn digraph_wrapper_end() -> String {
    "
    } // close top level cluster
} // close digraph\n".to_string()
}

fn run_to_dot(runnable: &Runnable) -> String {
    let mut dot_string = String::new();

    dot_string.push_str(&format!("\t\t\"{}\" [label=\"{}\\n({})\"]; // runnable @ route, label = runnable name \n",
                                 runnable.route(),
                                 runnable.alias(),
                                 runnable.name()));

    if let Some(iv) = runnable.get_initial_value() {
        // Add an extra graph entry for the initial value
        dot_string.push_str(&format!("\t\t\t\t\"{}_iv\"[style=invis] ; // initial value\n", runnable.route()));
        // with a connection to the runnable
        let iv_string = str::replace(&iv.to_string(), "\"", "'");
        dot_string.push_str(&format!("\t\t\t\t\"{}_iv\" -> \"{}\" [style=dotted] [color=blue] [label=\"{}\"]; // connect initial value to runnable\n",
                                     runnable.route(), runnable.route(), iv_string));
    }

    dot_string.push_str(&add_input_set(&runnable.get_inputs(), &runnable.route().to_string(), true));
    dot_string.push_str(&add_output_set(&runnable.get_outputs(), &runnable.route().to_string(), true));

    let mut box_visibility = "";
    if runnable.get_type() == "Value" {
        box_visibility = "\t\tstyle=invis;";
    }

    // Put inside a cluster of it's own
    format!("\n\t\t// Runnable of type = {}
    \tsubgraph cluster_runnable_{} {{
			margin=0;
    {}
    {}\t\t}} // close runnable {} \n",
            runnable.get_type(),
            str::replace(&runnable.alias(), "-", "_"),
            box_visibility,
            dot_string,
            runnable.alias())
}

/*
    Rotate through the 3 top 'ports' on the sub-flow bubble to try and make inputs separate out
    visually - but this breaks down if we have more than 3 inputs
*/
fn add_input_set(input_set: &IOSet, to: &Route, connect_subflow: bool) -> String {
    let mut string = String::new();

    if let &Some(ref inputs) = input_set {
        string.push_str("\n\t\t\t// Inputs\n");
        for input in inputs {
            // Avoid creating extra points to connect to for default input (e.g. on a value)
            if input.route != to.to_string() {
                // Add an entry for each input using it's route
                string.push_str(&format!("\t\t\t\"{}\" [label=\"{}\", style=filled, fixedsize=true, width=0.2, height=0.2, fillcolor=grey];\n",
                                         input.route, input.name));

                if connect_subflow {
                    // and connect the input to the sub-flow
                    string.push_str(&format!("\t\t\t\"{}\" -> \"{}\":n [len=0, weight=1000, style=invis, headtooltip=\"{}\"];\n",
                                             input.route, to, input.name));
                }
            }
        }
    }
    string
}

/*
    Add the outputs from a flow to add points to connect to
*/
fn add_output_set(output_set: &IOSet, from: &Route, connect_subflow: bool) -> String {
    let mut string = String::new();

    if let &Some(ref outputs) = output_set {
        string.push_str("\n\t\t\t// Outputs\n");
        for output in outputs {
            // Add an entry for each output using it's route
            string.push_str(&format!("\t\t\t\"{}\" [label=\"{}\", style=filled, fixedsize=true, width=0.2, height=0.2, fillcolor=grey];\n",
                                     output.route, output.name));

            if connect_subflow {
                // and connect the output to the sub-flow
                string.push_str(&format!("\t\t\t\"{}\":s -> \"{}\"[len=0, style=invis, weight=1000, headtooltip=\"{}\"];\n",
                                         from, output.route, output.name));
            }
        }
    }
    string
}

fn flow_reference_to_dot(flow_ref: &FlowReference) -> String {
    let mut dot_string = String::new();

    dot_string.push_str(&format!("\t\t\t\"{}\" [label=\"{}\", fixedsize=true, width=1, height=1, URL=\"{}.dot\"];\n",
                                 flow_ref.flow.route,
                                 flow_ref.alias,
                                 flow_ref.flow.alias));

    dot_string.push_str(&format!("\t\t\t{}", &add_input_set(&flow_ref.flow.inputs, &flow_ref.flow.route, true)));
    dot_string.push_str(&format!("\t\t\t{}", &add_output_set(&flow_ref.flow.outputs, &flow_ref.flow.route, true)));

    // Put inside a cluster of it's own
    format!("\t\t\t// Sub-flow\n\t\t\tsubgraph cluster_sub_flow_{} {{
                style=invis;
    {}\t\t\t}} // close sub-flow {}\n\n", str::replace(&flow_ref.flow.alias, "-", "_"), dot_string, flow_ref.flow.alias)
}

pub fn runnables_to_dot(flow_alias: &str, tables: &CodeGenTables, dot_file: &mut Write) -> io::Result<String> {
    // Create a directed graph named after the flow
    dot_file.write_all(format!("digraph {} {{\n", str::replace(flow_alias, "-", "_")).as_bytes())?;

    let mut runnables = String::new();
    for runnable in &tables.runnables {
        runnables.push_str(&runnable_to_dot(&runnable, &tables.runnables));
    }
    dot_file.write_all(runnables.as_bytes())?;

    dot_file.write_all("}".as_bytes())?;

    Ok("Dot file written".to_string())
}

// Given a Runnable as used in the code generation - generate a "dot" format string to draw it
fn runnable_to_dot(runnable: &Box<Runnable>, runnables: &Vec<Box<Runnable>>) -> String {
    let mut runnable_string = String::new();

    let shape = if runnable.get_type() == "Value" {
        "shape=box,"
    } else {
        ""
    };

    let fill = if runnable.is_static_value() {
        "style=filled, fillcolor=\"#999999\","
    } else {
        ""
    };

    runnable_string.push_str(&format!("r{}[{} {} label=\"{} (#{})\"];\n",
                                      runnable.get_id(),
                                      shape, fill,
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

    // Add edges for each of th eoutputs of this runnable to other ones
    for &(ref output_route, destination_index, destination_input_index) in runnable.get_output_routes() {
        let input_port = RUNNABLES_INPUT_PORTS[destination_input_index % RUNNABLES_INPUT_PORTS.len()];
        let destination_runnable = &runnables[destination_index];
        let input_name = &destination_runnable.get_inputs().unwrap()[destination_input_index].name;
        runnable_string.push_str(&format!("r{}:s -> r{}:{} [taillabel = \"{}\", headlabel = \"{}\"];\n",
                                          runnable.get_id(), destination_index, input_port,
                                          output_route, input_name));
    }

    runnable_string
}