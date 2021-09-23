use std::iter::Extend;

use crate::errors::*;
use crate::generator::generate::GenerationTables;
use crate::model::flow::Flow;
use crate::model::function::Function;
use crate::model::process::Process::FlowProcess;
use crate::model::process::Process::FunctionProcess;

/// This module is responsible for parsing the flow tree and gathering information into a set of
/// flat tables that the compiler can use for code generation.
pub fn gather_functions_and_connections(
    flow: &Flow,
    tables: &mut GenerationTables,
    level: usize,
) -> Result<()> {
    // Add Connections from this flow hierarchy to the connections table
    let mut connections = flow.connections.clone();
    for con in &mut connections {
        con.level = level;
    }
    tables.connections.append(&mut connections);

    // Do the same for all subprocesses referenced from this one
    for subprocess in &flow.subprocesses {
        match subprocess.1 {
            FlowProcess(ref flow) => {
                gather_functions_and_connections(flow, tables, level + 1)?; // recurse
            }
            FunctionProcess(ref function) => {
                // Add Functions from this flow to the table of functions
                tables.functions.push(function.clone());
            }
        }
    }

    // Add the library references of this flow into the tables list
    let lib_refs = &flow.lib_references;
    tables.libs.extend(lib_refs.iter().cloned());

    Ok(())
}

/*
    Give each function a unique index that will later be used to indicate where outputs get sent
    to, and used in code generation.
*/
pub fn index_functions(functions: &mut Vec<Function>) {
    for (index, function) in functions.iter_mut().enumerate() {
        function.set_id(index);
    }
}
