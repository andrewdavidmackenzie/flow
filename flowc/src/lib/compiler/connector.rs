use std::collections::HashMap;

use error_chain::bail;
use log::debug;

use flowcore::model::connection::Connection;
use flowcore::model::io::{IO, IOType};
use flowcore::model::name::HasName;
use flowcore::model::output_connection::{OutputConnection, Source};
use flowcore::model::output_connection::Source::{Input, Output};
use flowcore::model::route::HasRoute;
use flowcore::model::route::Route;

use crate::errors::*;
use crate::generator::generate::GenerationTables;

/// Go through all connections, finding:
/// - source process (process id and output route connection is from)
/// - destination process (process id and input number the connection is to)
///
/// Then add an output route to the source process's output routes vector
/// (according to each function's output route in the original description plus each connection from
/// that route, which could be to multiple destinations)
pub fn prepare_function_connections(tables: &mut GenerationTables) -> Result<()> {
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

    debug!("All output routes set on processes");

    Ok(())
}

/// Construct two look-up tables that can be used to find the index of a function in the functions table,
/// and the index of it's input - using the input route or it's output route
pub fn create_routes_table(tables: &mut GenerationTables) {
    for function in &mut tables.functions {
        // Add inputs to functions to the table as a possible source of connections from a
        // job that completed using this function
        for (input_number, input) in function.get_inputs().iter().enumerate() {
            tables.sources.insert(
                input.route().clone(),
                (Input(input_number), function.get_id()),
            );
        }

        // Add any output routes it has to the source routes table
        for output in function.get_outputs() {
            tables.sources.insert(
                output.route().clone(),
                (Output(output.name().to_string()), function.get_id()),
            );
        }

        // Add any inputs it has to the destination routes table
        for (input_index, input) in function.get_inputs().iter().enumerate() {
            tables.destination_routes.insert(
                input.route().clone(),
                (function.get_id(), input_index, function.get_flow_id()),
            );
        }
    }
}

