use crate::errors::*;
use crate::generator::generate::GenerationTables;
use crate::model::flow::Flow;

use super::checker;
use super::connector;
use super::gatherer;
use super::optimizer;

/// Take a hierarchical flow definition in memory and compile it, generating a manifest for execution
/// of the flow, including references to libraries required.
pub fn compile(flow: &Flow) -> Result<GenerationTables> {
    let mut tables = GenerationTables::new();

    info!("==== Compiler phase: Checking Context");
    checker::check_context(flow)?;
    info!("==== Compiler phase: Gathering");
    gatherer::gather_functions_and_connections(flow, &mut tables, 0);
    info!("==== Compiler phase: Collapsing connections");
    tables.collapsed_connections = connector::collapse_connections(&tables.connections);
    info!("==== Compiler phase: Optimizing");
    optimizer::optimize(&mut tables);
    info!("==== Compiler phase: Indexing");
    gatherer::index_functions(&mut tables.functions);
    info!("==== Compiler phase: Calculating routes tables");
    connector::create_routes_table(&mut tables);
    info!("==== Compiler phase: Checking connections");
    checker::check_connections(&mut tables)?;
    info!("==== Compiler phase: Checking processes");
    checker::check_function_inputs(&mut tables)?;
    info!("==== Compiler phase: Preparing functions connections");
    connector::prepare_function_connections(&mut tables)?;

    // TODO compile supplied functions
    compile_supplied_functions(&mut tables);

    Ok(tables)
}

// TODO move
fn compile_supplied_functions(tables: &mut GenerationTables) {
    for function in &mut tables.functions {
        match function.get_implementation() {
            Some(implementation) => {
                function.set_implementation(implementation.replace(".rs", ".wasm"));
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod test {
    use crate::model::flow::Flow;
    use crate::model::function::Function;
    use crate::model::io::IO;
    use crate::model::name::HasName;
    use crate::model::name::Name;
    use crate::model::process::Process::FunctionProcess;
    use crate::model::process_reference::ProcessReference;
    use crate::model::route::Route;

    use super::compile;

    /*
                    Test for a function that is dead code. It has no connections to it or from it so will
                    never run. So it should be removed by the optimizer and not fail at check stage.
                */
    #[test]
    fn dead_function() {
        let function = Function::new(Name::from("Stdout"),
                                     false,
                                     Some("lib://runtime/stdio/stdout.toml".to_string()),
                                     Name::from("test-function"),
                                     Some(vec!(IO::new("String",
                                                       &Route::from("/context/print")))),
                                     Some(vec!()),
                                     "lib://runtime/stdio/stdout.toml",
                                     Route::from("/context/print"),
                                     Some("lib://runtime/stdio/stdout.toml".to_string()),
                                     vec!(),
                                     0,
        );

        let function_ref = ProcessReference {
            alias: function.alias().to_owned(),
            source: "lib://runtime/stdio/stdout.toml".to_string(),
            initializations: None,
            process: FunctionProcess(function),
        };

        let mut flow = Flow::default();
        flow.alias = Name::from("context");
        flow.name = Name::from("test-flow");
        flow.process_refs = Some(vec!(function_ref));

        let _tables = compile(&flow).unwrap();
    }
}