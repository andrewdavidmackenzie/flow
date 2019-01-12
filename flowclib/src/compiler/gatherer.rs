use model::flow::Flow;
use model::runnable::Runnable;
use model::process::Process::FlowProcess;
use model::process::Process::FunctionProcess;
use generator::code_gen::CodeGenTables;

/*
    This module is responsible for parsing the flow tree and gathering information into a set of
    flat tables that the compiler can use for code generation.
*/
pub fn add_entries(flow: &Flow, tables: &mut CodeGenTables) {
    // Add Connections from this flow to the connections table
    if let Some(ref connections) = flow.connections {
        let mut conns = connections.clone();
        tables.connections.append(&mut conns);
    }

    // Add Values from this flow to the table
    if let Some(ref values) = flow.values {
        for value in values {
            tables.runnables.push(Box::new(value.clone()));
        }
    }

    // Do the same for all subprocesses referenced from this one
    if let Some(ref process_refs) = flow.process_refs {
        for process_ref in process_refs {
            match process_ref.process {
                FlowProcess(ref flow) => {
                    add_entries(flow, tables);
                }
                FunctionProcess(ref function) => {
                    tables.runnables.push(Box::new(function.clone()));
                }
            }
        }
    }

    for lib_ref in &flow.lib_references {
        let lib_reference = lib_ref.clone();
        let lib_name = lib_reference.split("/").next().unwrap().to_string();
        tables.lib_references.insert(lib_reference);
        tables.libs.insert(lib_name);
    }
}

/*
    Give each runnable a unique index that will later be used to indicate where outputs get sent
    to, and used in code generation.
*/
pub fn index_runnables(runnables: &mut Vec<Box<Runnable>>) {
    for (index, mut runnable) in runnables.into_iter().enumerate() {
        runnable.set_id(index);
    }
}