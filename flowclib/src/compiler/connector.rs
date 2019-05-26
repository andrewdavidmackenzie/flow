use model::route::Route;
use model::route::HasRoute;
use std::collections::HashMap;
use generator::generate::GenerationTables;
use model::connection::Connection;
use model::name::HasName;
use model::io::IOType;

/*
    Go through all connections, finding:
      - source process (process id and output route connection is from)
      - destination process (process id and input number the connection is to)

    Then add an output route to the source process's output routes vector
    (according to each function's output route in the original description plus each connection from
     that route, which could be to multiple destinations)
*/
pub fn prepare_function_connections(tables: &mut GenerationTables) -> Result<(), String> {
    debug!("Setting output routes on processes");
    for connection in &tables.collapsed_connections {
        if let Some((output_route, source_id)) = get_source(&tables.source_routes, &connection.from_io.route()) {
            if let Some(&(destination_process_id, destination_input_index)) = tables.destination_routes.get(connection.to_io.route()) {
                if let Some(source_function) = tables.functions.get_mut(source_id) {
                    debug!("Connection: from '{}' to '{}'", &connection.from_io.route(), &connection.to_io.route());
                    debug!("Output: Route = '{}', destination_process_id = {}, destination_input_index = {})",
                           output_route.to_string(), destination_process_id, destination_input_index);
                    source_function.add_output_route((output_route.to_string(), destination_process_id, destination_input_index));
                }

                // TODO when connection uses references to real IOs then we maybe able to remove this
                if connection.to_io.get_initializer().is_some() {
                    if let Some(destination_function) = tables.functions.get_mut(destination_process_id) {
                        if let Some(ref mut inputs) = destination_function.get_mut_inputs() {
                            let mut destination_input = inputs.get_mut(destination_input_index).unwrap();
                            if destination_input.get_initializer().is_none() {
                                destination_input.set_initial_value(connection.to_io.get_initializer());
                                debug!("Set initializer on destination function input at '{}' from connection",
                                       connection.to_io.route());
                            }
                        }
                    }
                }
            } else {
                return Err(format!("Connection destination process for route '{}' not found", connection.to_io.route()));
            }
        } else {
            return Err(format!("Connection source process for route '{}' not found", connection.from_io.route()));
        }
    }

    debug!("All output routes set on processes");

    Ok(())
}

/*
    find a function using the route to its output (removing the array index first to find outputs that are arrays)
    return a tuple of the sub-route to use (possibly with array index included), and the function index
*/
pub fn get_source(source_routes: &HashMap<Route, (Route, usize)>, from_route: &Route) -> Option<(Route, usize)> {
    let (source_without_index, array_index, is_array_output) = from_route.without_trailing_array_index();
    let source = source_routes.get(&*source_without_index.to_owned());

    if let Some(&(ref route, function_index)) = source {
        if is_array_output {
            if route.is_empty() {
                return Some((Route::from(&format!("/{}", array_index)), function_index));
            } else {
                return Some((Route::from(&format!("/{}/{}", route, array_index)), function_index));
            }
        } else {
            if route.is_empty() {
                return Some((route.clone(), function_index));
            } else {
                return Some((Route::from(&format!("/{}", route.to_string())), function_index));
            }
        }
    } else {
        return None;
    }
}

/*
    Construct two look-up tables that can be used to find the index of a function in the functions table,
    and the index of it's input - using the input route or it's output route
*/
pub fn create_routes_table(tables: &mut GenerationTables) {
    for mut function in &mut tables.functions {
        // Add any output routes it has to the source routes table
        if let Some(ref outputs) = function.get_outputs() {
            for output in outputs {
                tables.source_routes.insert(output.route().clone(), (Route::from(output.name()), function.get_id()));
            }
        }

        // Add any inputs it has to the destination routes table
        let mut input_index = 0;
        if let Some(ref inputs) = function.get_inputs() {
            for input in inputs {
                tables.destination_routes.insert(input.route().clone(), (function.get_id(), input_index));
                input_index += 1;
            }
        }
    }
}

