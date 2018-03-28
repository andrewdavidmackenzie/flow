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
    for connection in &tables.connections {
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

pub fn collapse_connections(original_connections: &Vec<Connection>) -> Vec<Connection> {
    let mut collapsed_table: Vec<Connection> = Vec::new();

    for left in original_connections {
        if left.ends_at_flow {
            for ref right in original_connections {
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

    // TODO this only follows one jump into a sub-flow, it should follow the connection until it
    // doesn't end at a flow, but a function or a value
    // Build the final connection table, leaving out the ones starting or ending at flow boundaries
    let mut final_table: Vec<Connection> = Vec::new();
    for connection in collapsed_table {
        if !connection.starts_at_flow && !connection.ends_at_flow {
            final_table.push(connection.clone());
        }
    }

    final_table
}

#[cfg(test)]
mod test {
    use model::connection::Connection;
    use super::collapse_connections;

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

        let connections = vec!(left_side, right_side);

        let collapsed = collapse_connections(&connections);
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

        let connections = vec!(one, other);
        let collapsed = collapse_connections(&connections);
        assert_eq!(collapsed.len(), 2);
    }
}
