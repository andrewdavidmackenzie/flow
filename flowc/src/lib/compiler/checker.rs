use std::collections::HashMap;
use std::collections::HashSet;

use error_chain::bail;

use flowrstructs::input::InputInitializer::Always;

use crate::compiler::connector;
use crate::errors::*;
use crate::generator::generate::GenerationTables;
use crate::model::connection::Connection;
use crate::model::route::HasRoute;
use crate::model::route::Route;

/*
    Check for a series of potential problems in connections
*/
pub fn check_connections(tables: &mut GenerationTables) -> Result<()> {
    check_for_competing_inputs(tables)?;

    remove_duplicates(&mut tables.collapsed_connections);

    Ok(())
}

/*
    Remove duplicate connections from a list
*/
fn remove_duplicates(connections: &mut Vec<Connection>) {
    let mut uniques = HashSet::<String>::new();

    // keep unique connections - dump duplicates
    connections.retain(|conn| {
        let unique_key = format!("{}->{}", conn.from_io.route(), conn.to_io.route());
        uniques.insert(unique_key)
    });
}

/*
    Check for a problems that lead to competition for inputs causing input overflow:
    - A single function has two output connections to the same destination input
    - a function connects to an input that has a constant initializer
*/
fn check_for_competing_inputs(tables: &GenerationTables) -> Result<()> {
    // HashMap where key is the Route of the input being sent to
    //               value is  a tuple of (sender_id, static_sender)
    // Use to determine when sending to a route if the same function is already sending to it
    // or if there is a different static sender sending to it
    let mut used_destinations = HashMap::<Route, usize>::new();

    for connection in &tables.collapsed_connections {
        // Check for double connection
        if let Some((_output_route, sender_id)) = connector::get_source(&tables.source_routes, &connection.from_io.route()) {
            if let Some(other_sender_id) = used_destinations.insert(connection.to_io.route().clone(), sender_id) {                    // The same function is already sending to this route!
                if other_sender_id == sender_id {
                    bail!("The function #{} has multiple outputs sending to the route '{}'",
                                           sender_id, connection.to_io.route());
                }
            }
        }

        // check for ConstantInitializer at destination
        if let Some(Always(_)) = connection.to_io.get_initializer() {
            bail!("Connection from '{}' to input at '{}' that also has a `always` initializer",
            connection.from_io.route(), connection.to_io.route() );
        }
    }

    Ok(())
}

/*
    Check that all Functions have connections to all their inputs or return an error
*/
pub fn check_function_inputs(tables: &mut GenerationTables) -> Result<()> {
    for function in &tables.functions {
        if let Some(inputs) = function.get_inputs() {
            for input in inputs {
                match input.get_initializer() {
                    None => {
                        if !connection_to(tables, &input.route()) {
                            bail!("Input at route '{}' is not used", input.route());
                        }
                    }
                    Some(Always(_)) => {
                        // Has a constant initializer and there is another
                        // connections to this input then flag that as an error
                        if connection_to(tables, &input.route()) {
                            bail!("Input at route '{}' has a 'constant' initializer and a connection to it",
                                               input.route());
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
    use crate::model::connection::Connection;
    use crate::model::io::IO;
    use crate::model::name::Name;

    use super::remove_duplicates;

    /*
                                                                                    Test that when two functions are connected doubly, the connection gets reduced to a single one
                                                                                */
    #[test]
    fn remove_duplicated_connection() {
        let first = Connection {
            name: Some(Name::from("first")),
            from: "/r1".into(),
            to: "/r2".into(),
            from_io: IO::new("String", "/r1"),
            to_io: IO::new("String", "/r2"),
            level: 0,
        };

        let second = Connection {
            name: Some(Name::from("second")),
            from: "/r1".into(),
            to: "/r2".into(),
            from_io: IO::new("String", "/r1"),
            to_io: IO::new("String", "/r2"),
            level: 0,
        };

        let mut connections = vec!(first, second);

        assert_eq!(connections.len(), 2);
        remove_duplicates(&mut connections);
        assert_eq!(connections.len(), 1);
    }
}