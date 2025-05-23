use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use log::{debug, info};
use serde_derive::Serialize;
use url::Url;

use flowcore::model::connection::Connection;
use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::function_definition::FunctionDefinition;
use flowcore::model::name::HasName;
use flowcore::model::output_connection::{OutputConnection, Source};
use flowcore::model::output_connection::Source::{Input, Output};
use flowcore::model::route::{HasRoute, Route};

use crate::compiler::compile_wasm;
use crate::errors::{Result, ResultExt};

use super::checker;
use super::gatherer;
use super::optimizer;

/// `CompilerTables` are built from the flattened and connected flow model in memory and are
/// used to generate the flow's manifest ready to be executed.
#[derive(Serialize, Default)]
pub struct CompilerTables {
    /// The set of connections between functions in the compiled flow
    pub connections: Vec<Connection>,
    /// Map of sources of values and what route they are connected to
    pub sources: BTreeMap<Route, (Source, usize)>,
    /// Map from "route of the output of a function" --> (output name, `source_function_id`)
    pub destination_routes: BTreeMap<Route, (usize, usize, usize)>,
    /// `HashMap` from "route of the input of a function" --> (`destination_function_id`, `input_number`, `flow_id`)
    pub collapsed_connections: Vec<Connection>,
    /// The set of functions left in a flow after it has been flattened, connected and optimized
    pub functions: Vec<FunctionDefinition>,
    /// The set of libraries used by a flow, from their Urls
    pub libs: BTreeSet<Url>,
    /// The set of context functions used by a flow, from their Urls
    pub context_functions: BTreeSet<Url>,
    /// The list of source files that were used in the flow definition
    pub source_files: Vec<String>,
}

impl CompilerTables {
    /// Create a new set of `CompilerTables` for use in compiling a flow
    #[must_use]
    pub fn new() -> Self {
        CompilerTables {
            connections: Vec::new(),
            sources: BTreeMap::<Route, (Source, usize)>::new(),
            destination_routes: BTreeMap::<Route, (usize, usize, usize)>::new(),
            collapsed_connections: Vec::new(),
            functions: Vec::new(),
            libs: BTreeSet::new(),
            context_functions: BTreeSet::new(),
            source_files: Vec::new(),
        }
    }

    // TODO limit lifetime to table lifetime and return a reference
    // TODO make collapsed_connections a Map and just try and get by to_io.route()
    /// Return an optional connection found to a destination input
    #[must_use]
    pub fn connection_to(&self, input: &Route) -> Option<Connection> {
        for connection in &self.collapsed_connections {
            if connection.to_io().route() == input {
                return Some(connection.clone());
            }
        }
        None
    }

    /// consistently order the functions so each compile produces the same numbering
    pub fn sort_functions(&mut self) {
        self.functions.sort_by_key(FunctionDefinition::get_id);
    }

    /// Construct two look-up tables that can be used to find the index of a function in the functions table,
    /// and the index of it's input - using the input route, or it's output route
    pub fn create_routes_table(&mut self) {
        for function in &mut self.functions {
            // Add inputs to functions to the table as a possible source of connections from a
            // job that completed using this function
            for (input_number, input) in function.get_inputs().iter().enumerate() {
                self.sources.insert(
                    input.route().clone(),
                    (Input(input_number), function.get_id()),
                );
            }

            // Add any output routes it has to the source routes table
            for output in function.get_outputs() {
                self.sources.insert(
                    output.route().clone(),
                    (Output(output.name().to_string()), function.get_id()),
                );
            }

            // Add any inputs it has to the destination routes table
            for (input_index, input) in function.get_inputs().iter().enumerate() {
                self.destination_routes.insert(
                    input.route().clone(),
                    (function.get_id(), input_index, function.get_flow_id()),
                );
            }
        }
    }
}

/// Take a hierarchical flow definition in memory and compile it, generating a manifest for execution
/// of the flow, including references to libraries required.
///
/// # Errors
///
/// Returns an error if the parsed `FlowDefinition` cannot be compiled into a valid set of
/// `CompilerTables`. Possible causes include:
/// - Connections between inputs, outputs, sub-flows etc. cannot be established
/// - The constructed Connections cannot be collapsed joining sources and destinations directly
/// - All function's inputs are connected
/// - The flow does not produce any "side effects" (output)
/// - There is an issue compiling any of the supplied implementations to WASM
pub fn compile(flow: &FlowDefinition,
               output_dir: &Path,
               skip_building: bool,
               optimize: bool,
               #[cfg(feature = "debugger")]
               source_urls: &mut BTreeMap<String, Url>,
) -> Result<CompilerTables> {
    let mut tables = CompilerTables::new();

    gatherer::gather_functions_and_connections(flow, &mut tables)?;
    gatherer::collapse_connections(&mut tables)?;
    if optimize {
        optimizer::optimize(&mut tables);
    }
    checker::check_function_inputs(&tables)?;
    checker::check_side_effects(&tables)?;
    configure_output_connections(&mut tables)?;
    compile_supplied_implementations(
        output_dir,
        &mut tables,
        skip_building,
        optimize,
        source_urls,
    ).chain_err(|| "Could not compile to wasm the flow's supplied implementation(s)")?;

    Ok(tables)
}

