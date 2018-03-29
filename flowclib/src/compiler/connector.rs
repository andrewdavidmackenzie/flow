use model::connection::Route;
use std::collections::HashMap;
use generator::code_gen::CodeGenTables;
use model::runnable::Runnable;
use model::connection::Connection;

/*
    First build a table of input routes to (runnable_index, input_index) for all inputs of runnables,
    to enable finding the destination of a connection as (runnable_index, input_index) from a route.

    Then iterate through the values and function setting each one's id and the output routes array setup
    (according to each runnable's output route in the original description plus each connection from it)
    to point to the runnable (by index) and the runnable's input (by index) in the table
*/
pub fn connect(tables: &mut CodeGenTables) -> Result<String, String> {
    let (source_routes, destination_routes) = routes_table(&mut tables.runnables);

    debug!("Building connections");
    for connection in &tables.collapsed_connections {
        let source = source_routes.get(&connection.from_route);
        let destination = destination_routes.get(&connection.to_route);

        if let Some(&(ref output_name, source_id)) = source {
            if let Some(&(destination_id, destination_input_index)) = destination {
                let source_runnable = tables.runnables.get_mut(source_id).unwrap();
                debug!("Connection built: from '{}' to '{}'", &connection.from_route, &connection.to_route);
                source_runnable.add_output_connection((output_name.to_string(), destination_id, destination_input_index));
            } else {
                return Err(format!("Connection destination '{}' not found", connection.to_route));
            }
        } else {
            return Err(format!("Connection source '{}' not found", connection.from_route));
        }
    }
    debug!("All connections built");

    Ok("All connections built".to_string())
}

/*
    Construct a look-up table that we can use to find the index of a runnable in the runnables table,
    and the index of it's input - using the input route
*/
fn routes_table(runnables: &mut Vec<Box<Runnable>>) -> (HashMap<Route, (String, usize)>, HashMap<Route, (usize, usize)>) {
    let mut source_route_table = HashMap::<Route, (String, usize)>::new();
    let mut destination_route_table = HashMap::<Route, (usize, usize)>::new();
    let mut runnable_index = 0;

    for runnable in runnables {
        runnable.set_id(runnable_index);

        // Add any output routes it has to the source routes rable
        if let Some(ref outputs) = runnable.get_outputs() {
            for output in outputs {
                source_route_table.insert(output.route.clone(), (output.name.clone(), runnable_index));
            }
        }

        // Add any inputs it has to the destination routes table
        let mut input_index = 0;
        if let Some(ref inputs) = runnable.get_inputs() {
            for input in inputs {
                destination_route_table.insert(input.route.clone(), (runnable.get_id(), input_index));
                input_index += 1;
            }
        }
        runnable_index += 1;
    }

    debug!("Source routes table built\n{:?}", source_route_table);
    debug!("Destination routes table built\n{:?}", destination_route_table);
    (source_route_table, destination_route_table)
}

/*
    Given a route we have a connection to, attempt to find the final destination, potentially
    traversing multiple intermediate connections (recusively) until we find one that does not
    end at a flow. Return that as the final destination.
*/
fn find_destinations(from_route: &Route, connections: &Vec<Connection>) -> Vec<Route> {
    let mut destinations = vec!();

    for connection in connections {
        if connection.from_route == *from_route {
            if connection.ends_at_flow {
                // Keep following connections until you get to one that doesn't end at a flow
                destinations.append(&mut find_destinations(&connection.to_route, connections));
            } else {
                // Found a destination that is not a flow boundary, add it to the list
                destinations.push(connection.to_route.clone());
            }
        }
    }

    destinations
}

pub fn collapse_connections(original_connections: &Vec<Connection>) -> Vec<Connection> {
    let mut collapsed_table: Vec<Connection> = Vec::new();

    for left in original_connections {
        if left.ends_at_flow {
            for final_destination in find_destinations(&left.to_route, original_connections) {
                let mut joined_connection = left.clone();
                joined_connection.to_route = final_destination;
                joined_connection.ends_at_flow = false;
                collapsed_table.push(joined_connection);
            }
        } else {
            collapsed_table.push(left.clone());
        }
    }

    // Remove connections starting or ending at flow boundaries as they don't go anywhere useful
    collapsed_table.retain(|conn| !conn.starts_at_flow && !conn.ends_at_flow);

    collapsed_table
}

#[cfg(test)]
mod test {
    use model::connection::Connection;
    use super::collapse_connections;

    #[test]
    fn drop_useless_connections() {
        let unused = Connection {
            name: Some("left".to_string()),
            from: "point a".to_string(),
            from_route: "/f1/a".to_string(),
            from_type: "String".to_string(),
            starts_at_flow: false,
            to: "point b".to_string(),
            to_route: "/f2/a".to_string(),
            to_type: "String".to_string(),
            ends_at_flow: true,
        };

        let connections = vec!(unused);
        let collapsed = collapse_connections(&connections);
        assert_eq!(collapsed.len(), 0);
    }

