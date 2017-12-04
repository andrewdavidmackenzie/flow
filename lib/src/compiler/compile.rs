use model::flow::Flow;
use model::connection::Connection;

pub fn compile(flow: &mut Flow, dump: bool) {
    let mut connection_table: Vec<Connection> = Vec::new();
    add_connections(&mut connection_table, flow);

    let collapsed_table = collapse_connections(&connection_table);

    if dump {
        print_connections(&collapsed_table);
    }
}

fn print_connections(table: &Vec<Connection>) {
    println!("\nConnections:");
    for connection in table.iter() {
        println!("{}", connection);
    }
}

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

fn add_connections(connection_table: &mut Vec<Connection>, flow: &mut Flow) {
    // Add connections from this flow to the table
    if let Some(ref mut connections) = flow.connections {
        connection_table.append(connections);
    }

    // Do the same for all subflows referenced from this one
    if let Some(ref mut flow_refs) = flow.flow_refs {
        for flow_ref in flow_refs {
            add_connections(connection_table, &mut flow_ref.flow);
        }
    }
}