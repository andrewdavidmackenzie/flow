use model::flow::Flow;
use model::process::Process::FlowProcess;
use model::process::Process::FunctionProcess;
use generator::generate::GenerationTables;
use model::function::Function;

/*
    This module is responsible for parsing the flow tree and gathering information into a set of
    flat tables that the compiler can use for code generation.
*/
pub fn gather_functions_and_connections(flow: &Flow, tables: &mut GenerationTables) {
    // Add Connections from this flow hierarchy to the connections table
    if let Some(ref connections) = flow.connections {
        let mut conns = connections.clone();
        tables.connections.append(&mut conns);
    }

    // Do the same for all subprocesses referenced from this one
    if let Some(ref process_refs) = flow.process_refs {
        for process_ref in process_refs {
            match process_ref.process {
                FlowProcess(ref flow) => {
                    gather_functions_and_connections(flow, tables); // recurse
                }
                FunctionProcess(ref function) => {
                    // Add Functions from this flow to the table of functions
                    tables.functions.push(Box::new(function.clone()));
                }
            }
        }
    }
}

/*
    Give each function a unique index that will later be used to indicate where outputs get sent
    to, and used in code generation.
*/
pub fn index_functions(functions: &mut Vec<Box<Function>>) {
    for (index, mut function) in functions.into_iter().enumerate() {
        function.set_id(index);
    }
}