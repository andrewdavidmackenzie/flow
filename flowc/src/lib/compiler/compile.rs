use std::collections::{HashMap, HashSet};
use std::path::Path;

use log::{debug, info};
use serde_derive::Serialize;
use url::Url;

use flowcore::model::connection::{Connection, LOOPBACK_PRIORITY};
use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::function_definition::FunctionDefinition;
use flowcore::model::name::HasName;
use flowcore::model::output_connection::{OutputConnection, Source};
use flowcore::model::output_connection::Source::{Input, Output};
use flowcore::model::route::{HasRoute, Route};

use crate::compiler::compile_wasm;
use crate::errors::*;

use super::checker;
use super::gatherer;
use super::optimizer;

/// `CompilerTables` are built from the flattened and connected flow model in memory and are
/// used to generate the flow's manifest ready to be executed.
#[derive(Serialize, Default)]
pub struct CompilerTables {
    /// The set of connections between functions in the compiled flow
    pub connections: Vec<Connection>,
    /// HashMap of sources of values and what route they are connected to
    pub sources: HashMap<Route, (Source, usize)>,
    /// HashMap from "route of the output of a function" --> (output name, source_function_id)
    pub destination_routes: HashMap<Route, (usize, usize, usize)>,
    /// HashMap from "route of the input of a function" --> (destination_function_id, input number, flow_id)
    pub collapsed_connections: Vec<Connection>,
    /// The set of functions left in a flow after it has been flattened, connected and optimized
    pub functions: Vec<FunctionDefinition>,
    /// The set of libraries used by a flow, from their Urls
    pub libs: HashSet<Url>,
    /// The set of context functions used by a flow, from their Urls
    pub context_functions: HashSet<Url>,
    /// The list of source files that were used in the flow definition
    pub source_files: Vec<String>,
}

impl CompilerTables {
    /// Create a new set of `CompilerTables` for use in compiling a flow
    pub fn new() -> Self {
        CompilerTables {
            connections: Vec::new(),
            sources: HashMap::<Route, (Source, usize)>::new(),
            destination_routes: HashMap::<Route, (usize, usize, usize)>::new(),
            collapsed_connections: Vec::new(),
            functions: Vec::new(),
            libs: HashSet::new(),
            context_functions: HashSet::new(),
            source_files: Vec::new(),
        }
    }
}

/// Take a hierarchical flow definition in memory and compile it, generating a manifest for execution
/// of the flow, including references to libraries required.
pub fn compile(flow: &FlowDefinition, output_dir: &Path,
               skip_building: bool,
               optimize: bool,
               #[cfg(feature = "debugger")] source_urls: &mut HashSet<(Url, Url)>,
    ) -> Result<CompilerTables> {
    let mut tables = CompilerTables::new();

    gatherer::gather_functions_and_connections(flow, &mut tables)?;
    gatherer::collapse_connections(&mut tables);
    if optimize {
        optimizer::optimize(&mut tables);
    }
    checker::check_connections(&mut tables)?;
    checker::check_function_inputs(&mut tables)?;
    checker::check_side_effects(&mut tables)?;
    configuring_output_connections(&mut tables)?;
    checker::check_priorities(&tables)?;
    compile_supplied_implementations(
        output_dir,
        &mut tables,
        skip_building,
        optimize,
        #[cfg(feature = "debugger")] source_urls,
    ).chain_err(|| "Could not compile to wasm the flow's supplied implementation(s)")?;

    Ok(tables)
}

// For any function that provides an implementation - compile the source to wasm and modify the
// implementation to indicate it is the wasm file
fn compile_supplied_implementations(
    out_dir: &Path,
    tables: &mut CompilerTables,
    skip_building: bool,
    release_build: bool,
    #[cfg(feature = "debugger")] source_urls: &mut HashSet<(Url, Url)>,
) -> Result<String> {
    for function in &mut tables.functions {
        if function.get_lib_reference().is_none() && function.get_context_reference().is_none() {
            compile_wasm::compile_implementation(
                out_dir,
                function,
                skip_building,
                release_build,
                #[cfg(feature = "debugger")] source_urls,
            )?;
        }
    }

    Ok("All supplied implementations compiled successfully".into())
}

// Go through all connections, finding:
// - source process (process id and the output route the connection is from)
// - destination process (process id and input number the connection is to)
//
// Then add an output route to the source process's list of output routes
// (according to each function's output route in the original description plus each connection from
// that route, which could be to multiple destinations)
fn configuring_output_connections(tables: &mut CompilerTables) -> Result<()> {
    info!("\n=== Compiler: Configuring Output Connections");
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
                    debug!("  Source output route = '{}' --> function #{}:{}",
                           source, destination_function_id, destination_input_index);

                    // Detect loopback connections and set their priority appropriately
                    let priority = if source_id == destination_function_id {
                        LOOPBACK_PRIORITY
                    } else {
                        connection.get_priority()
                    };

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
                        priority,
                    );
                    source_function.add_output_connection(output_conn);
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

    info!("Output Connections set on all functions");

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
    use std::collections::{HashMap, HashSet};

    use tempdir::TempDir;
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

    /*
        Test an error is thrown if a flow has no side effects, and that unconnected functions
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

        let mut source_urls = HashSet::<(Url, Url)>::new();
        let output_dir = TempDir::new("flow-test").expect("A temp dir").into_path();

        // Optimizer should remove unconnected function leaving no side-effects
        match compile(&flow,
                      &output_dir,
                      true,
                      false,
                      #[cfg(feature = "debugger")] &mut source_urls,
                        ) {
            Ok(_tables) => panic!("Flow should not compile when it has no side-effects"),
            Err(e) => assert_eq!("Flow has no side-effects", e.description()),
        }
    }
}
