use std::collections::HashMap;
use std::collections::HashSet;

use flowrlib::input::InputInitializer::Constant;
use generator::generate::GenerationTables;
use model::connection::Connection;
use model::name::HasName;
use model::route::HasRoute;
use model::route::Route;
use compiler::connector;

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
fn remove_duplicates(connections: &mut Vec<Connection>) -> Result<(), String> {
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
        if let Some((_output_route, sender_id)) = connector::get_source(&tables.source_routes, &connection.from_io.route()) {
            match used_destinations.insert(connection.to_io.route().clone(), sender_id) {
                Some(other_sender_id) => {
                    // The same function is already sending to this route!
                    if other_sender_id == sender_id {
                        return Err(format!("The function #{} has multiple outputs sending to the route '{}'",
                                           sender_id, connection.to_io.route()));
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

/*
    Check that all Functions have connections to all their inputs or return an error
*/
pub fn check_function_inputs(tables: &mut GenerationTables) -> Result<(), String> {
    for function in &tables.functions {
        if let Some(inputs) = function.get_inputs() {
            for input in inputs {
                match input.get_initializer() {
                    None => {
                        if !connection_to(tables, &input.route()) {
                            return Err(format!("Input '{}' at route '{}' of Function '{}' at route '{}' is not used",
                                               input.name(), input.route(), function.alias(), function.route()));
                        }
                    }
                    Some(Constant(_)) => {
                        // Has a constant initializer and there is another
                        // connections to this input then flag that as an error
                        if connection_to(tables, &input.route()) {
                            return Err(format!("Input '{}' at route '{}' of Function '{}' at route '{}' has a 'constant' initializer and a connection to it",
                                               input.name(), input.route(), function.alias(), function.route()));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn connection_to(tables: &GenerationTables, input: &Route) -> bool {
    for connection in &tables.collapsed_connections {
        if connection.to_io.route() == input {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod test {
    use model::connection::Connection;
    use model::datatype::DataType;
    use model::route::Route;
    use model::io::IO;
    use super::remove_duplicates;

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
}