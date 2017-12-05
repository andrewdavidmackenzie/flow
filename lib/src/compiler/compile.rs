use model::flow::Flow;
use model::value::Value;
use model::function::Function;
use model::connection::Connection;
use std::fmt;

pub fn compile(flow: &mut Flow, dump: bool) {
    let mut connection_table: Vec<Connection> = Vec::new();
    let mut value_table: Vec<Value> = Vec::new();
    let mut function_table: Vec<Function> = Vec::new();
    add_entries(&mut connection_table, &mut value_table, &mut function_table, flow);

    let collapsed_table = collapse_connections(&connection_table);

    prune_tables(&mut connection_table, &mut value_table, &mut function_table);

    if dump {
        print(&collapsed_table, "Collapsed Connections");
        print(&value_table, "Values");
        print(&function_table, "Functions");
    }
}

fn print<E: fmt::Display>(table: &Vec<E>, title: &str) {
    println!("\n{}:", title);
    for e in table.iter() {
        println!("{}", e);
    }
}

// TODO write tests for all this before any modification
// TODO Write a test that checks it actually eliminates connections thru multiple levels of flows
fn collapse_connections(complete_table: &Vec<Connection>) -> Vec<Connection> {
    let mut collapsed_table: Vec<Connection> = Vec::new();

    for left in complete_table {
        if left.ends_at_flow {
            for ref right in complete_table {
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

    // Now don't include the ones starting or ending on flows.
    let mut final_table: Vec<Connection> = Vec::new();

    for connection in collapsed_table {
        if !connection.starts_at_flow && !connection.ends_at_flow {
            final_table.push(connection.clone());
        }
    }

    final_table
}

// TODO write tests for all this before any modification
fn add_entries(connection_table: &mut Vec<Connection>,
               value_table: &mut Vec<Value>,
               function_table: &mut Vec<Function>,
               flow: &mut Flow) {
    // Add Connections from this flow to the table
    if let Some(ref mut connections) = flow.connections {
        connection_table.append(connections);
    }

    // Add Values from this flow to the table
    if let Some(ref mut values) = flow.values {
        value_table.append(values);
    }

    // Add Functions referenced from this flow to the table
    if let Some(ref mut function_refs) = flow.function_refs {
        for function_ref in function_refs {
            function_table.push(function_ref.function.clone());
        }
    }

    // Do the same for all subflows referenced from this one
    if let Some(ref mut flow_refs) = flow.flow_refs {
        for flow_ref in flow_refs {
            add_entries(connection_table, value_table, function_table, &mut flow_ref.flow);
        }
    }
}

/*
    Drop the following combinations, with warnings:
    - values that don't have connections from them.
    - values that have only outputs and are not initialized.
    - functions that don't have connections from at least one output.
    - functions that don't have connections to all their inputs.
*/
// TODO implement this
fn prune_tables(connection_table: &mut Vec<Connection>,
                value_table: &mut Vec<Value>,
                function_table: &mut Vec<Function>) {

}