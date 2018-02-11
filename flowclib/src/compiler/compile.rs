use model::flow::Flow;
use model::value::Value;
use model::function::Function;
use model::connection::Connection;
use model::connection::Route;
use std::fmt;
use std::collections::HashMap;
use flowrlib::runnable::Runnable;
use flowrlib::value::Value as RunnableValue;
use flowrlib::function::Function as RunnableFunction;
use flowrlib::implementation::Implementation;
use std::fmt::Debug;

pub struct ImplementationStub {
    name: String,
}

impl Implementation for ImplementationStub {
    fn run(&self, _inputs: Vec<Option<String>>) -> Option<String> {
        unimplemented!()
    }

    fn number_of_inputs(&self) -> usize {
        1
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl Debug for ImplementationStub {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "defined in file: '{}'", file!())
    }
}

/// Take a hierarchical flow definition in memory and compile it, generating code that implements
/// the flow, including links to the flowrlib runtime library and library functions used in the
/// flowstdlib standard library. It takes an optional bool dump option to dump to standard output
/// some of the intermediate values and operations during the compilation process.
pub fn compile(flow: &mut Flow) ->
    (Vec<Connection>, Vec<Value>, Vec<Function>, Vec<Box<Runnable>>) {
    let mut connection_table: Vec<Connection> = Vec::new();
    let mut value_table: Vec<Value> = Vec::new();
    let mut function_table: Vec<Function> = Vec::new();
    add_entries(&mut connection_table, &mut value_table, &mut function_table, flow);

    connection_table = collapse_connections(&connection_table);

    prune_tables(&mut connection_table, &mut value_table, &mut function_table);

    let runnables = create_runnables_table(&value_table, &function_table, &connection_table);

    (connection_table, value_table, function_table, runnables)
}

/*
    Construct a look-up table that we can use to find the index of a runnable
    in the runnables table, and the index of it's input - for every route out of another runnable
*/
fn inputs_table(value_table: &Vec<Value>, function_table: &Vec<Function>) -> HashMap<Route, (usize, usize)> {
    let mut input_route_table = HashMap::<Route, (usize, usize)>::new();
    let mut runnable_index = 0;

    for value in value_table {
        // Value has only one input and it's route is that of the value itself
        input_route_table.insert(value.route.clone(), (runnable_index, 0));
        runnable_index += 1;
    }

    for function in function_table {
        let mut input_index = 0;
        // A function can have a number of inputs, each with different routes
        if let Some(ref inputs) = function.inputs {
            for input in inputs {
                input_route_table.insert(input.route.clone(), (runnable_index, input_index));
                input_index += 1;
            }
        }
        runnable_index += 1;
    }

    debug!("Input routes: {:?}", input_route_table);
    input_route_table
}

// TODO see if some of this can be bi-product of earlier stages?
/*
    First build a table of routes to (runnable_index, input_index) for all inputs of runnables, to
    enable finding the destination of a connection as (runnable_index, input_index).

    Then iterate through the runnables adding them to a list, with the output routes array setup
    (according to each ruannable's output route in the original description plus each connection from it)
    to point to the runnable (by index) and the runnable's input (by index) in the table
*/
fn create_runnables_table(value_table: &Vec<Value>,
                          function_table: &Vec<Function>,
                          connection_table: &Vec<Connection>) -> Vec<Box<Runnable>> {
    let inputs_routes = inputs_table(&value_table, &function_table);

    let mut runnables = Vec::<Box<Runnable>>::new();
    let mut runnable_index = 0;

    for value in value_table {
        debug!("Looking for connection from value @ '{}'", &value.route);
        let mut output_connections = Vec::<(usize, usize)>::new();
        // Find the list of connections from the output of this runnable - there can be multiple
        for connection in connection_table {
            if value.route == connection.from_route {
                debug!("Connection found: to '{}'", &connection.to_route);
                // Get the index of runnable and input index of the destination of the connection
                output_connections.push(inputs_routes.get(&connection.to_route).unwrap().clone());
            }
        }
        let runnable_value = Box::new(RunnableValue::new(runnable_index,
                                                         value.value.clone(),
                                                         output_connections));
        runnable_index += 1;
        runnables.push(runnable_value);
    }

    for function in function_table {
        let mut output_connections = Vec::<(usize, usize)>::new();
        // if it has any outputs at all
        if let Some(ref outputs) = function.outputs {
            debug!("Looking for connection from function @ '{}'", &function.route);
            // Find the list of connections from the output of this runnable - there can be multiple
            for connection in connection_table {
                if outputs[0].route == connection.from_route {
                    debug!("Connection found: to '{}'", &connection.to_route);
                    // Get the index of runnable and input index of the destination of the connection
                    output_connections.push(*inputs_routes.get(&connection.to_route).unwrap());
                }
            }
        }
        let implementation = Box::new(ImplementationStub { name: function.name.clone() });
        let runnable_function = Box::new(RunnableFunction::new(runnable_index,
                                                               implementation, output_connections));
        runnable_index += 1;

        runnables.push(runnable_function);
    }

    runnables
}

