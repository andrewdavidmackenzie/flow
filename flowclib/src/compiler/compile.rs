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
    gatherer::gather_runnables_and_connections(flow, &mut tables);
    info!("==== Compiler phase: Collapsing connections");
    tables.collapsed_connections = connector::collapse_connections(&tables.connections);
    info!("==== Compiler phase: Optimizing");
    optimizer::optimize(&mut tables);
    info!("==== Compiler phase: Indexing");
    gatherer::index_runnables(&mut tables.runnables);
    info!("==== Compiler phase: Calculating routes tables");
    connector::create_routes_table(&mut tables);
    info!("==== Compiler phase: Setting output routes");
    connector::set_runnable_outputs(&mut tables)?;
    info!("==== Compiler phase: Checking connections");
    connector::check_connections(&mut tables)?;
    info!("==== Compiler phase: Checking processes");
    checker::check_process_inputs(&mut tables)?;

    Ok(tables)
}

#[cfg(test)]
mod test {
    use ::compiler::loader;
    use flowrlib::provider::Provider;
    use super::compile;
    use ::model::process::Process::FlowProcess;
    use ::model::flow::Flow;
    use ::model::function::Function;
    use ::model::io::IO;
    use ::model::process::Process::FunctionProcess;
    use ::model::process_reference::ProcessReference;
    use ::model::name::HasName;
    use ::model::runnable::Runnable;

    struct TestProvider {
        test_content: &'static str
    }

    impl Provider for TestProvider {
        fn resolve(&self, url: &str, _default_filename: &str)
                   -> Result<(String, Option<String>), String> {
            Ok((url.to_string(), None))
        }

        fn get(&self, _url: &str) -> Result<Vec<u8>, String> {
            Ok(self.test_content.as_bytes().to_owned())
        }
    }

    /*
        Test for a value that is dead code. It is NOT initialized to a value, and so if no
        connection reads from it then it is dead-code and has no effect.
        The value should be removed, and there should be no connections to it.
    */
    #[test]
    fn dead_value() {
        let test_provider = TestProvider {
            test_content:
            "flow = 'test'
        [[value]]
        name = 'test-value'
        type = 'Number'
        "
        };
        let url = "file://fake.toml";

        match loader::load_context(url, &test_provider) {
            Ok(FlowProcess(flow)) => {
                let tables = compile(&flow).unwrap();
                // Dead value should be removed - currently can't assume that args function can be removed
                assert_eq!(tables.runnables.len(), 0, "Incorrect number of runnables after optimization");
            }
            Ok(_) => panic!("Didn't load test flow"),
            Err(e) => panic!(e.to_string())
        }
    }

    /*
        Test for a function that is dead code. It has no connections to it or from it so will
        never run. So it should be removed by the optimizer and not fail at check stage.
    */
    #[test]
    fn dead_function() {
        let function = Function::new("Stdout".to_string(),
                                         false,
                                         Some("lib://flowr/stdio/stdout.toml".to_string()),
                                         "test-function".to_string(),
                                         Some(vec!(IO::new(&"String".to_string(),
                                                           &"/context/print".to_string()))),
                                         Some(vec!()),
                                         "lib://flowr/stdio/stdout.toml".to_string(),
                                         "/context/print".to_string(),
                                         Some("lib://flowr/stdio/stdout.toml".to_string()),
                                         vec!(),
                                         0,
        );

        let function_ref = ProcessReference {
            alias: function.alias().to_string(),
            source: "lib://flowr/stdio/stdout.toml".to_string(),
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