/// Calculate the source and output file paths of a provided function implementation to be compiled
///
/// # Errors
///
/// Returns an error if the functions `source_url` cannot be used to form a valid `Url`
///
pub fn get_paths(wasm_output_dir: &Path, function: &FunctionDefinition) -> Result<(PathBuf, PathBuf)> {
    let source_url = function.get_source_url().join(function.get_source())?;

    let source_path = source_url
        .to_file_path()
        .map_err(|()| "Could not convert source url to file path")?;

    let mut wasm_path = wasm_output_dir.join(function.get_source());
    wasm_path.set_extension("wasm");

    Ok((source_path, wasm_path))
}

// For any function that provides an implementation - compile the source to wasm and modify the
// implementation to indicate it is the wasm file
fn compile_supplied_implementations(
    out_dir: &Path,
    tables: &mut CompilerTables,
    skip_building: bool,
    release_build: bool,
    #[cfg(feature = "debugger")]
    source_urls: &mut BTreeMap<String, Url>,
) -> Result<String> {
    for function in &mut tables.functions {
        if function.get_lib_reference().is_none() && function.get_context_reference().is_none() {
            let (implementation_source_path, wasm_destination) = get_paths(out_dir, function)?;
            let mut cargo_target_dir = implementation_source_path.parent()
                .ok_or("Could not get directory where Cargo.toml resides")?.to_path_buf();
            if release_build {
                cargo_target_dir.push("target/wasm32-unknown-unknown/release/");
            } else {
                cargo_target_dir.push("target/wasm32-unknown-unknown/debug/");
            }

            compile_wasm::compile_implementation(
                out_dir,
                cargo_target_dir,
                &wasm_destination,
                &implementation_source_path,
                function,
                skip_building,
                release_build,
                #[cfg(feature = "debugger")]
                    source_urls,
            )?;
        }
    }

    Ok("All supplied implementations compiled successfully".into())
}

