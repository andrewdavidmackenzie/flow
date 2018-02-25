use model::flow::Flow;
use compiler::compile::CompilerTables;

/*
    This module is responsible for parsing the flow tree and gathering information into a set of
    flat tables that the compiler can use for code generation.
*/
pub fn add_entries(flow: &mut Flow, tables: &mut CompilerTables) {
    // Add Connections from this flow to the table
    if let Some(ref mut connections) = flow.connections {
        tables.connections.append(connections);
    }

    // Add Values from this flow to the table
    if let Some(ref mut values) = flow.values {
        tables.values.append(values);
    }

    // Add Functions referenced from this flow to the table
    if let Some(ref mut function_refs) = flow.function_refs {
        for function_ref in function_refs {
            tables.functions.push(function_ref.function.clone());
        }
    }

    for lib_ref in &flow.lib_references {
        let lib_reference = lib_ref.clone();
        let lib_name = lib_reference.split("/").next().unwrap().to_string();
        tables.lib_references.insert(lib_reference);
        tables.libs.insert(lib_name);
    }

    // Do the same for all subflows referenced from this one
    if let Some(ref mut flow_refs) = flow.flow_refs {
        for flow_ref in flow_refs {
            add_entries(&mut flow_ref.flow, tables);
        }
    }
}