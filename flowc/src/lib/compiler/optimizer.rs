use log::debug;

use flowcore::model::connection::Connection;
use flowcore::model::function_definition::FunctionDefinition;
use flowcore::model::name::HasName;
use flowcore::model::route::HasRoute;

use crate::compiler::tables::CompilerTables;

/*
    Keep removing dead processes (that have no effect) and any connection that goes
    no-where or comes from nowhere, iteratively until no more can be removed
*/
pub fn optimize(tables: &mut CompilerTables) {
    while remove_dead_processes(tables) {}
}

fn remove_dead_processes(tables: &mut CompilerTables) -> bool {
    let mut processes_to_remove = vec![];
    let mut connections_to_remove = vec![];

    for (index, function) in tables.functions.iter().enumerate() {
        if dead_function(&tables.collapsed_connections, function) {
            debug!(
                "Function #{} '{}' @ '{}' has no connection from it, so it will be removed",
                index,
                function.alias(),
                function.route()
            );
            processes_to_remove.push(index);

            let removed_route = function.route();
            // remove connections to and from the process
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

    // Remove the processes marked for removal
    let process_remove_count = processes_to_remove.len();
    processes_to_remove.reverse();
    for index in processes_to_remove {
        let removed_process = tables.functions.remove(index);
        debug!(
            "Removed process #{}, with route '{}'",
            index,
            removed_process.route()
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

    debug!(
        "Removed {} processes, {} associated connections",
        process_remove_count, connection_remove_count
    );

    (process_remove_count + connection_remove_count) > 0
}

/*
    A function is "dead" or has no effect if it is pure and has no connection to the output
*/
fn dead_function(connections: &[Connection], function: &FunctionDefinition) -> bool {
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