/*
    Given a route we have a connection to, attempt to find the final destinations, potentially
    traversing multiple intermediate connections (recursively) until we find any that do not
    end at a flow. Return them as the final destinations.

    As a connection at a flow boundary can connect to multiple destinations, one
    original connection can branch to connect to multiple destinations.

        Chained connections allowed
               origin          intermediate    destination
                 +-----------------+--------------+
               function          flow io       function

        Connections (before combining them) only exist within a given flow, to Functions within it
        or to/from outputs/inputs of that flow, and to/from inputs/outputs of sub-flows referenced

        There are six connection types:
            - Function Input
            - Function Output
            - Sub-flow Input
            - Sub-flow Output
            - Flow Input
            - Flow Outout

        If we process from From->To, then there are only 3 source types

        Connection Source types:
            - Function Output
            - Sub-flow Output
            - Flow Input

        and 3 destination types:
            - Function Input
            - Sub-flow Input
            - Flow Output

        Giving nine possible combinations:

         Case Source Type               Destination Type        Description
         ------------------------------------------------------------------------------------------------
   Handled directly by collapse_connections
         1    Function Output           Function Input          Direct connection within a flow
         2    Function Output           Flow Input (subflow)
         3    Function Output           Flow Output             From a function and exits this flow

   Handled by find_function_destinations
         4    Flow Output (subflow)     Function Input          From a subflow into a function in this flow
         5    Flow Output (subflow)     Flow Input (subflow)    Connector betweem two sub-flows
         6    Flow Output (subflow)     Flow Output (parent)    From a sub-flow and exits this flow

         7    Flow Input (from parent)  Function Input          Enters flow from higher level into a Function
         8    Flow Input (from parent)  Flow Input (subflow)    Enters flow from higher level into a Sub-flow
         9    Flow Input (from parent)  Flow Output             A pass-thru connection within a flow
*/
fn find_function_destinations(from_io_route: &Route, from_level: usize, connections: &Vec<Connection>) -> Vec<Route> {
    let mut destinations = vec!();

    debug!("\tLooking for connections from '{}' on level={}", from_io_route, from_level);

    let mut found = false;
    for next_connection in connections {
        if next_connection.from_io.route() == from_io_route {
            let next_level = match *next_connection.from_io.io_type() {
                // Can't escape the context!
                IOType::FlowOutput if from_level > 0 => from_level - 1,
                IOType::FlowOutput if from_level == 0 => std::usize::MAX,
                IOType::FlowInput  => from_level + 1,
                _ => from_level
            };

            if next_connection.level == next_level {
                match *next_connection.to_io.io_type() {
                    IOType::FunctionIO => {
                        debug!("\t\tFound destination function input at '{}'", next_connection.to_io.route());
                        // Found a destination that is a function, add it to the list
                        destinations.push(next_connection.to_io.route().clone());
                        found = true;
                    }
                    IOType::FlowInput => {
                        debug!("\t\tFollowing connection into sub-flow via '{}'", from_io_route);
                        destinations.append(
                            &mut find_function_destinations(&next_connection.to_io.route(),
                                                            next_connection.level, connections));
                    }
                    IOType::FlowOutput => {
                        debug!("\t\tFollowing connection out of flow via '{}'", from_io_route);
                        destinations.append(
                            &mut find_function_destinations(&next_connection.to_io.route(),
                                                            next_connection.level, connections));
                    }
                }
            }
        }
    }

    if !found {
        debug!("\t\tEnd of connection chain reached without finding a destination function");
    }

    destinations
}

/*
    Take the original table of connections as gathered from the flow hierarchy, and for each one
    follow it through any intermediate connections (sub-flow boundaries) to arrive at the final
    destination. Then create a new direct connection from source to destination and add that
    to the table of "collapsed" connections which will be used to configure the outputs of the
    functions.
*/
pub fn collapse_connections(original_connections: &Vec<Connection>) -> Vec<Connection> {
    let mut collapsed_connections: Vec<Connection> = Vec::new();

    debug!("Working on {} flow hierarchy connections", original_connections.len());

    for connection in original_connections {
        // Try to collapse connections that start at a Function
        if *connection.from_io.io_type() == IOType::FunctionIO {
            debug!("Trying to create connection from function output at '{}' (level={})",
                   connection.from_io.route(), connection.level);
            if *connection.to_io.io_type() == IOType::FunctionIO {
                debug!("\tFound direct connection to function input at '{}'", connection.to_io.route());
                collapsed_connections.push(connection.clone());
            } else {
                // If the connection enters or leaves this flow, then follow it to function destinations
                for final_destination in find_function_destinations(&connection.to_io.route(),
                                                                    connection.level, original_connections) {
                    let mut collapsed_connection = connection.clone();
                    collapsed_connection.to_io.set_route(&final_destination, &IOType::FunctionIO);
                    collapsed_connection.to = final_destination;
                    debug!("\tIndirect connection {}", collapsed_connection);
                    collapsed_connections.push(collapsed_connection);
                }
            }
        } else {
            debug!("Skipping connection from flow at '{}'", connection.from_io.route());
        }
    }

// Print some stats in debug logs
    debug!("Connections between functions: {}", collapsed_connections.len());

    collapsed_connections
}

