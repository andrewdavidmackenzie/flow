use model::flow::Flow;
use super::gatherer;
use super::connector;
use generator::generate::GenerationTables;
use super::checker;

/// Take a hierarchical flow definition in memory and compile it, generating code that implements
/// the flow, including links to the flowrlib runtime library and library functions used in the
/// flowstdlib standard library. It takes an optional bool dump option to dump to standard output
/// some of the intermediate values and operations during the compilation process.
pub fn compile(flow: &Flow) -> Result<GenerationTables, String> {
    let mut tables = GenerationTables::new();

    gatherer::gather_runnables_and_connections(flow, &mut tables);
    gatherer::index_runnables(&mut tables.runnables);
    tables.collapsed_connections = connector::collapse_connections(&tables.connections);
    connector::routes_table(&mut tables);
    connector::set_runnable_outputs(&mut tables)?;
    connector::check_connections(&mut tables)?;
    checker::check_process_inputs(&mut tables)?;

    Ok(tables)
}

#[cfg(test)]
mod test {
    use ::loader::loader;
    use flowrlib::provider::Provider;
    use super::compile;
    use ::model::process::Process::FlowProcess;

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
        let parent_route = &"".to_string();

        let alias = &"my process".to_string();
        let test_provider = TestProvider { test_content:
        "flow = 'test'
        [[value]]
        name = 'test-value'
        type = 'Number'
        "
        };
        let url = "file://fake.toml";

        match loader::load_process(parent_route, alias, url, &test_provider) {
            Ok(FlowProcess(flow)) => {
                let tables = compile(&flow);
            },
            Ok(_) => panic!("Didn't load test flow"),
            Err(e) => panic!(e.to_string())
        }
    }
}