use crate::generator::generate::GenerationTables;
use crate::model::flow::Flow;
use crate::model::function::Function;
use crate::model::process::Process::FlowProcess;
use crate::model::process::Process::FunctionProcess;

/*
    This module is responsible for parsing the flow tree and gathering information into a set of
    flat tables that the compiler can use for code generation.
*/
pub fn gather_functions_and_connections(flow: &Flow, tables: &mut GenerationTables, level: usize) {
    // Add Connections from this flow hierarchy to the connections table
    if let Some(ref connections) = flow.connections {
        let mut conns = connections.clone();
        for con in &mut conns {
            con.level = level;
        };
        tables.connections.append(&mut conns);
    }

    // Do the same for all subprocesses referenced from this one
    if let Some(ref process_refs) = flow.process_refs {
        for process_ref in process_refs {
            match process_ref.process {
                FlowProcess(ref flow) => {
                    gather_functions_and_connections(flow, tables, level + 1); // recurse
                }
                FunctionProcess(ref function) => {
                    // Add Functions from this flow to the table of functions
                    tables.functions.push(Box::new(function.clone()));
                }
            }
        }
    }

    // Add libraries referenced from this flow to the overall list
    for lib_reference in &flow.lib_references {
        let lib_name = lib_reference.split('/').collect::<Vec<&str>>()[0].to_string();
        tables.libs.insert(format!("lib://{}", lib_name));
    }
}

/*
    Give each function a unique index that will later be used to indicate where outputs get sent
    to, and used in code generation.
*/
pub fn index_functions(functions: &mut Vec<Box<Function>>) {
    for (index, function) in functions.into_iter().enumerate() {
        function.set_id(index);
    }
}