#[cfg(test)]
mod test {
    use model::connection::Connection;
    use model::io::{IO, IOType};
    use super::collapse_connections;
    use model::route::HasRoute;
    use model::name::Name;
    use model::route::Route;

    #[test]
    fn drop_useless_connections() {
        let mut unused = Connection {
            name: Some(Name::from("left")),
            from: Route::from("/f1/a"),
            to: Route::from("/f2/a"),
            from_io: IO::new("String", &Route::from("/f1/a")),
            to_io: IO::new("String", &Route::from("/f2/a")),
            level: 0,
        };
        unused.to_io.set_flow_io(IOType::FlowInput);

        let connections = vec!(unused);
        let collapsed = collapse_connections(&connections);
        assert_eq!(collapsed.len(), 0);
    }

    #[test]
    fn collapse_a_connection() {
        let mut left_side = Connection {
            name: Some(Name::from("left")),
            from: Route::from("/context/function1"),
            to: Route::from("/context/flow2/a"),
            from_io: IO::new("String", &Route::from("/context/function1")),
            to_io: IO::new("String", &Route::from("/context/flow2/a")),
            level: 0,
        };
        left_side.from_io.set_flow_io(IOType::FunctionIO);
        left_side.to_io.set_flow_io(IOType::FlowInput);

// This one goes to a flow but then nowhere, so should be dropped
        let mut extra_one = Connection {
            name: Some(Name::from("unused")),
            from: Route::from("/context/flow2/a"),
            to: Route::from("/context/flow2/f4/a"),
            from_io: IO::new("String", &Route::from("/context/flow2/a")),
            to_io: IO::new("String", &Route::from("/context/flow2/f4/a")),
            level: 1,
        };
        extra_one.from_io.set_flow_io(IOType::FlowInput);
        extra_one.to_io.set_flow_io(IOType::FlowInput); // /context/flow2/f4 doesn't exist

        let mut right_side = Connection {
            name: Some(Name::from("right")),
            from: Route::from("/context/flow2/a"),
            to: Route::from("/context/flow2/function3"),
            from_io: IO::new("String", &Route::from("/context/flow2/a")),
            to_io: IO::new("String", &Route::from("/context/flow2/function3")),
            level: 1,
        };
        right_side.from_io.set_flow_io(IOType::FlowInput);
        right_side.to_io.set_flow_io(IOType::FunctionIO);

        let connections = vec!(left_side, extra_one, right_side);

        let collapsed = collapse_connections(&connections);
        assert_eq!(collapsed.len(), 1);
        assert_eq!(*collapsed[0].from_io.route(), Route::from("/context/function1"));
        assert_eq!(*collapsed[0].to_io.route(), Route::from("/context/flow2/function3"));
    }