/// Take the original table of connections as gathered from the flow hierarchy, and for each one
/// follow it through any intermediate connections (sub-flow boundaries) to arrive at the final
/// destination. Then create a new direct connection from source to destination and add that
/// to the table of "collapsed" connections which will be used to configure the outputs of the
/// functions.
pub fn collapse_connections(original_connections: &[Connection]) -> Vec<Connection> {
    let mut collapsed_connections: Vec<Connection> = Vec::new();

    debug!(
        "Working on collapsing {} flow connections",
        original_connections.len()
    );

    for connection in original_connections {
        // All collapsed connections must start and end at a Function, so we only build
        // them starting at ones that begin at a Function's IO
        if *connection.from_io().io_type() == IOType::FunctionIO {
            debug!(
                "Trying to create connection from function output at '{}' (level={})",
                connection.from_io().route(),
                connection.level()
            );
            if *connection.to_io().io_type() == IOType::FunctionIO {
                debug!(
                    "\tFound direct connection to function input at '{}'",
                    connection.to_io().route()
                );
                collapsed_connections.push(connection.clone());
            } else {
                // If the connection enters or leaves this flow, then follow it to all destinations at function inputs
                for (source_subroute, destination_io) in find_function_destinations(
                    Route::from(""),
                    connection.to_io().route(),
                    connection.level(),
                    original_connections,
                ) {
                    let mut collapsed_connection = connection.clone();
                    // append the subroute from the origin function IO - to select from with in that IO
                    // as prescribed by the connections along the way
                    let from_route = connection
                        .from_io()
                        .route()
                        .clone()
                        .extend(&source_subroute)
                        .clone();
                    collapsed_connection
                        .from_io_mut()
                        .set_route(&from_route, &IOType::FunctionIO);
                    *collapsed_connection.to_io_mut() = destination_io;
                    debug!("\tIndirect connection {}", collapsed_connection);
                    collapsed_connections.push(collapsed_connection);
                }
            }
        }
    }

    debug!(
        "Connections between functions: {}",
        collapsed_connections.len()
    );

    collapsed_connections
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

/*
    Given a route we have a connection to, attempt to find the final destinations, potentially
    traversing multiple intermediate connections (recursively) until we find any that do not
    end at a flow. Return them as the final destinations.

    As a connection at a flow boundary can connect to multiple destinations, one
    original connection can branch to connect to multiple destinations.

        Chained connections allowed
               origin          intermediate    destination
                 +-----------------+--------------+
               function          flow io       function

        Connections (before combining them) only exist within a given flow, to Functions within it
        or to/from outputs/inputs of that flow, and to/from inputs/outputs of sub-flows referenced

        There are six connection types:
            - Function Input
            - Function Output
            - Sub-flow Input
            - Sub-flow Output
            - Flow Input
            - Flow Output

        If we process from From->To, then there are only 3 source types

        Connection Source types:
            - Function Output
            - Sub-flow Output
            - Flow Input

        and 3 destination types:
            - Function Input
            - Sub-flow Input
            - Flow Output

        Giving nine possible combinations:

         Case Source Type               Destination Type        Description
         ------------------------------------------------------------------------------------------------
   Handled directly by collapse_connections
         1    Function Output           Function Input          Direct connection within a flow
         2    Function Output           Flow Input (subflow)
         3    Function Output           Flow Output             From a function and exits this flow

   Handled by find_function_destinations
         4    Flow Output (subflow)     Function Input          From a subflow into a function in this flow
         5    Flow Output (subflow)     Flow Input (subflow)    Connector between two sub-flows
         6    Flow Output (subflow)     Flow Output (parent)    From a sub-flow and exits this flow

         7    Flow Input (from parent)  Function Input          Enters flow from higher level into a Function
         8    Flow Input (from parent)  Flow Input (subflow)    Enters flow from higher level into a Sub-flow
         9    Flow Input (from parent)  Flow Output             A pass-through connection within a flow

         Output is: source_subroute: Route, final_destination: Route
*/
fn find_function_destinations(
    prev_subroute: Route,
    from_io_route: &Route,
    from_level: usize,
    connections: &[Connection],
) -> Vec<(Route, IO)> {
    let mut destinations = vec![];

    debug!(
        "\tLooking for connections from '{}' on level={}",
        from_io_route, from_level
    );

    let mut found = false;
    for next_connection in connections {
        if let Some(subroute) = next_connection
            .from_io()
            .route()
            .sub_route_of(from_io_route)
        {
            let next_level = match *next_connection.from_io().io_type() {
                // Can't escape the context!
                IOType::FlowOutput if from_level > 0 => from_level - 1,
                IOType::FlowOutput if from_level == 0 => usize::MAX,
                IOType::FlowInput => from_level + 1,
                _ => from_level,
            };

            if next_connection.level() == next_level {
                // Add any subroute from this connection to the origin subroute accumulated so far
                let accumulated_source_subroute = prev_subroute.clone().extend(&subroute).clone();

                match *next_connection.to_io().io_type() {
                    IOType::FunctionIO => {
                        debug!(
                            "\t\tFound destination function input at '{}'",
                            next_connection.to_io().route()
                        );
                        // Found a destination that is a function, add it to the list
                        destinations
                            .push((accumulated_source_subroute, next_connection.to_io().clone()));
                        found = true;
                    }
                    IOType::FlowInput => {
                        debug!(
                            "\t\tFollowing connection into sub-flow via '{}'",
                            from_io_route
                        );
                        let new_destinations = &mut find_function_destinations(
                            accumulated_source_subroute,
                            next_connection.to_io().route(),
                            next_connection.level(),
                            connections,
                        );
                        // TODO accumulate the source subroute that builds up as we go
                        destinations.append(new_destinations);
                    }
                    IOType::FlowOutput => {
                        debug!(
                            "\t\tFollowing connection out of flow via '{}'",
                            from_io_route
                        );
                        let new_destinations = &mut find_function_destinations(
                            accumulated_source_subroute,
                            next_connection.to_io().route(),
                            next_connection.level(),
                            connections,
                        );
                        // TODO accumulate the source subroute that builds up as we go
                        destinations.append(new_destinations);
                    }
                }
            }
        }
    }

    if !found {
        debug!("\t\tEnd of connection chain reached without finding a destination function");
    }

    destinations
}

#[cfg(test)]
mod test {
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

    mod collapse_tests {
        use flowcore::model::connection::Connection;
        use flowcore::model::io::{IO, IOType};
        use flowcore::model::route::HasRoute;
        use flowcore::model::route::Route;

        use super::super::collapse_connections;

        #[test]
        fn collapse_drops_a_useless_connections() {
            let mut unused = Connection::new("/f1/a", "/f2/a");
            unused
                .connect(IO::new(vec!("String".into()), "/f1/a"),
                         IO::new(vec!("String".into()), "/f2/a"), 1)
                .expect("Could not connect IOs");
            unused.to_io_mut().set_io_type(IOType::FlowInput);

            let connections = vec![unused];
            let collapsed = collapse_connections(&connections);
            assert_eq!(collapsed.len(), 0);
        }

        #[test]
        fn collapse_a_connection() {
            let mut left_side = Connection::new("/function1", "/flow2/a");
            left_side
                .connect(
                    IO::new(vec!("String".into()), "/function1"),
                    IO::new(vec!("String".into()), "/flow2/a"),
                    0,
                )
                .expect("Could not connect IOs");
            left_side.from_io_mut().set_io_type(IOType::FunctionIO);
            left_side.to_io_mut().set_io_type(IOType::FlowInput);

            // This one goes to a flow but then nowhere, so should be dropped
            let mut extra_one = Connection::new("/flow2/a", "/flow2/f4/a");
            extra_one
                .connect(
                    IO::new(vec!("String".into()), "/flow2/a"),
                    IO::new(vec!("String".into()), "/flow2/f4/a"),
                    1,
                )
                .expect("Could not connect IOs");
            extra_one.from_io_mut().set_io_type(IOType::FlowInput);
            extra_one.to_io_mut().set_io_type(IOType::FlowInput); // /flow2/f4 doesn't exist

            let mut right_side = Connection::new("/flow2/a", "/flow2/function3");
            right_side
                .connect(
                    IO::new(vec!("String".into()), "/flow2/a"),
                    IO::new(vec!("String".into()), "/flow2/function3"),
                    1,
                )
                .expect("Could not connect IOs");
            right_side.from_io_mut().set_io_type(IOType::FlowInput);
            right_side.to_io_mut().set_io_type(IOType::FunctionIO);

            let connections = vec![left_side, extra_one, right_side];

            let collapsed = collapse_connections(&connections);
            assert_eq!(collapsed.len(), 1);
            assert_eq!(*collapsed[0].from_io().route(), Route::from("/function1"));
            assert_eq!(
                *collapsed[0].to_io().route(),
                Route::from("/flow2/function3")
            );
        }

        /*
            This tests a connection into a sub-flow, that in the sub-flow branches with two
            connections to different elements in it.
            This should result in two end to end connections from the original source to the elements
            in the subflow
        */
        #[test]
        fn collapse_two_connections_from_flow_boundary() {
            let mut left_side = Connection::new("/f1", "/f2/a");
            left_side
                .connect(IO::new(vec!("String".into()), "/f1"),
                         IO::new(vec!("String".into()), "/f2/a"), 0)
                .expect("Could not connect IOs");
            left_side.from_io_mut().set_io_type(IOType::FunctionIO);
            left_side.to_io_mut().set_io_type(IOType::FlowInput);

            let mut right_side_one = Connection::new("/f2/a", "/f2/value1");
            right_side_one
                .connect(
                    IO::new(vec!("String".into()), "/f2/a"),
                    IO::new(vec!("String".into()), "/f2/value1"),
                    1,
                )
                .expect("Could not connect IOs");
            right_side_one.from_io_mut().set_io_type(IOType::FlowInput);
            right_side_one.to_io_mut().set_io_type(IOType::FunctionIO);

            let mut right_side_two = Connection::new("/f2/a", "/f2/value2");
            right_side_two
                .connect(
                    IO::new(vec!("String".into()), "/f2/a"),
                    IO::new(vec!("String".into()), "/f2/value2"),
                    1,
                )
                .expect("Could not connect IOs");
            right_side_two.from_io_mut().set_io_type(IOType::FlowInput);
            right_side_two.to_io_mut().set_io_type(IOType::FunctionIO);

            let connections = vec![left_side, right_side_one, right_side_two];
            assert_eq!(3, connections.len());

            let collapsed = collapse_connections(&connections);
            assert_eq!(2, collapsed.len());
            assert_eq!(Route::from("/f1"), *collapsed[0].from_io().route());
            assert_eq!(Route::from("/f2/value1"), *collapsed[0].to_io().route());
            assert_eq!(Route::from("/f1"), *collapsed[1].from_io().route());
            assert_eq!(Route::from("/f2/value2"), *collapsed[1].to_io().route());
        }

        #[test]
        fn collapse_connection_into_sub_flow() {
            let mut first_level = Connection::new("/value", "/flow1/a");
            first_level
                .connect(
                    IO::new(vec!("String".into()), "/value"),
                    IO::new(vec!("String".into()), "/flow1/a"),
                    0,
                )
                .expect("Could not connect IOs");
            first_level.from_io_mut().set_io_type(IOType::FunctionIO);
            first_level.to_io_mut().set_io_type(IOType::FlowInput);

            let mut second_level = Connection::new("/flow1/a", "/flow1/flow2/a");
            second_level
                .connect(
                    IO::new(vec!("String".into()), "/flow1/a"),
                    IO::new(vec!("String".into()), "/flow1/flow2/a"),
                    1,
                )
                .expect("Could not connect IOs");
            second_level.from_io_mut().set_io_type(IOType::FlowInput);
            second_level.to_io_mut().set_io_type(IOType::FlowInput);

            let mut third_level = Connection::new("/flow1/flow2/a", "/flow1/flow2/func/in");
            third_level
                .connect(
                    IO::new(vec!("String".into()), "/flow1/flow2/a"),
                    IO::new(vec!("String".into()), "/flow1/flow2/func/in"),
                    2,
                )
                .expect("Could not connect IOs");
            third_level.from_io_mut().set_io_type(IOType::FlowInput);
            third_level.to_io_mut().set_io_type(IOType::FunctionIO);

            let connections = vec![first_level, second_level, third_level];

            let collapsed = collapse_connections(&connections);
            assert_eq!(1, collapsed.len());
            assert_eq!(Route::from("/value"), *collapsed[0].from_io().route());
            assert_eq!(
                Route::from("/flow1/flow2/func/in"),
                *collapsed[0].to_io().route()
            );
        }

        #[test]
        fn does_not_collapse_a_non_connection() {
            let mut one = Connection::new("/f1/a", "/f2/a");
            one.connect(IO::new(vec!("String".into()), "/f1/a"),
                        IO::new(vec!("String".into()), "/f2/a"), 1)
                .expect("Could not connect IOs");

            let mut other = Connection::new("/f3/a", "/f4/a");
            other
                .connect(IO::new(vec!("String".into()), "/f3/a"),
                         IO::new(vec!("String".into()), "/f4/a"), 1)
                .expect("Could not connect IOs");
            let connections = vec![one, other];
            let collapsed = collapse_connections(&connections);
            assert_eq!(collapsed.len(), 2);
        }
    }
}