fn collapse_connections(complete_table: &Vec<Connection>) -> Vec<Connection> {
    let mut collapsed_table: Vec<Connection> = Vec::new();

    for left in complete_table {
        if left.ends_at_flow {
            for ref right in complete_table {
                if left.to_route == right.from_route {
                    // They are connected - modify first to go to destination of second
                    let mut joined_connection = left.clone();
                    joined_connection.to_route = format!("{}", right.to_route);
                    joined_connection.ends_at_flow = right.ends_at_flow;
                    collapsed_table.push(joined_connection);
                    //                      connection_table.drop(right)
                }
            }
        } else {
            collapsed_table.push(left.clone());
        }
    }

    // Now don't include the ones starting or ending on flows.
    let mut final_table: Vec<Connection> = Vec::new();

    for connection in collapsed_table {
        if !connection.starts_at_flow && !connection.ends_at_flow {
            final_table.push(connection.clone());
        }
    }

    final_table
}

#[test]
fn collapses_a_connection() {
    let left_side = Connection {
        name: Some("left".to_string()),
        from: "point a".to_string(),
        from_route: "/f1/a".to_string(),
        from_type: "String".to_string(),
        starts_at_flow: false,
        to: "point b".to_string(),
        to_route: "/f2/a".to_string(),
        to_type: "String".to_string(),
        ends_at_flow: true
    };

    let right_side = Connection {
        name: Some("right".to_string()),
        from: "point b".to_string(),
        from_route: "/f2/a".to_string(),
        from_type: "String".to_string(),
        starts_at_flow: true,
        to: "point c".to_string(),
        to_route: "/f3/a".to_string(),
        to_type: "String".to_string(),
        ends_at_flow: false
    };

    let connections = &vec!(left_side, right_side);

    let collapsed = collapse_connections(connections);
    assert_eq!(collapsed.len(), 1);
    assert_eq!(collapsed[0].from_route, "/f1/a".to_string());
    assert_eq!(collapsed[0].to_route, "/f3/a".to_string());
}

#[test]
fn doesnt_collapse_a_non_connection() {
    let one = Connection {
        name: Some("left".to_string()),
        from: "point a".to_string(),
        from_route: "/f1/a".to_string(),
        from_type: "String".to_string(),
        starts_at_flow: false,
        to: "point b".to_string(),
        to_route: "/f2/a".to_string(),
        to_type: "String".to_string(),
        ends_at_flow: false
    };

    let other = Connection {
        name: Some("right".to_string()),
        from: "point b".to_string(),
        from_route: "/f3/a".to_string(),
        from_type: "String".to_string(),
        starts_at_flow: false,
        to: "point c".to_string(),
        to_route: "/f4/a".to_string(),
        to_type: "String".to_string(),
        ends_at_flow: false
    };

    let connections = &vec!(one, other);
    let collapsed = collapse_connections(connections);
    assert_eq!(collapsed.len(), 2);
}

// TODO write tests for all this before any modification
fn add_entries(connection_table: &mut Vec<Connection>,
               value_table: &mut Vec<Value>,
               function_table: &mut Vec<Function>,
               flow: &mut Flow) {
    // Add Connections from this flow to the table
    if let Some(ref mut connections) = flow.connections {
        connection_table.append(connections);
    }

    // Add Values from this flow to the table
    if let Some(ref mut values) = flow.values {
        value_table.append(values);
    }

    // Add Functions referenced from this flow to the table
    if let Some(ref mut function_refs) = flow.function_refs {
        for function_ref in function_refs {
            function_table.push(function_ref.function.clone());
        }
    }

    // Do the same for all subflows referenced from this one
    if let Some(ref mut flow_refs) = flow.flow_refs {
        for flow_ref in flow_refs {
            add_entries(connection_table, value_table, function_table, &mut flow_ref.flow);
        }
    }
}

/*
    Drop the following combinations, with warnings:
    - values that don't have connections from them.
    - values that have only outputs and are not initialized.
    - functions that don't have connections from at least one output.
    - functions that don't have connections to all their inputs.
*/
// TODO implement this
fn prune_tables(_connection_table: &mut Vec<Connection>,
                _value_table: &mut Vec<Value>,
                _function_table: &mut Vec<Function>) {}