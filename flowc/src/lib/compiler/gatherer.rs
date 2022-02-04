use std::iter::Extend;

use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::function_definition::FunctionDefinition;
use flowcore::model::process::Process::FlowProcess;
use flowcore::model::process::Process::FunctionProcess;

use crate::errors::*;
use crate::generator::generate::GenerationTables;

/// This module is responsible for parsing the flow tree and gathering information into a set of
/// flat tables that the compiler can use for code generation.
pub fn gather_functions_and_connections(flow: &FlowDefinition, tables: &mut GenerationTables) -> Result<()> {
    // Add Connections from this flow hierarchy to the connections table
    let mut connections = flow.connections.clone();
    tables.connections.append(&mut connections);

    // Do the same for all subprocesses referenced from this one
    for subprocess in &flow.subprocesses {
        match subprocess.1 {
            FlowProcess(ref flow) => {
                gather_functions_and_connections(flow, tables)?; // recurse
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
pub fn index_functions(functions: &mut [FunctionDefinition]) {
    for (index, function) in functions.iter_mut().enumerate() {
        function.set_id(index);
    }
}

#[cfg(test)]
mod test {
    use url::Url;

    use flowcore::model::function_definition::FunctionDefinition;
    use flowcore::model::io::IO;
    use flowcore::model::name::Name;
    use flowcore::model::route::Route;
    use flowcore::output_connection::{OutputConnection, Source};

    #[test]
    fn empty_index_test() {
        super::index_functions(&mut[]);
    }

    #[test]
    fn index_test() {
        let function = FunctionDefinition::new(
            Name::from("Stdout"),
            false,
            "lib://context/stdio/stdout".to_string(),
            Name::from("print"),
            vec![],
            vec![IO::new(vec!("String".into()), Route::default())],
            Url::parse("file:///fake/file").expect("Could not parse Url"),
            Route::from("/flow0/stdout"),
            Some("context/stdio/stdout".to_string()),
            vec![OutputConnection::new(
                Source::default(),
                1,
                0,
                0,
                0,
                false,
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
            )],
            99,
            0,
        );

        let mut functions = vec![function.clone(), function];
        super::index_functions(&mut functions);
        assert_eq!(
            functions.get(0).expect("Could not get function").get_id(),
            0
        );
        assert_eq!(
            functions.get(1).expect("Could not get function").get_id(),
            1
        );
    }
}
