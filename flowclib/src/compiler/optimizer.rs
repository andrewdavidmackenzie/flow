use generator::generate::GenerationTables;
use model::route::HasRoute;
use compiler::connector;
use model::connection::Connection;
use model::runnable::Runnable;

/*
    Keep removing dead-code (functions or values that have no effect) and any connection that goes
    no-where or comes from nowhere, iteratively until there are none left to remove.
*/
pub fn optimize(tables: &mut GenerationTables) {
    while remove_dead_processes(tables) {}
}

fn remove_dead_processes(tables: &mut GenerationTables) -> bool {
    let mut processes_to_remove = vec!();
    let mut connections_to_remove = vec!();

    for (index, runnable) in tables.runnables.iter().enumerate() {
        if runnable.get_type() == "Value" &&
            !connector::connection_from_runnable(&tables.collapsed_connections, runnable) {
            println!("Process #{} '{}' is a Value with no connection to its output, so will be removed",
                   index, runnable.alias());
            processes_to_remove.push(index);
        }

        if runnable.get_type() == "Function" && dead_function(&tables.collapsed_connections, runnable) {
            println!("Process #{} '{}' @ '{}' is a Function with no connection to its output, so will be removed",
                   index, runnable.alias(), runnable.route());
            processes_to_remove.push(index);
        }
    }

    let process_remove_count = processes_to_remove.len();

    for index in processes_to_remove {
        let removed_process = tables.runnables.remove(index);
        let removed_route = removed_process.route();
        info!("Removing process with index {} at route '{}'", index, removed_route);

        // remove connections to and from it
        for (index, connection) in tables.collapsed_connections.iter().enumerate() {
            if connection.from_io.route().starts_with(removed_route) ||
                connection.to_io.route().starts_with(removed_route) {
                connections_to_remove.push(index);
            }
        }
    }

    connections_to_remove.reverse(); // start from last index to avoid index changes while working
    let connection_remove_count = connections_to_remove.len();

    for connection_index_to_remove in connections_to_remove {
        tables.collapsed_connections.remove(connection_index_to_remove);
    }

    println!("Removed {} processes, {} associated connections",
           process_remove_count, connection_remove_count);

    process_remove_count > 0
}

fn dead_function(connections: &Vec<Connection>, runnable: &Box<Runnable>) -> bool {
    !runnable.is_impure() && false &&
        !connector::connection_from_runnable(connections, runnable) &&
        !connector::connection_to_runnable(connections, runnable)
}