use model::flow::Flow;
use model:: connection::Connection;

pub fn compile(flow: &mut Flow) {
    let mut connection_table: Vec<Connection> = Vec::new();
    add_connections(&mut connection_table, flow);
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