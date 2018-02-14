use model::flow::Flow;
use model::value::Value;
use model::function::Function;
use model::connection::Connection;
use std::collections::HashSet;

// TODO write tests for all this before any modification
pub fn add_entries(connection_table: &mut Vec<Connection>,
                   value_table: &mut Vec<Value>,
                   function_table: &mut Vec<Function>,
                   lib_table: &mut HashSet<String>,
                   lib_reference_table: &mut HashSet<String>,
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

    for lib_ref in &flow.lib_references {
        let lib_reference = lib_ref.clone();
        let lib_name = lib_reference.split("/").next().unwrap().to_string();
        lib_reference_table.insert(lib_reference);
        lib_table.insert(lib_name);
    }

    // Do the same for all subflows referenced from this one
    if let Some(ref mut flow_refs) = flow.flow_refs {
        for flow_ref in flow_refs {
            add_entries(connection_table, value_table, function_table,
                        lib_table, lib_reference_table, &mut flow_ref.flow);
        }
    }
}