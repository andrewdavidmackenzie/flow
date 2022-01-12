use log::{info, trace};

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
    trace!("compile()");
    let mut tables = GenerationTables::new();

    info!("=== Compiler phase: Gathering");
    gatherer::gather_functions_and_connections(flow, &mut tables)?;
    info!("=== Compiler phase: Collapsing connections");
    tables.collapsed_connections = connector::collapse_connections(&tables.connections);
    info!("=== Compiler phase: Optimizing");
    optimizer::optimize(&mut tables);
    info!("=== Compiler phase: Indexing");
    gatherer::index_functions(&mut tables.functions);
    info!("=== Compiler phase: Calculating routes tables");
    connector::create_routes_table(&mut tables);
    info!("=== Compiler phase: Checking connections");
    checker::check_connections(&mut tables)?;
    info!("=== Compiler phase: Checking processes");
    checker::check_function_inputs(&mut tables)?;
    info!("=== Compiler phase: Checking flow has side-effects");
    checker::check_side_effects(&mut tables)?;
    info!("=== Compiler phase: Preparing functions connections");
    connector::prepare_function_connections(&mut tables)?;

    Ok(tables)
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use url::Url;

    use crate::compiler::compile::compile;
    use crate::model::flow::Flow;
    use crate::model::function::Function;
    use crate::model::io::IO;
    use crate::model::name::{HasName, Name};
    use crate::model::process_reference::ProcessReference;
    use crate::model::route::Route;

    /*
                Test an error is thrown if a flow has no side effects, and that unconnected functions
                are removed by the optimizer
            */
    #[test]
    fn no_side_effects() {
        let function = Function::new(
            Name::from("Stdout"),
            false,
            "lib://context/stdio/stdout.toml".to_owned(),
            Name::from("test-function"),
            vec![IO::new(vec!("String".into()), "/print")],
            vec![],
            Url::parse("lib://context/stdio/stdout.toml").expect("Could not parse Url"),
            Route::from("/print"),
            Some("lib://context/stdio/stdout.toml".to_string()),
            vec![],
            0,
            0,
        );

        let function_ref = ProcessReference {
            alias: function.alias().to_owned(),
            source: function.source_url.to_string(),
            initializations: HashMap::new(),
        };

        let _test_flow = Flow::default();

        let flow = Flow {
            alias: Name::from("context"),
            name: Name::from("test-flow"),
            process_refs: vec![function_ref],
            source_url: Flow::default_url(),
            ..Default::default()
        };

        // Optimizer should remove unconnected function leaving no side-effects
        match compile(&flow) {
            Ok(_tables) => panic!("Flow should not compile when it has no side-effects"),
            Err(e) => assert_eq!("Flow has no side-effects", e.description()),
        }
    }
}
