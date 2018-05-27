use model::route::Route;
use model::route::HasRoute;
use std::collections::HashMap;
use generator::code_gen::CodeGenTables;
use model::connection::Connection;
use model::connection;
use model::name::HasName;

/*
    Then iterate through the values and function setting each one's id and the output routes array setup
    (according to each runnable's output route in the original description plus each connection from it)
    to point to the runnable (by index) and the runnable's input (by index) in the table
*/
pub fn set_runnable_outputs(tables: &mut CodeGenTables) -> Result<(), String> {
    debug!("Building connections");
    for connection in &tables.collapsed_connections {
        let source = get_source(&tables.source_routes, &connection.from_io.route());

        if let Some((output_route, source_id)) = source {
            let destination = tables.destination_routes.get(connection.to_io.route());
            if let Some(&(destination_id, destination_input_index)) = destination {
                let source_runnable = tables.runnables.get_mut(source_id).unwrap();
                debug!("Connection built: from '{}' to '{}'", &connection.from_io.route(), &connection.to_io.route());
                source_runnable.add_output_connection((output_route.to_string(), destination_id, destination_input_index));
            } else {
                return Err(format!("Connection destination '{}' not found", connection.to_io.route()));
            }
        } else {
            return Err(format!("Connection source '{}' not found", connection.from_io.route()));
        }
    }
    debug!("All connections built");

    Ok(())
}

/*
    find a runnable using the route to its output (removing the array index first to find outputs that are arrays)
    return a tuple of the sub-route to use (possibly with array index included), and the runnable index
*/
fn get_source<'a>(source_routes: &'a HashMap<Route, (String, usize)>, from_route: &str) -> Option<(String, usize)> {
    let (source_without_index, num, array) = connection::name_without_trailing_number(from_route);
    let source = source_routes.get(&source_without_index.to_string());

    if let Some(&(ref route, runnable_index)) = source {
        if array {
            if route.is_empty() {
                return Some((format!("{}", num), runnable_index)); // avoid leading '/'
            } else {
                return Some((format!("{}/{}", route, num), runnable_index));
            }
        } else {
            return Some((route.to_string(), runnable_index));
        }
    } else {
        return None;
    }
}

