use model::route::Route;
use model::route::Router;
use model::route::HasRoute;
use std::collections::HashMap;
use generator::generate::GenerationTables;
use model::connection::Connection;
use model::name::HasName;
use model::io::IOType;

#[derive(PartialEq, Debug)]
enum Direction {
    Into,
    Outof,
}

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
    let (source_without_index, array_index, is_array_output) = Router::without_trailing_array_index(from_route);
    let source = source_routes.get(&source_without_index.to_string());

    if let Some(&(ref route, function_index)) = source {
        if is_array_output {
            if route.is_empty() {
                return Some((format!("/{}", array_index), function_index));
            } else {
                return Some((format!("/{}/{}", route, array_index), function_index));
            }
        } else {
            if route.is_empty() {
                return Some((route.to_string(), function_index));
            } else {
                return Some((format!("/{}", route.to_string()), function_index));
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
                tables.source_routes.insert(output.route().clone(), (output.name().clone(), function.get_id()));
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
*/
fn find_function_destinations(from_route: &Route, connections: &Vec<Connection>, direction: &Direction) -> Vec<Route> {
    let mut destinations = vec!();

    debug!("\tFollowing connection {:?} flow via '{}'", direction, from_route);

    for connection in connections {
        if connection.from_io.route() == from_route {
            match *connection.to_io.io_type() {
                IOType::FunctionIO => {
                    debug!("\t\tFound destination function input at '{}'", connection.to_io.route());
                    // Found a destination that is a function, add it to the list
                    destinations.push(connection.to_io.route().clone());
                }
                IOType::FlowInput if *direction == Direction::Into => {
                    // Keep following connections into sub flows until you we find a function
                    destinations.append(
                        &mut find_function_destinations(&connection.to_io.route(),
                                                        connections,
                                                        direction));
                }
                IOType::FlowOutput if *direction == Direction::Outof => {
                    // Keep following connections out to parent flows until you we find a function
                    destinations.append(
                        &mut find_function_destinations(&connection.to_io.route(),
                                                        connections,
                                                        direction));
                }
                _ => {}
            }
        }
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

    for left in original_connections {
        if *left.from_io.io_type() == IOType::FunctionIO {
            debug!("Trying to create connection from function ouput at '{}'", left.from_io.route());
            if *left.to_io.io_type() == IOType::FunctionIO {
                debug!("\tFound direct connection to function input at '{}'", left.to_io.route());
                collapsed_connections.push(left.clone());
            } else {
                let direction = if *left.to_io.io_type() == IOType::FlowInput {
                    Direction::Into
                } else {
                    Direction::Outof
                };

                // If the connection goes into a flow, then follow it to function destinations
                for final_destination in find_function_destinations(&left.to_io.route(),
                                                                    original_connections,
                                                                    &direction) {
                    let mut collapsed_connection = left.clone();
                    collapsed_connection.to_io.set_route(&final_destination, &IOType::FunctionIO);
                    collapsed_connection.to = final_destination;
                    debug!("\tIndirect connection {}", collapsed_connection);
                    collapsed_connections.push(collapsed_connection);
                }
            }
        } else {
            debug!("Skipping connection from flow at '{}'", left.from_io.route());
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

    #[test]
    fn drop_useless_connections() {
        let mut unused = Connection {
            name: Some("left".to_string()),
            from: "/f1/a".to_string(),
            to: "/f2/a".to_string(),

            from_io: IO::new(&"String".to_string(), &"/f1/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f2/a".to_string()),
        };
        unused.to_io.set_flow_io(IOType::FlowInput);

        let connections = vec!(unused);
        let collapsed = collapse_connections(&connections);
        assert_eq!(collapsed.len(), 0);
    }

    #[test]
    fn collapse_a_connection() {
        let mut left_side = Connection {
            name: Some("left".to_string()),
            from: "/f1/a".to_string(),
            to: "/f2/a".to_string(),
            from_io: IO::new(&"String".to_string(), &"/f1/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f2/a".to_string()),
        };
        left_side.to_io.set_name("point b".to_string());
        left_side.to_io.set_flow_io(IOType::FlowInput);

        // This one goes to a flow but then nowhere, so should be dropped
        let mut extra_one = Connection {
            name: Some("unused".to_string()),
            from: "/f2/a".to_string(),
            to: "/f4/a".to_string(),
            from_io: IO::new(&"String".to_string(), &"/f2/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f4/a".to_string()),
        };
        extra_one.from_io.set_name("point b".to_string());
        extra_one.from_io.set_flow_io(IOType::FlowOutput);
        extra_one.to_io.set_name("pointless".to_string());
        extra_one.to_io.set_flow_io(IOType::FlowInput);


        let mut right_side = Connection {
            name: Some("right".to_string()),
            from: "/f2/a".to_string(),
            to: "/f3/a".to_string(),
            from_io: IO::new(&"String".to_string(), &"/f2/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f3/a".to_string()),
        };
        right_side.from_io.set_flow_io(IOType::FlowOutput);

        let connections = vec!(left_side,
                               extra_one, right_side);

        let collapsed = collapse_connections(&connections);
        assert_eq!(collapsed.len(), 1);
        assert_eq!(collapsed[0].from_io.route(), "/f1/a");
        assert_eq!(collapsed[0].to_io.route(), "/f3/a");
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
            name: Some("left".to_string()),
            from: "/f1/a".to_string(),
            to: "/f2/a".to_string(),
            from_io: IO::new(&"String".to_string(), &"/f1/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f2/a".to_string()),
        };
        left_side.to_io.set_flow_io(IOType::FlowInput);

        let mut right_side_one = Connection {
            name: Some("right1".to_string()),
            from: "/f2/a".to_string(),
            to: "/f2/value1".to_string(),
            from_io: IO::new(&"String".to_string(), &"/f2/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f2/value1".to_string()),
        };
        right_side_one.from_io.set_flow_io(IOType::FlowOutput);

        let mut right_side_two = Connection {
            name: Some("right2".to_string()),
            from: "/f2/a".to_string(),
            to: "/f2/value2".to_string(),
            from_io: IO::new(&"String".to_string(), &"/f2/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f2/value2".to_string()),
        };
        right_side_two.from_io.set_flow_io(IOType::FlowOutput);

        let connections = vec!(left_side,
                               right_side_one,
                               right_side_two);

        assert_eq!(connections.len(), 3);

        let collapsed = collapse_connections(&connections);
        assert_eq!(collapsed.len(), 2);
        assert_eq!(collapsed[0].from_io.route(), "/f1/a");
        assert_eq!(collapsed[0].to_io.route(), "/f2/value1");
        assert_eq!(collapsed[1].from_io.route(), "/f1/a");
        assert_eq!(collapsed[1].to_io.route(), "/f2/value2");
    }

    #[test]
    fn collapses_connection_traversing_a_flow() {
        let mut first_level = Connection {
            name: Some("context".to_string()),
            from: "/value".to_string(),
            to: "/f1/a".to_string(),
            from_io: IO::new(&"String".to_string(), &"/value".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f1/a".to_string()),
        };
        first_level.to_io.set_flow_io(IOType::FlowInput);

        let mut second_level = Connection {
            name: Some("subflow_connection".to_string()),
            from: "/f1/a".to_string(),
            to: "/f2/a".to_string(),
            from_io: IO::new(&"String".to_string(), &"/f1/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f2/a".to_string()),
        };
        second_level.from_io.set_flow_io(IOType::FlowOutput);
        second_level.to_io.set_flow_io(IOType::FlowInput);

        let mut third_level = Connection {
            name: Some("subsubflow_connection".to_string()),
            from: "/f2/a".to_string(),
            to: "/f2/func/in".to_string(),
            from_io: IO::new(&"String".to_string(), &"/f2/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f2/func/in".to_string()),
        };
        third_level.from_io.set_flow_io(IOType::FlowOutput);

        let connections = vec!(first_level,
                               second_level,
                               third_level);

        let collapsed = collapse_connections(&connections);
        assert_eq!(collapsed.len(), 1);
        assert_eq!(collapsed[0].from_io.route(), "/value");
        assert_eq!(collapsed[0].to_io.route(), "/f2/func/in");
    }

    #[test]
    fn doesnt_collapse_a_non_connection() {
        let one = Connection {
            name: Some("left".to_string()),
            from: "/f1/a".to_string(),
            to: "/f2/a".to_string(),
            from_io: IO::new(&"String".to_string(), &"/f1/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f2/a".to_string()),
        };

        let other = Connection {
            name: Some("right".to_string()),
            from: "/f3/a".to_string(),
            to: "/f4/a".to_string(),
            from_io: IO::new(&"String".to_string(), &"/f3/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f4/a".to_string()),
        };

        let connections = vec!(one, other);
        let collapsed = collapse_connections(&connections);
        assert_eq!(collapsed.len(), 2);
    }
}
