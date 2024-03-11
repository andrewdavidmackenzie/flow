use log::{debug, info};

use flowcore::model::connection::Connection;
use flowcore::model::function_definition::FunctionDefinition;
use flowcore::model::name::HasName;
use flowcore::model::route::HasRoute;

use crate::compiler::compile::CompilerTables;

/// Keep removing dead function (that have no effect) and any connection that goes
/// no-where or comes from nowhere, iteratively until no more can be removed-
///
/// This is iterative, as removing a function can remove the destination to a connection
/// that was valid previously, so the connection can be removed. That in turn can lead to
/// more functions without outputs that should be removed, and so on and so forth.
pub fn optimize(tables: &mut CompilerTables) {
    info!("\n=== Compiler: Optimizing");
    while remove_dead_functions(tables) {}
    info!("All unused functions and unnecessary connections removed");
}

// Remove all the "dead" functions we can find in one iteration of the list of functions
// and remove connections to them after that.
fn remove_dead_functions(tables: &mut CompilerTables) -> bool {
    let mut functions_to_remove = vec![];
    let mut connections_to_remove = vec![];

    for (index, function) in tables.functions.iter().enumerate() {
        if dead_function(&tables.collapsed_connections, function) {
            debug!(
                "Function #{} '{}' @ '{}' has no connection from it, so it will be removed",
                index,
                function.alias(),
                function.route()
            );
            functions_to_remove.push(index);

            let removed_route = function.route();
            // remove connections to and from the function
            for (conn_index, connection) in tables.collapsed_connections.iter().enumerate() {
                if connection
                    .from_io()
                    .route()
                    .sub_route_of(removed_route)
                    .is_some()
                    || connection
                        .to_io()
                        .route()
                        .sub_route_of(removed_route)
                        .is_some()
                {
                    debug!("Connection for removal {}", connection);
                    connections_to_remove.push(conn_index);
                }
            }
        }
    }

    // Remove the functions marked for removal
    let function_remove_count = functions_to_remove.len();
    functions_to_remove.reverse();
    for index in functions_to_remove {
        let removed_function = tables.functions.remove(index);
        debug!(
            "Removed function #{}, with route '{}'",
            index,
            removed_function.route()
        );
    }

    // Remove the connections marked for removal
    let connection_remove_count = connections_to_remove.len();
    connections_to_remove.reverse(); // start from last index to avoid index changes while working
    for connection_index_to_remove in connections_to_remove {
        let removed = tables
            .collapsed_connections
            .remove(connection_index_to_remove);
        debug!("Removed connection: {}", removed);
    }

    let removed_something = (function_remove_count + connection_remove_count) > 0;
    if removed_something {
        info!(
            "Removed {} functions and {} connections",
            function_remove_count, connection_remove_count
        );
    }

    removed_something
}

// A function is "dead" or has no effect if it is pure (not impure) and there is no connection from it
fn dead_function(connections: &[Connection], function: &FunctionDefinition) -> bool {
    // TODO this should be expanded to detect context functions that produce an output but have no
    // TODO connection from it, and context functions that consume an input but have no connection to it
    // TODO and pure functions with no connections to any input
    !function.is_impure() && !connection_from_function(connections, function)
}

fn connection_from_function(connections: &[Connection], function: &FunctionDefinition) -> bool {
    for connection in connections {
        if connection
            .from_io()
            .route()
            .sub_route_of(function.route())
            .is_some()
        {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod test {
    use url::Url;

    use flowcore::model::connection::Connection;
    use flowcore::model::function_definition::FunctionDefinition;
    use flowcore::model::io::{IOSet, IOType};

    use crate::compiler::compile::CompilerTables;
    use crate::compiler::optimizer::optimize;

    fn pure_function() -> FunctionDefinition {
        FunctionDefinition::new(
            "pure".into(),
            false,
            "file:///fake/pure".into(),
            String::new(),
            IOSet::new(),
            IOSet::new(),
            Url::parse("file:///fake/pure").expect("Could not parse Url"),
            "/root/pure_function".into(),
            None,
            None,
            vec!(),
            0,
            0
        )
    }

    fn impure_function() -> FunctionDefinition {
        FunctionDefinition::new(
            "impure".into(),
            true,
            "file:///fake/impure".into(),
            String::new(),
            IOSet::new(),
            IOSet::new(),
            Url::parse("file:///fake/impure").expect("Could not parse Url"),
            "/root/impure_function".into(),
            None,
            None,
            vec!(),
            1,
            0
        )
    }

    #[test]
    fn detect_dead_pure_function_no_connections() {
        assert!(super::dead_function(&[], &pure_function()));
    }

    #[test]
    fn dont_detect_dead_impure_function_no_connections() {
        assert!(!super::dead_function(&[], &impure_function()));
    }

    #[test]
    fn detect_live_pure_function_with_connection() {
        let mut from_connection = Connection::new("/root/pure_function/output",
                                              "/root/other/input");
        from_connection.from_io_mut().set_route(&"/root/pure_function/output".into(),
                                                &IOType::FunctionOutput);
        let connections = vec!(from_connection);

        assert!(!super::dead_function(&connections, &pure_function()));
    }

    #[test]
    fn detect_dead_pure_function_with_bad_connection() {
        let mut from_connection = Connection::new("/root/no_such_function/no_such_output",
                                                  "/root/other/input");
        from_connection.from_io_mut().set_route(&"/root/no_such_function/no_such_output".into(),
                                                &IOType::FunctionOutput);
        let connections = vec!(from_connection);

        assert!(super::dead_function(&connections, &pure_function()));
    }

    #[test]
    fn optimize_empty_tables() {
        let mut tables = CompilerTables::new();
        optimize(&mut tables);
    }

    #[test]
    fn no_optimization_to_be_done() {
        let mut tables = CompilerTables::new();
        tables.functions = vec!(pure_function(), impure_function());

        let mut from_connection = Connection::new("/root/pure_function/output",
                                                  "/root/other/input");
        from_connection.from_io_mut().set_route(&"/root/pure_function/output".into(),
                                                &IOType::FunctionOutput);
        tables.collapsed_connections = vec!(from_connection);

        assert_eq!(tables.functions.len(), 2);
        optimize(&mut tables);
        assert_eq!(tables.functions.len(), 2);
    }

    #[test]
    fn optimization_out_a_dead_one() {
        let mut tables = CompilerTables::new();
        tables.functions = vec!(pure_function(), impure_function());

        assert_eq!(tables.functions.len(), 2);
        optimize(&mut tables);
        assert_eq!(tables.functions.len(), 1);
    }
}