/*
    Construct a look-up table that can be used to find the index of a runnable in the runnables table,
    and the index of it's input - using the input route
*/
pub fn routes_table(tables: &mut CodeGenTables) {
    for mut runnable in &mut tables.runnables {
        // Add any output routes it has to the source routes table
        if let Some(ref outputs) = runnable.get_outputs() {
            for output in outputs {
                tables.source_routes.insert(output.route().clone(), (output.name().clone(), runnable.get_id()));
            }
        }

        // Add any inputs it has to the destination routes table
        let mut input_index = 0;
        if let Some(ref inputs) = runnable.get_inputs() {
            for input in inputs {
                tables.destination_routes.insert(input.route().clone(), (runnable.get_id(), input_index));
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
    original connection can branch into multiple.
*/
fn find_destinations(from_route: &Route, connections: &Vec<Connection>) -> Vec<Route> {
    let mut destinations = vec!();

    for connection in connections {
        if connection.from_io.route() == from_route {
            if connection.to_io.flow_io {
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
    runnables.
*/
pub fn collapse_connections(original_connections: &Vec<Connection>) -> Vec<Connection> {
    let mut collapsed_connections: Vec<Connection> = Vec::new();

    for left in original_connections {
        if left.to_io.flow_io {
            for final_destination in find_destinations(&left.to_io.route(), original_connections) {
                let mut joined_connection = left.clone();
                joined_connection.to_io.set_route(final_destination);
                joined_connection.to_io.flow_io = false;
                collapsed_connections.push(joined_connection);
            }
        } else {
            collapsed_connections.push(left.clone());
        }
    }

    // Remove connections starting or ending at flow boundaries as they don't go anywhere useful
    collapsed_connections.retain(|conn| !conn.from_io.flow_io && !conn.to_io.flow_io);

    collapsed_connections
}

/*
    Check for a series of potential problems in connections
*/
pub fn check_connections(tables: &CodeGenTables) -> Result<(), String> {
    for connection in &tables.collapsed_connections {
        connection.check_for_loops("Collapsed Connections list")?;
    }

    check_for_competing_inputs(tables)
}

/*
    When two runnables try to send to the same input, and one of them is a static value (that is
    always available), then there will be a run-time problem as the other input will never be able
    to win. So detect this and report the error.
*/
fn check_for_competing_inputs(tables: &CodeGenTables) -> Result<(), String> {
    let mut used_destinations = HashMap::<Route, bool> ::new();
    for connection in &tables.collapsed_connections {
        if let Some((_output_route, sender_id)) = get_source(&tables.source_routes, &connection.from_io.route()) {
            let sender = tables.runnables.get(sender_id).unwrap();
            match used_destinations.insert(connection.to_io.route().clone(), sender.is_static_value()) {
                Some(other_sender_is_static_value) => {
                    // this destination is being sent to already - if the existing sender or this sender are
                    // static then it's being used by two senders, at least one of which is static :-(
                    if other_sender_is_static_value || sender.is_static_value() {
                        return Err(format!("The route '{}' is being sent to by a static value as well as other outputs, causing competition that will fail at run-time",
                                           connection.to_io.route()));
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
    use model::route::HasRoute;
    use model::io::IO;
    use super::collapse_connections;

    #[test]
    fn drop_useless_connections() {
        let mut unused = Connection {
            name: Some("left".to_string()),
            from: "/f1/a".to_string(),
            to: "/f2/a".to_string(),

            from_io: IO::new(&"String".to_string(), &"/f1/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f2/a".to_string()),
        };
        unused.to_io.flow_io = true;

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
        left_side.to_io.flow_io = true;

        // This one goes to a flow but then nowhere, so should be dropped
        let mut extra_one = Connection {
            name: Some("unused".to_string()),
            from: "/f2/a".to_string(),
            to: "/f4/a".to_string(),
            from_io: IO::new(&"String".to_string(), &"/f2/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f4/a".to_string()),
        };
        extra_one.from_io.set_name("point b".to_string());
        extra_one.from_io.flow_io = true;
        extra_one.to_io.set_name("pointless".to_string());
        extra_one.to_io.flow_io = true;


        let mut right_side = Connection {
            name: Some("right".to_string()),
            from: "/f2/a".to_string(),
            to: "/f3/a".to_string(),
            from_io: IO::new(&"String".to_string(), &"/f2/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f3/a".to_string()),
        };
        right_side.from_io.flow_io = true;

        let connections = vec!(left_side,
                               extra_one, right_side);

        let collapsed = collapse_connections(&connections);
        println!("collapsed: {:?}", collapsed);
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
        left_side.to_io.flow_io = true;

        let mut right_side_one = Connection {
            name: Some("right1".to_string()),
            from: "/f2/a".to_string(),
            to: "/f2/value1".to_string(),
            from_io: IO::new(&"String".to_string(), &"/f2/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f2/value1".to_string()),
        };
        right_side_one.from_io.flow_io = true;

        let mut right_side_two = Connection {
            name: Some("right2".to_string()),
            from: "/f2/a".to_string(),
            to: "/f2/value2".to_string(),
            from_io: IO::new(&"String".to_string(), &"/f2/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f2/value2".to_string()),
        };
        right_side_two.from_io.flow_io = true;

        let connections = vec!(left_side,
                               right_side_one,
                               right_side_two);

        assert_eq!(connections.len(), 3);

        let collapsed = collapse_connections(&connections);
        println!("Connections \n{:?}", collapsed);
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
        first_level.to_io.flow_io = true;

        let mut second_level = Connection {
            name: Some("subflow_connection".to_string()),
            from: "/f1/a".to_string(),
            to: "/f2/a".to_string(),
            from_io: IO::new(&"String".to_string(), &"/f1/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f2/a".to_string()),
        };
        second_level.from_io.flow_io = true;
        second_level.to_io.flow_io = true;

        let mut third_level = Connection {
            name: Some("subsubflow_connection".to_string()),
            from: "/f2/a".to_string(),
            to: "/f2/func/in".to_string(),
            from_io: IO::new(&"String".to_string(), &"/f2/a".to_string()),
            to_io: IO::new(&"String".to_string(), &"/f2/func/in".to_string()),
        };
        third_level.from_io.flow_io = true;

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
