use model::flow::Flow;
use super::gatherer;
use super::connector;
use generator::generate::GenerationTables;
use super::checker;
use super::optimizer;

/// Take a hierarchical flow definition in memory and compile it, generating code that implements
/// the flow, including links to the flowrlib runtime library and library functions used in the
/// flowstdlib standard library. It takes an optional bool dump option to dump to standard output
/// some of the intermediate values and operations during the compilation process.
pub fn compile(flow: &Flow) -> Result<GenerationTables, String> {
    let mut tables = GenerationTables::new();

    info!("==== Compiler phase: Gathering");
    gatherer::gather_functions_and_connections(flow, &mut tables);
    info!("==== Compiler phase: Collapsing connections");
    tables.collapsed_connections = connector::collapse_connections(&tables.connections);
    info!("==== Compiler phase: Optimizing");
    optimizer::optimize(&mut tables);
    info!("==== Compiler phase: Indexing");
    gatherer::index_functions(&mut tables.functions);
    info!("==== Compiler phase: Calculating routes tables");
    connector::create_routes_table(&mut tables);
    info!("==== Compiler phase: Checking connections");
    connector::check_connections(&mut tables)?;
    info!("==== Compiler phase: Checking processes");
    checker::check_function_inputs(&mut tables)?;
    info!("==== Compiler phase: Preparing functions connections");
    connector::prepare_function_connections(&mut tables)?;

    Ok(tables)
}

#[cfg(test)]
mod test {
    use super::compile;
    use ::model::flow::Flow;
    use ::model::function::Function;
    use ::model::io::IO;
    use ::model::process::Process::FunctionProcess;
    use ::model::process_reference::ProcessReference;
    use ::model::name::HasName;

    /*
        Test for a function that is dead code. It has no connections to it or from it so will
        never run. So it should be removed by the optimizer and not fail at check stage.
    */
    #[test]
    fn dead_function() {
        let function = Function::new("Stdout".to_string(),
                                         false,
                                         Some("lib://runtime/stdio/stdout.toml".to_string()),
                                         "test-function".to_string(),
                                         Some(vec!(IO::new(&"String".to_string(),
                                                           &"/context/print".to_string()))),
                                         Some(vec!()),
                                         "lib://runtime/stdio/stdout.toml".to_string(),
                                         "/context/print".to_string(),
                                         Some("lib://runtime/stdio/stdout.toml".to_string()),
                                         vec!(),
                                         0,
        );

        let function_ref = ProcessReference {
            alias: function.alias().to_string(),
            source: "lib://runtime/stdio/stdout.toml".to_string(),
            initializations: None,
            source_url: function.get_implementation_source(),
            process: FunctionProcess(function),
        };

        let mut flow = Flow::default();
        flow.alias = "context".to_string();
        flow.name = "test-flow".to_string();
        flow.process_refs = Some(vec!(function_ref));

        let _tables = compile(&flow).unwrap();
    }
}