    #[test]
    fn collapse_a_connection() {
        let left_side = Connection {
            name: Some("left".to_string()),
            from: "point a".to_string(),
            from_route: "/f1/a".to_string(),
            from_type: "String".to_string(),
            starts_at_flow: false,
            to: "point b".to_string(),
            to_route: "/f2/a".to_string(),
            to_type: "String".to_string(),
            ends_at_flow: true,
        };

        // This one goes to a flow but then nowhere, so should be dropped
        let extra_one = Connection {
            name: Some("unused".to_string()),
            from: "point b".to_string(),
            from_route: "/f2/a".to_string(),
            from_type: "String".to_string(),
            starts_at_flow: true,
            to: "pointless".to_string(),
            to_route: "/f4/a".to_string(),
            to_type: "String".to_string(),
            ends_at_flow: true,
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
            ends_at_flow: false,
        };

        let connections = vec!(left_side,
                                   extra_one, right_side);

        let collapsed = collapse_connections(&connections);
        println!("collapsed: {:?}", collapsed);
        assert_eq!(collapsed.len(), 1);
        assert_eq!(collapsed[0].from_route, "/f1/a".to_string());
        assert_eq!(collapsed[0].to_route, "/f3/a".to_string());
    }

    /*
        This tests a connection into a sub-flow, that in the sub-flow branches with two
        connections to different elements in it.
        This should result in two end to end connections from the orginal source to the elements
        in the subflow
    */
    #[test]
    fn two_connections_from_flow_boundary() {
        let left_side = Connection {
            name: Some("left".to_string()),
            from: "point a".to_string(),
            from_route: "/f1/a".to_string(),
            from_type: "String".to_string(),
            starts_at_flow: false,
            to: "point b".to_string(),
            to_route: "/f2/a".to_string(),
            to_type: "String".to_string(),
            ends_at_flow: true,
        };

        let right_side_one = Connection {
            name: Some("right1".to_string()),
            from: "point b".to_string(),
            from_route: "/f2/a".to_string(),
            from_type: "String".to_string(),
            starts_at_flow: true,
            to: "point c".to_string(),
            to_route: "/f2/value1".to_string(),
            to_type: "String".to_string(),
            ends_at_flow: false,
        };

        let right_side_two = Connection {
            name: Some("right2".to_string()),
            from: "point c".to_string(),
            from_route: "/f2/a".to_string(),
            from_type: "String".to_string(),
            starts_at_flow: true,
            to: "point c".to_string(),
            to_route: "/f2/value2".to_string(),
            to_type: "String".to_string(),
            ends_at_flow: false,
        };

        let connections = vec!(left_side,
                                   right_side_one,
                                   right_side_two);

        assert_eq!(connections.len(), 3);

        let collapsed = collapse_connections(&connections);
        println!("Connections \n{:?}", collapsed);
        assert_eq!(collapsed.len(), 2);
        assert_eq!(collapsed[0].from_route, "/f1/a".to_string());
        assert_eq!(collapsed[0].to_route, "/f2/value1".to_string());
        assert_eq!(collapsed[1].from_route, "/f1/a".to_string());
        assert_eq!(collapsed[1].to_route, "/f2/value2".to_string());
    }

    #[test]
    fn collapses_connection_traversing_a_flow() {
        let first_level = Connection {
            name: Some("context".to_string()),
            from: "value_in_context".to_string(),
            from_route: "/value".to_string(),
            from_type: "String".to_string(),
            starts_at_flow: false,
            to: "sub-flow".to_string(),
            to_route: "/f1/a".to_string(),
            to_type: "String".to_string(),
            ends_at_flow: true,
        };

        let second_level = Connection {
            name: Some("subflow_connection".to_string()),
            from: "sub-flow".to_string(),
            from_route: "/f1/a".to_string(),
            from_type: "String".to_string(),
            starts_at_flow: true,
            to: "sub-sub-flow".to_string(),
            to_route: "/f2/a".to_string(),
            to_type: "String".to_string(),
            ends_at_flow: true,
        };

        let third_level = Connection {
            name: Some("subsubflow_connection".to_string()),
            from: "sub-sub-flow".to_string(),
            from_route: "/f2/a".to_string(),
            from_type: "String".to_string(),
            starts_at_flow: true,
            to: "function_in_sub_sub_flow".to_string(),
            to_route: "/f2/func/in".to_string(),
            to_type: "String".to_string(),
            ends_at_flow: false,
        };

        let connections = vec!(first_level,
                                   second_level,
                                   third_level);

        let collapsed = collapse_connections(&connections);
        assert_eq!(collapsed.len(), 1);
        assert_eq!(collapsed[0].from_route, "/value".to_string());
        assert_eq!(collapsed[0].to_route, "/f2/func/in".to_string());
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
            ends_at_flow: false,
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
            ends_at_flow: false,
        };

        let connections = vec!(one,other);
        let collapsed = collapse_connections(&connections);
        assert_eq!(collapsed.len(), 2);
    }
}
