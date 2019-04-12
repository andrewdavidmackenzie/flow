use model::route::Route;
use model::route::Router;
use model::route::HasRoute;
use std::collections::HashMap;
use std::collections::HashSet;
use generator::generate::GenerationTables;
use model::connection::Connection;
use model::name::HasName;
use model::function::Function;

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

pub fn connection_from_function(connections: &Vec<Connection>, function: &Box<Function>) -> bool {
    if let Some(outputs) = function.get_outputs() {
        for output in outputs {
            let route = output.route();
            for connection in connections {
                let connection_route = Router::without_trailing_array_index(connection.from_io.route());
                // connection route without any trailing array index
                if connection_route.0.to_string() == *route {
                    return true;
                }
            }
        }
    }

    false
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
fn find_destinations(from_route: &Route, connections: &Vec<Connection>) -> Vec<Route> {
    let mut destinations = vec!();

    for connection in connections {
        if connection.from_io.route() == from_route {
            if connection.to_io.flow_io() {
                // Keep following connections until you get to one that doesn't end at a flow
                destinations.append(&mut find_destinations(&connection.to_io.route(), connections));
            } else {
                // Found a destination that is not a flow boundary, add it to the list
                destinations.push(connection.to_io.route().clone());
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
        if left.to_io.flow_io() {
            for final_destination in find_destinations(&left.to_io.route(), original_connections) {
                let mut joined_connection = left.clone();
                joined_connection.to_io.set_route(final_destination, false);
                debug!("Collapsed connection {}", joined_connection);
                collapsed_connections.push(joined_connection);
            }
        } else {
            collapsed_connections.push(left.clone());
            debug!("Preserved connection {}", left);
        }
    }

    let connections_before = collapsed_connections.len();
    debug!("Connections resulting: {}", connections_before);

    // Remove connections starting or ending at flow boundaries as they don't go anywhere useful
    collapsed_connections.retain(|conn| !conn.from_io.flow_io() && !conn.to_io.flow_io());
    let connections_after = collapsed_connections.len();
    let dropped_connections = connections_before - connections_after;
    debug!("Dropped {} unused connections to or from flow boundaries", dropped_connections);
    debug!("Connections between functions: {}", connections_after);

    collapsed_connections
}

/*
    Check for a series of potential problems in connections
*/
pub fn check_connections(tables: &mut GenerationTables) -> Result<(), String> {
    check_for_competing_inputs(tables)?;

    remove_duplicates(&mut tables.collapsed_connections)
}

/*
    Check for duplicate connections
*/
pub fn remove_duplicates(connections: &mut Vec<Connection>) -> Result<(), String> {
    let mut uniques = HashSet::<String>::new();

    // keep unique connections - dump duplicates
    connections.retain(|conn| {
        let unique_key = format!("{}->{}", conn.from_io.route(), conn.to_io.route());
        uniques.insert(unique_key)
    });

    Ok(())
}

/*
    Check for two problems that lead to competition for inputs causing input overflow:
    1) Two functions have output connections to the same input, and one of them is a static value
    2) A single function has two output connections to the same destination route.
*/
fn check_for_competing_inputs(tables: &GenerationTables) -> Result<(), String> {
    // HashMap where key is the Route of the input being sent to
    //               value is  a tuple of (sender_id, static_sender)
    // Use to determine when sending to a route if the same function is already sending to it
    // or if there is a different static sender sending to it
    let mut used_destinations = HashMap::<Route, usize>::new();

    for connection in &tables.collapsed_connections {
        if let Some((_output_route, sender_id)) = get_source(&tables.source_routes, &connection.from_io.route()) {
            match used_destinations.insert(connection.to_io.route().clone(), sender_id) {
                Some(other_sender_id) => {
                    // The same function is already sending to this route!
                    if other_sender_id == sender_id {
                        return Err(format!("The function #'{}' has multiple outputs sending to the route '{}'",
                                           sender_id, connection.to_io.route()));
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use model::connection::Connection;
    use model::datatype::DataType;
    use model::route::Route;
    use model::route::HasRoute;
    use model::io::IO;
    use super::collapse_connections;
    use super::remove_duplicates;

    #[test]
    fn drop_useless_connections() {
        let mut unused = Connection {
            name: Some("left".to_string()),
            from: "/f1/a".to_string(),
            to: "/f2/a".to_string(),

            from_io: IO::new(&"String".to_string(), &"/f1/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f2/a".to_string()),
        };
        unused.to_io.set_flow_io(true);

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
        left_side.to_io.set_flow_io(true);

        // This one goes to a flow but then nowhere, so should be dropped
        let mut extra_one = Connection {
            name: Some("unused".to_string()),
            from: "/f2/a".to_string(),
            to: "/f4/a".to_string(),
            from_io: IO::new(&"String".to_string(), &"/f2/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f4/a".to_string()),
        };
        extra_one.from_io.set_name("point b".to_string());
        extra_one.from_io.set_flow_io(true);
        extra_one.to_io.set_name("pointless".to_string());
        extra_one.to_io.set_flow_io(true);


        let mut right_side = Connection {
            name: Some("right".to_string()),
            from: "/f2/a".to_string(),
            to: "/f3/a".to_string(),
            from_io: IO::new(&"String".to_string(), &"/f2/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f3/a".to_string()),
        };
        right_side.from_io.set_flow_io(true);

        let connections = vec!(left_side,
                               extra_one, right_side);

        let collapsed = collapse_connections(&connections);
        assert_eq!(collapsed.len(), 1);
        assert_eq!(collapsed[0].from_io.route(), "/f1/a");
        assert_eq!(collapsed[0].to_io.route(), "/f3/a");
    }

    /*
        Test that when two functions are connected doubly, the connection gets reduced to a single one
    */
    #[test]
    fn collapse_double_connection() {
        let first = Connection {
            name: Some("first".to_string()),
            from: "/r1".to_string(),
            to: "/r2".to_string(),
            from_io: IO::new(&DataType::from("String"), &Route::from("/r1")),
            to_io: IO::new(&DataType::from("String"), &Route::from("/r2")),
        };

        let second = Connection {
            name: Some("second".to_string()),
            from: "/r1".to_string(),
            to: "/r2".to_string(),
            from_io: IO::new(&DataType::from("String"), &Route::from("/r1")),
            to_io: IO::new(&DataType::from("String"), &Route::from("/r2")),
        };

        let mut connections = vec!(first, second);

        assert_eq!(connections.len(), 2);
        remove_duplicates(&mut connections).unwrap();
        assert_eq!(connections.len(), 1);
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
        left_side.to_io.set_flow_io(true);

        let mut right_side_one = Connection {
            name: Some("right1".to_string()),
            from: "/f2/a".to_string(),
            to: "/f2/value1".to_string(),
            from_io: IO::new(&"String".to_string(), &"/f2/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f2/value1".to_string()),
        };
        right_side_one.from_io.set_flow_io(true);

        let mut right_side_two = Connection {
            name: Some("right2".to_string()),
            from: "/f2/a".to_string(),
            to: "/f2/value2".to_string(),
            from_io: IO::new(&"String".to_string(), &"/f2/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f2/value2".to_string()),
        };
        right_side_two.from_io.set_flow_io(true);

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
        first_level.to_io.set_flow_io(true);

        let mut second_level = Connection {
            name: Some("subflow_connection".to_string()),
            from: "/f1/a".to_string(),
            to: "/f2/a".to_string(),
            from_io: IO::new(&"String".to_string(), &"/f1/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f2/a".to_string()),
        };
        second_level.from_io.set_flow_io(true);
        second_level.to_io.set_flow_io(true);

        let mut third_level = Connection {
            name: Some("subsubflow_connection".to_string()),
            from: "/f2/a".to_string(),
            to: "/f2/func/in".to_string(),
            from_io: IO::new(&"String".to_string(), &"/f2/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f2/func/in".to_string()),
        };
        third_level.from_io.set_flow_io(true);

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