    /*
        This tests a connection into a sub-flow, that in the sub-flow branches with two
        connections to different elements in it.
        This should result in two end to end connections from the orginal source to the elements
        in the subflow
    */
    #[test]
    fn two_connections_from_flow_boundary() {
        let mut left_side = Connection {
            name: Some(Name::from("left")),
            from: Route::from("/context/f1"),
            to: Route::from("/context/f2/a"),
            from_io: IO::new("String", &Route::from("/context/f1")),
            to_io: IO::new("String", &Route::from("/context/f2/a")),
            level: 0,
        };
        left_side.from_io.set_flow_io(IOType::FunctionIO);
        left_side.to_io.set_flow_io(IOType::FlowInput);

        let mut right_side_one = Connection {
            name: Some(Name::from("right1")),
            from: Route::from("/context/f2/a"),
            to: Route::from("/context/f2/value1"),
            from_io: IO::new("String", &Route::from("/context/f2/a")),
            to_io: IO::new("String", &Route::from("/context/f2/value1")),
            level: 1,
        };
        right_side_one.from_io.set_flow_io(IOType::FlowInput);
        right_side_one.to_io.set_flow_io(IOType::FunctionIO);

        let mut right_side_two = Connection {
            name: Some(Name::from("right2")),
            from: Route::from("/context/f2/a"),
            to: Route::from("/context/f2/value2"),
            from_io: IO::new("String", &Route::from("/context/f2/a")),
            to_io: IO::new("String", &Route::from("/context/f2/value2")),
            level: 1,
        };
        right_side_two.from_io.set_flow_io(IOType::FlowInput);
        right_side_two.to_io.set_flow_io(IOType::FunctionIO);

        let connections = vec!(left_side, right_side_one, right_side_two);
        assert_eq!(3, connections.len());

        let collapsed = collapse_connections(&connections);
        assert_eq!(2, collapsed.len());
        assert_eq!(Route::from("/context/f1"), *collapsed[0].from_io.route());
        assert_eq!(Route::from("/context/f2/value1"), *collapsed[0].to_io.route());
        assert_eq!(Route::from("/context/f1"), *collapsed[1].from_io.route());
        assert_eq!(Route::from("/context/f2/value2"), *collapsed[1].to_io.route());
    }

    #[test]
    fn collapses_connection_into_subflow() {
        let mut first_level = Connection {
            name: Some(Name::from("value-to-f1:a at context level")),
            from: Route::from("/context/value"),
            to: Route::from("/context/flow1/a"),
            from_io: IO::new("String", &Route::from("/context/value")),
            to_io: IO::new("String", &Route::from("/context/flow1/a")),
            level: 0,
        };
        first_level.from_io.set_flow_io(IOType::FunctionIO);
        first_level.to_io.set_flow_io(IOType::FlowInput);

        let mut second_level = Connection {
            name: Some(Name::from("subflow_connection")),
            from: Route::from("/context/flow1/a"),
            to: Route::from("/context/flow1/flow2/a"),
            from_io: IO::new("String", &Route::from("/context/flow1/a")),
            to_io: IO::new("String", &Route::from("/context/flow1/flow2/a")),
            level: 1,
        };
        second_level.from_io.set_flow_io(IOType::FlowInput);
        second_level.to_io.set_flow_io(IOType::FlowInput);

        let mut third_level = Connection {
            name: Some(Name::from("subsubflow_connection")),
            from: Route::from("/context/flow1/flow2/a"),
            to: Route::from("/context/flow1/flow2/func/in"),
            from_io: IO::new("String", &Route::from("/context/flow1/flow2/a")),
            to_io: IO::new("String", &Route::from("/context/flow1/flow2/func/in")),
            level: 2,
        };
        third_level.from_io.set_flow_io(IOType::FlowInput);
        third_level.to_io.set_flow_io(IOType::FunctionIO);

        let connections = vec!(first_level,
                               second_level,
                               third_level);

        let collapsed = collapse_connections(&connections);
        assert_eq!(1, collapsed.len());
        assert_eq!(Route::from("/context/value"), *collapsed[0].from_io.route());
        assert_eq!(Route::from("/context/flow1/flow2/func/in"), *collapsed[0].to_io.route());
    }

    #[test]
    fn doesnt_collapse_a_non_connection() {
        let one = Connection {
            name: Some(Name::from("left")),
            from: Route::from("/f1/a"),
            to: Route::from("/f2/a"),
            from_io: IO::new("String", &Route::from("/f1/a")),
            to_io: IO::new("String", &Route::from("/f2/a")),
            level: 0,
        };

        let other = Connection {
            name: Some(Name::from("right")),
            from: Route::from("/f3/a"),
            to: Route::from("/f4/a"),
            from_io: IO::new("String", &Route::from("/f3/a")),
            to_io: IO::new("String", &Route::from("/f4/a")),
            level: 0,
        };

        let connections = vec!(one, other);
        let collapsed = collapse_connections(&connections);
        assert_eq!(collapsed.len(), 2);
    }
}
