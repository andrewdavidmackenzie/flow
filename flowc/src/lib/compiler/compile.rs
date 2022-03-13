use std::collections::HashMap;

use log::{debug, info};

use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::name::HasName;
use flowcore::model::output_connection::{OutputConnection, Source};
use flowcore::model::output_connection::Source::{Input, Output};
use flowcore::model::route::{HasRoute, Route};

use crate::compiler::tables::CompilerTables;
use crate::errors::*;

use super::checker;
use super::gatherer;
use super::optimizer;
use super::tables;

/// Take a hierarchical flow definition in memory and compile it, generating a manifest for execution
/// of the flow, including references to libraries required.
pub fn compile(flow: &FlowDefinition) -> Result<CompilerTables> {
    let mut tables = CompilerTables::new();

    info!("=== Compiler phase: Gathering Functions and Connections");
    gatherer::gather_functions_and_connections(flow, &mut tables)?;
    info!("=== Compiler phase: Collapsing connections");
    tables.collapsed_connections = gatherer::collapse_connections(&tables.connections);
    info!("=== Compiler phase: Optimizing");
    optimizer::optimize(&mut tables);
    info!("=== Compiler phase: Indexing Functions");
    gatherer::index_functions(&mut tables.functions);
    info!("=== Compiler phase: Creating routes tables");
    tables::create_routes_table(&mut tables);
    info!("=== Compiler phase: Checking connections");
    checker::check_connections(&mut tables)?;
    info!("=== Compiler phase: Checking Function Inputs");
    checker::check_function_inputs(&mut tables)?;
    info!("=== Compiler phase: Checking flow has side-effects");
    checker::check_side_effects(&mut tables)?;
    info!("=== Compiler phase: Preparing OutputConnections");
    prepare_output_connections(&mut tables)?;

    Ok(tables)
}

/// Go through all connections, finding:
/// - source process (process id and the output route the connection is from)
/// - destination process (process id and input number the connection is to)
///
/// Then add an output route to the source process's list of output routes
/// (according to each function's output route in the original description plus each connection from
/// that route, which could be to multiple destinations)
pub fn prepare_output_connections(tables: &mut CompilerTables) -> Result<()> {
    debug!("Setting output routes on processes");
    for connection in &tables.collapsed_connections {
        if let Some((source, source_id)) = get_source(&tables.sources, connection.from_io().route())
        {
            if let Some(&(destination_function_id, destination_input_index, destination_flow_id)) =
            tables.destination_routes.get(connection.to_io().route())
            {
                if let Some(source_function) = tables.functions.get_mut(source_id) {
                    debug!(
                        "Connection: from '{}' to '{}'",
                        &connection.from_io().route(),
                        &connection.to_io().route()
                    );
                    debug!("  Source output route = '{}' --> Destination: Process ID = {},  Input number = {}",
                           source, destination_function_id, destination_input_index);

                    let output_conn = OutputConnection::new(
                        source,
                        destination_function_id,
                        destination_input_index,
                        destination_flow_id,
                        connection.to_io().datatypes()[0].array_order()?, // TODO
                        connection.to_io().datatypes()[0].is_generic(), // TODO
                        connection.to_io().route().to_string(),
                        #[cfg(feature = "debugger")]
                            connection.name().to_string(),
                    );
                    source_function.add_output_route(output_conn);
                }

                // TODO when connection uses references to real IOs then we maybe able to remove this
                if connection.to_io().get_initializer().is_some() {
                    if let Some(destination_function) =
                    tables.functions.get_mut(destination_function_id)
                    {
                        let destination_input = destination_function
                            .get_mut_inputs()
                            .get_mut(destination_input_index)
                            .ok_or("Could not get inputs")?;
                        if destination_input.get_initializer().is_none() {
                            destination_input.set_initializer(connection.to_io().get_initializer());
                            debug!("Set initializer on destination function '{}' input at '{}' from connection",
                                       destination_function.name(), connection.to_io().route());
                        }
                    }
                }
            } else {
                bail!(
                    "Connection destination for route '{}' was not found",
                    connection.to_io().route()
                );
            }
        } else {
            bail!(
                "Connection source for route '{}' was not found",
                connection.from_io().route()
            );
        }
    }

    debug!("Output routes set on all processes");

    Ok(())
}