// Go through all connections, finding:
// - source function (function id and the output route the connection is from)
// - destination function (function id and input number the connection is to)
//
// Then add an output route to the source function's list of output routes
// (according to each function's output route in the original description plus each connection from
// that route, which could be to multiple destinations)
fn configure_output_connections(tables: &mut CompilerTables) -> Result<()> {
    info!("\n=== Compiler: Configuring Output Connections");
    for connection in &tables.collapsed_connections {
        let (source, source_id) = get_source(&tables.sources,
                                             connection.from_io().route())
            .ok_or(format!("Connection source for route '{}' was not found",
                           connection.from_io().route()))?;

        let (destination_function_id, destination_input_index, destination_flow_id) =
            tables.destination_routes.get(connection.to_io().route())
                .ok_or(format!("Connection destination for route '{}' was not found",
                               connection.to_io().route()))?;

        let source_function = tables.functions.get_mut(source_id)
            .ok_or(format!("Could not find function with id: {source_id} \
            while configuring output connection '{connection}'"))?;

        debug!(
            "Connection: from '{}' to '{}'",
            &connection.from_io().route(),
            &connection.to_io().route()
        );
        debug!("  Source output route = '{source}' --> function #{destination_function_id}:{destination_input_index}");

        let output_conn = OutputConnection::new(
            source,
            *destination_function_id,
            *destination_input_index,
            *destination_flow_id,
            connection.to_io().route().to_string(),
            #[cfg(feature = "debugger")]
                connection.name().to_string(),
        );
        source_function.add_output_connection(output_conn);
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
    source_routes: &BTreeMap<Route, (Source, usize)>,
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
                    Some((Output(format!("{sub_route}")), *function_index))
                } else {
                    Some((
                        Output(format!("/{io_sub_route}{sub_route}")),
                        *function_index,
                    ))
                };
            }
            Some((Input(io_index), function_index)) => {
                return Some((Input(*io_index), *function_index));
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
    #[cfg(feature = "debugger")]
    use std::collections::BTreeMap;
    use std::path::Path;

    use tempfile::tempdir;
    use url::Url;

    use flowcore::model::datatype::STRING_TYPE;
    use flowcore::model::flow_definition::FlowDefinition;
    use flowcore::model::function_definition::FunctionDefinition;
    use flowcore::model::io::IO;
    use flowcore::model::name::{HasName, Name};
    use flowcore::model::output_connection::{OutputConnection, Source};
    use flowcore::model::process_reference::ProcessReference;
    use flowcore::model::route::Route;

    use crate::compiler::compile::{compile, get_paths};

    mod get_source_tests {
        use std::collections::BTreeMap;

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
            BTreeMap<Route, (Source, usize)>,
            Vec<(&'static str, Route, Option<(Source, usize)>)>,
        ) {
            // make sure a corresponding entry (if applicable) is in the table to give the expected response
            let mut test_sources = BTreeMap::<Route, (Source, usize)>::new();
            test_sources.insert(Route::from("/root/f1"), (Source::default(), 0));
            test_sources.insert(
                Route::from("/root/f2/output_value"),
                (Output("output_value".into()), 1),
            );
            test_sources.insert(
                Route::from("/root/f2/output_value_2"),
                (Output("output_value_2".into()), 2),
            );

            // Create a vector of test cases and expected responses
            //                 Input:Test Route    Outputs: Subroute,       Function ID
            let mut test_cases: Vec<(&str, Route, Option<(Source, usize)>)> = vec![(
                "the default IO",
                Route::from("/root/f1"),
                Some((Source::default(), 0)),
            )];
            test_cases.push((
                "array element selected from the default output",
                Route::from("/root/f1/1"),
                Some((Output("/1".into()), 0)),
            ));
            test_cases.push((
                "correctly named IO",
                Route::from("/root/f2/output_value"),
                Some((Output("/output_value".into()), 1)),
            ));
            test_cases.push((
                "incorrectly named function",
                Route::from("/root/f2b"),
                None,
            ));
            test_cases.push((
                "incorrectly named IO",
                Route::from("/root/f2/output_fake"),
                None,
            ));
            test_cases.push((
                "the default IO of a function (which does not exist)",
                Route::from("/root/f2"),
                None,
            ));
            test_cases.push((
                "subroute to part of non-existent function",
                Route::from("/root/f0/sub_struct"),
                None,
            ));
            test_cases.push((
                "subroute to part of a function's default output's structure",
                Route::from("/root/f1/sub_struct"),
                Some((Output("/sub_struct".into()), 0)),
            ));
            test_cases.push((
                "subroute to an array element from part of output's structure",
                Route::from("/root/f1/sub_array/1"),
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
            source: function.get_source_url().to_string(),
            initializations: BTreeMap::new(),
        };

        let _test_flow = FlowDefinition::default();

        let flow = FlowDefinition {
            alias: Name::from("root"),
            name: Name::from("test-flow"),
            process_refs: vec![function_ref],
            source_url: FlowDefinition::default_url(),
            ..Default::default()
        };

        let output_dir = tempdir().expect("A temp dir").keep();
        let mut source_urls = BTreeMap::<String, Url>::new();
        // Optimizer should remove unconnected function leaving no side effects
        match compile(&flow,
                      &output_dir,
                      true,
                      false,
                      #[cfg(feature = "debugger")]
                          &mut source_urls,
        ) {
            Ok(_tables) => panic!("Flow should not compile when it has no side-effects"),
            Err(e) => assert_eq!("Flow has no side-effects", e.description()),
        }
    }

    fn test_function() -> FunctionDefinition {
        FunctionDefinition::new(
            "Stdout".into(),
            false,
            "test.rs".to_string(),
            "print".into(),
            vec![IO::new(vec!(STRING_TYPE.into()), Route::default())],
            vec![IO::new(vec!(STRING_TYPE.into()), Route::default())],
            Url::parse(&format!(
                "file://{}/{}",
                env!("CARGO_MANIFEST_DIR"),
                "tests/test-functions/test/test"
            ))
                .expect("Could not create source Url"),
            Route::from("/flow0/stdout"),
            Some(Url::parse("lib::/tests/test-functions/test/test")
                .expect("Could not parse Url")),
            None,
            vec![OutputConnection::new(
                Source::default(),
                1,
                0,
                0,
                String::default(),
                #[cfg(feature = "debugger")]
                    String::default(),
            )],
            0,
            0,
        )
    }

    #[test]
    fn paths_test() {
        let function = test_function();

        let target_dir = tempdir()
            .expect("Could not create temporary directory during testing")
            .keep();
        let expected_output_wasm = target_dir.join("test.wasm");

        let (impl_source_path, impl_wasm_path) =
            get_paths(&target_dir, &function).expect("Error in 'get_paths'");

        assert_eq!(
            format!(
                "{}/{}",
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .expect("Error getting Manifest Dir")
                    .display(),
                "flowc/tests/test-functions/test/test.rs"
            ),
            impl_source_path
                .to_str()
                .expect("Error converting path to str")
        );
        assert_eq!(expected_output_wasm, impl_wasm_path);
    }
}
