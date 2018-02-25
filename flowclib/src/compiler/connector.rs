use model::value::Value;
use model::function::Function;
use model::connection::Route;
use std::collections::HashMap;
use compiler::compile::CompilerTables;

/*
    First build a table of routes to (runnable_index, input_index) for all inputs of runnables, to
    enable finding the destination of a connection as (runnable_index, input_index).

    Then iterate through the values and function setting each one's id and the output routes array setup
    (according to each ruannable's output route in the original description plus each connection from it)
    to point to the runnable (by index) and the runnable's input (by index) in the table
*/
pub fn connect(tables: &mut CompilerTables) {
    let inputs_routes= inputs_table(&tables.values, &tables.functions);
    let mut runnable_index = 0;

    for value in &mut tables.values {
        debug!("Looking for connection from value @ '{}'", &value.route);
        let mut output_connections = Vec::<(usize, usize)>::new();
        // Find the list of connections from the output of this runnable - there can be multiple
        for connection in &tables.connections {
            if value.route == connection.from_route {
                debug!("Connection found: to '{}'", &connection.to_route);
                // Get the index of runnable and input index of the destination of the connection
                output_connections.push(inputs_routes.get(&connection.to_route).unwrap().clone());
            }
        }
        value.id = runnable_index;
        value.output_routes = output_connections;
        runnable_index += 1;
    }

    for function in &mut tables.functions {
        let mut output_connections = Vec::<(usize, usize)>::new();
        // if it has any outputs at all
        if let Some(ref outputs) = function.outputs {
            debug!("Looking for connection from function @ '{}'", &function.route);
            // Find the list of connections from the output of this runnable - there can be multiple
            for connection in &tables.connections {
                if outputs[0].route == connection.from_route {
                    debug!("Connection found: to '{}'", &connection.to_route);
                    // Get the index of runnable and input index of the destination of the connection
                    output_connections.push(*inputs_routes.get(&connection.to_route).unwrap());
                }
            }
        }
        function.id = runnable_index;
        function.output_routes = output_connections;
        runnable_index += 1;
    }
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