/*
    Find a Function's IO using a route to it or subroute of it
    Return an Option:
        None --> The IO was not found
        Some (subroute, function_index) with:
        - The subroute of the IO relative to the function it belongs to
        - The function's index in the compiler's tables
    -  (removing the array index first to find outputs that are arrays, but then adding it back into the subroute) TODO change
*/
fn get_source(
    source_routes: &HashMap<Route, (Source, usize)>,
    from_route: &Route,
) -> Option<(Source, usize)> {
    let mut source_route = from_route.clone();
    let mut sub_route = Route::from("");

    // Look for a function/output or function/input with a route that matches what we are looking for
    // popping off sub-structure sub-path segments until none left
    loop {
        match source_routes.get(&source_route) {
            Some((Output(io_sub_route), function_index)) => {
                return if io_sub_route.is_empty() {
                    Some((Source::Output(format!("{}", sub_route)), *function_index))
                } else {
                    Some((
                        Source::Output(format!("/{}{}", io_sub_route, sub_route)),
                        *function_index,
                    ))
                }
            }
            Some((Input(io_index), function_index)) => {
                return Some((Source::Input(*io_index), *function_index));
            }
            _ => {}
        }

        // pop a route segment off the source route - if there are any left
        match source_route.pop() {
            (_, None) => break,
            (parent, Some(sub)) => {
                source_route = parent.into_owned();
                // insert new route segment at the start of the sub_route we are building up
                sub_route.insert(sub);
                sub_route.insert("/");
            }
        }
    }

    None
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use url::Url;

    use flowcore::model::datatype::STRING_TYPE;
    use flowcore::model::flow_definition::FlowDefinition;
    use flowcore::model::function_definition::FunctionDefinition;
    use flowcore::model::io::IO;
    use flowcore::model::name::{HasName, Name};
    use flowcore::model::process_reference::ProcessReference;
    use flowcore::model::route::Route;

    use crate::compiler::compile::compile;

    mod get_source_tests {
        use std::collections::HashMap;

        use flowcore::model::output_connection::Source;
        use flowcore::model::output_connection::Source::Output;
        use flowcore::model::route::Route;

        use super::super::get_source;

        /*
                    Create a HashTable of routes for use in tests.
                    Each entry (K, V) is:
                    - Key   - the route to a function's IO
                    - Value - a tuple of
                                - sub-route (or IO name) from the function to be used at runtime
                                - the id number of the function in the functions table, to select it at runtime

                    Plus a vector of test cases with the Route to search for and the expected function_id and output sub-route
                */
        #[allow(clippy::type_complexity)]
        fn test_source_routes() -> (
            HashMap<Route, (Source, usize)>,
            Vec<(&'static str, Route, Option<(Source, usize)>)>,
        ) {
            // make sure a corresponding entry (if applicable) is in the table to give the expected response
            let mut test_sources = HashMap::<Route, (Source, usize)>::new();
            test_sources.insert(Route::from("/context/f1"), (Source::default(), 0));
            test_sources.insert(
                Route::from("/context/f2/output_value"),
                (Output("output_value".into()), 1),
            );
            test_sources.insert(
                Route::from("/context/f2/output_value_2"),
                (Output("output_value_2".into()), 2),
            );

            // Create a vector of test cases and expected responses
            //                 Input:Test Route    Outputs: Subroute,       Function ID
            let mut test_cases: Vec<(&str, Route, Option<(Source, usize)>)> = vec![(
                "the default IO",
                Route::from("/context/f1"),
                Some((Source::default(), 0)),
            )];
            test_cases.push((
                "array element selected from the default output",
                Route::from("/context/f1/1"),
                Some((Output("/1".into()), 0)),
            ));
            test_cases.push((
                "correctly named IO",
                Route::from("/context/f2/output_value"),
                Some((Output("/output_value".into()), 1)),
            ));
            test_cases.push((
                "incorrectly named function",
                Route::from("/context/f2b"),
                None,
            ));
            test_cases.push((
                "incorrectly named IO",
                Route::from("/context/f2/output_fake"),
                None,
            ));
            test_cases.push((
                "the default IO of a function (which does not exist)",
                Route::from("/context/f2"),
                None,
            ));
            test_cases.push((
                "subroute to part of non-existent function",
                Route::from("/context/f0/sub_struct"),
                None,
            ));
            test_cases.push((
                "subroute to part of a function's default output's structure",
                Route::from("/context/f1/sub_struct"),
                Some((Output("/sub_struct".into()), 0)),
            ));
            test_cases.push((
                "subroute to an array element from part of output's structure",
                Route::from("/context/f1/sub_array/1"),
                Some((Output("/sub_array/1".into()), 0)),
            ));

            (test_sources, test_cases)
        }

        #[test]
        fn test_get_source() {
            let (test_sources, test_cases) = test_source_routes();

            for test_case in test_cases {
                let found = get_source(&test_sources, &test_case.1);
                assert_eq!(found, test_case.2);
            }
        }
    }

    /* Test an error is thrown if a flow has no side effects, and that unconnected functions
                       are removed by the optimizer
                    */
    #[test]
    fn no_side_effects() {
        let function = FunctionDefinition::new(
            Name::from("Stdout"),
            false,
            "context://stdio/stdout.toml".to_owned(),
            Name::from("test-function"),
            vec![IO::new(vec!(STRING_TYPE.into()), "/print")],
            vec![],
            Url::parse("context://stdio/stdout.toml").expect("Could not parse Url"),
            Route::from("/print"),
            None,
            Some(Url::parse("context://stdio/stdout.toml")
                     .expect("Could not parse Url")),
            vec![],
            0,
            0,
        );

        let function_ref = ProcessReference {
            alias: function.alias().to_owned(),
            source: function.source_url.to_string(),
            initializations: HashMap::new(),
        };

        let _test_flow = FlowDefinition::default();

        let flow = FlowDefinition {
            alias: Name::from("root"),
            name: Name::from("test-flow"),
            process_refs: vec![function_ref],
            source_url: FlowDefinition::default_url(),
            ..Default::default()
        };

        // Optimizer should remove unconnected function leaving no side-effects
        match compile(&flow) {
            Ok(_tables) => panic!("Flow should not compile when it has no side-effects"),
            Err(e) => assert_eq!("Flow has no side-effects", e.description()),
        }
    }
}
