use std::collections::HashMap;

use error_chain::bail;
use log::debug;

use flowcore::output_connection::OutputConnection;

use crate::errors::*;
use crate::generator::generate::GenerationTables;
use crate::model::connection::Connection;
use crate::model::io::{IOType, IO};
use crate::model::name::HasName;
use crate::model::route::HasRoute;
use crate::model::route::Route;

/*
    Go through all connections, finding:
      - source process (process id and output route connection is from)
      - destination process (process id and input number the connection is to)

    Then add an output route to the source process's output routes vector
    (according to each function's output route in the original description plus each connection from
     that route, which could be to multiple destinations)
*/
pub fn prepare_function_connections(tables: &mut GenerationTables) -> Result<()> {
    debug!("Setting output routes on processes");
    for connection in &tables.collapsed_connections {
        if let Some((output_route, source_id)) =
            get_source(&tables.source_routes, &connection.from_io.route())
        {
            if let Some(&(destination_function_id, destination_input_index, destination_flow_id)) =
                tables.destination_routes.get(connection.to_io.route())
            {
                if let Some(source_function) = tables.functions.get_mut(source_id) {
                    debug!(
                        "Connection: from '{}' to '{}'",
                        &connection.from_io.route(),
                        &connection.to_io.route()
                    );
                    debug!("  Source output route = '{}' --> Destination: Process ID = {},  Input number = {}",
                           output_route, destination_function_id, destination_input_index);

                    let output_conn = OutputConnection::new(
                        output_route.to_string(),
                        destination_function_id,
                        destination_input_index,
                        destination_flow_id,
                        connection.to_io.datatype().array_order()?,
                        connection.to_io.datatype().is_generic(),
                        Some(connection.to_io.route().to_string()),
                        #[cfg(feature = "debugger")]
                        connection.name.to_string(),
                    );
                    source_function.add_output_route(output_conn);
                }

                // TODO when connection uses references to real IOs then we maybe able to remove this
                if connection.to_io.get_initializer().is_some() {
                    if let Some(destination_function) =
                        tables.functions.get_mut(destination_function_id)
                    {
                        let destination_input = destination_function
                            .get_mut_inputs()
                            .get_mut(destination_input_index)
                            .unwrap();
                        if destination_input.get_initializer().is_none() {
                            destination_input.set_initializer(connection.to_io.get_initializer());
                            debug!("Set initializer on destination function '{}' input at '{}' from connection",
                                       destination_function.name(), connection.to_io.route());
                        }
                    }
                }
            } else {
                bail!(
                    "Connection destination for route '{}' was not found",
                    connection.to_io.route()
                );
            }
        } else {
            bail!(
                "Connection source for route '{}' was not found",
                connection.from_io.route()
            );
        }
    }

    debug!("All output routes set on processes");

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
pub fn get_source(
    source_routes: &HashMap<Route, (Route, usize)>,
    from_route: &Route,
) -> Option<(Route, usize)> {
    let mut source = from_route.clone();
    let mut sub_route = Route::from("");

    // Look for a function/output with a route that matches what we are looking for
    // popping off sub-structure sub-path segments until none left
    loop {
        if let Some(&(ref io_sub_route, function_index)) = source_routes.get(&source) {
            // TODO see if we can insert the default IO into the table with sub_route "/"
            // then this below can be collapsed into a single statement
            return if io_sub_route.is_empty() {
                Some((Route::from(format!("{}", sub_route)), function_index))
            } else {
                Some((
                    Route::from(&format!("/{}{}", io_sub_route, sub_route)),
                    function_index,
                ))
            };
        }

        // pop a route segment off the route - if there are any left
        match source.pop() {
            (_, None) => break,
            (parent, Some(sub)) => {
                source = parent.into_owned();
                // insert new route segment at the start of the sub_route we are building up
                sub_route.insert(sub);
                sub_route.insert("/");
            }
        }
    }

    None
}

/*
    Construct two look-up tables that can be used to find the index of a function in the functions table,
    and the index of it's input - using the input route or it's output route
*/
pub fn create_routes_table(tables: &mut GenerationTables) {
    for function in &mut tables.functions {
        // Add any output routes it has to the source routes table
        for output in function.get_outputs() {
            tables.source_routes.insert(
                output.route().clone(),
                (Route::from(output.name()), function.get_id()),
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
        if let Some(subroute) = next_connection.from_io.route().sub_route_of(from_io_route) {
            let next_level = match *next_connection.from_io.io_type() {
                // Can't escape the context!
                IOType::FlowOutput if from_level > 0 => from_level - 1,
                IOType::FlowOutput if from_level == 0 => usize::MAX,
                IOType::FlowInput => from_level + 1,
                _ => from_level,
            };

            if next_connection.level == next_level {
                // Add any subroute from this connection to the origin subroute accumulated so far
                let accumulated_source_subroute = prev_subroute.clone().extend(&subroute).clone();

                match *next_connection.to_io.io_type() {
                    IOType::FunctionIO => {
                        debug!(
                            "\t\tFound destination function input at '{}'",
                            next_connection.to_io.route()
                        );
                        // Found a destination that is a function, add it to the list
                        destinations
                            .push((accumulated_source_subroute, next_connection.to_io.clone()));
                        found = true;
                    }
                    IOType::FlowInput => {
                        debug!(
                            "\t\tFollowing connection into sub-flow via '{}'",
                            from_io_route
                        );
                        let new_destinations = &mut find_function_destinations(
                            accumulated_source_subroute,
                            &next_connection.to_io.route(),
                            next_connection.level,
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
                            &next_connection.to_io.route(),
                            next_connection.level,
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

/*
    Take the original table of connections as gathered from the flow hierarchy, and for each one
    follow it through any intermediate connections (sub-flow boundaries) to arrive at the final
    destination. Then create a new direct connection from source to destination and add that
    to the table of "collapsed" connections which will be used to configure the outputs of the
    functions.
*/
pub fn collapse_connections(original_connections: &[Connection]) -> Vec<Connection> {
    let mut collapsed_connections: Vec<Connection> = Vec::new();

    debug!(
        "Working on collapsing {} flow connections",
        original_connections.len()
    );

    for connection in original_connections {
        // All collapsed connections must start and end at a Function, so we only build
        // them starting at ones that begin at a Function's IO
        if *connection.from_io.io_type() == IOType::FunctionIO {
            debug!(
                "Trying to create connection from function output at '{}' (level={})",
                connection.from_io.route(),
                connection.level
            );
            if *connection.to_io.io_type() == IOType::FunctionIO {
                debug!(
                    "\tFound direct connection to function input at '{}'",
                    connection.to_io.route()
                );
                collapsed_connections.push(connection.clone());
            } else {
                // If the connection enters or leaves this flow, then follow it to all destinations at function inputs
                for (source_subroute, destination_io) in find_function_destinations(
                    Route::from(""),
                    &connection.to_io.route(),
                    connection.level,
                    original_connections,
                ) {
                    let mut collapsed_connection = connection.clone();
                    // append the subroute from the origin function IO - to select from with in that IO
                    // as prescribed by the connections along the way
                    let from_route = connection
                        .from_io
                        .route()
                        .clone()
                        .extend(&source_subroute)
                        .clone();
                    collapsed_connection
                        .from_io
                        .set_route(&from_route, &IOType::FunctionIO);
                    collapsed_connection.from = from_route;
                    // collapsed_connection.to_io.set_route(&destination_io.route(), &IOType::FunctionIO);
                    collapsed_connection.to = destination_io.route().to_owned();
                    collapsed_connection.to_io = destination_io;
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

#[cfg(test)]
mod test {
    mod get_source_tests {
        use std::collections::HashMap;

        use crate::model::route::Route;

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
            HashMap<Route, (Route, usize)>,
            Vec<(&'static str, Route, Option<(Route, usize)>)>,
        ) {
            // make sure a corresponding entry (if applicable) is in the table to give the expected response
            let mut test_sources = HashMap::<Route, (Route, usize)>::new();
            test_sources.insert(Route::from("/context/f1"), (Route::from(""), 0));
            test_sources.insert(
                Route::from("/context/f2/output_value"),
                (Route::from("output_value"), 1),
            );
            test_sources.insert(
                Route::from("/context/f2/output_value_2"),
                (Route::from("output_value_2"), 2),
            );

            // Create a vector of test cases and expected responses
            //                 Input:Test Route    Outputs: Subroute,       Function ID
            let mut test_cases: Vec<(&str, Route, Option<(Route, usize)>)> = vec![];

            test_cases.push((
                "the default IO",
                Route::from("/context/f1"),
                Some((Route::from(""), 0 as usize)),
            ));
            test_cases.push((
                "array element selected from the default output",
                Route::from("/context/f1/1"),
                Some((Route::from("/1"), 0 as usize)),
            ));
            test_cases.push((
                "correctly named IO",
                Route::from("/context/f2/output_value"),
                Some((Route::from("/output_value"), 1 as usize)),
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
                Some((Route::from("/sub_struct"), 0 as usize)),
            ));
            test_cases.push((
                "subroute to an array element from part of output's structure",
                Route::from("/context/f1/sub_array/1"),
                Some((Route::from("/sub_array/1"), 0 as usize)),
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
        use crate::model::connection::Connection;
        use crate::model::io::{IOType, IO};
        use crate::model::name::Name;
        use crate::model::route::HasRoute;
        use crate::model::route::Route;

        use super::super::collapse_connections;

        fn test_connection() -> Connection {
            Connection {
                name: Name::from("left"),
                from: Route::from("/f1/a"),
                to: Route::from("/f2/a"),
                from_io: IO::new("String", "/f1/a"),
                to_io: IO::new("String", "/f2/a"),
                level: 0,
            }
        }

        #[test]
        fn collapse_drops_a_useless_connections() {
            let mut unused = test_connection();
            unused.to_io.set_flow_io(IOType::FlowInput);

            let connections = vec![unused];
            let collapsed = collapse_connections(&connections);
            assert_eq!(collapsed.len(), 0);
        }

        #[test]
        fn collapse_a_connection() {
            let mut left_side = Connection {
                name: Name::from("left"),
                from: Route::from("/function1"),
                to: Route::from("/flow2/a"),
                from_io: IO::new("String", "/function1"),
                to_io: IO::new("String", "/flow2/a"),
                level: 0,
            };
            left_side.from_io.set_flow_io(IOType::FunctionIO);
            left_side.to_io.set_flow_io(IOType::FlowInput);

            // This one goes to a flow but then nowhere, so should be dropped
            let mut extra_one = Connection {
                name: Name::from("unused"),
                from: Route::from("/flow2/a"),
                to: Route::from("/flow2/f4/a"),
                from_io: IO::new("String", "/flow2/a"),
                to_io: IO::new("String", "/flow2/f4/a"),
                level: 1,
            };
            extra_one.from_io.set_flow_io(IOType::FlowInput);
            extra_one.to_io.set_flow_io(IOType::FlowInput); // /flow2/f4 doesn't exist

            let mut right_side = Connection {
                name: Name::from("right"),
                from: Route::from("/flow2/a"),
                to: Route::from("/flow2/function3"),
                from_io: IO::new("String", "/flow2/a"),
                to_io: IO::new("String", "/flow2/function3"),
                level: 1,
            };
            right_side.from_io.set_flow_io(IOType::FlowInput);
            right_side.to_io.set_flow_io(IOType::FunctionIO);

            let connections = vec![left_side, extra_one, right_side];

            let collapsed = collapse_connections(&connections);
            assert_eq!(collapsed.len(), 1);
            assert_eq!(*collapsed[0].from_io.route(), Route::from("/function1"));
            assert_eq!(*collapsed[0].to_io.route(), Route::from("/flow2/function3"));
        }

        /*
            This tests a connection into a sub-flow, that in the sub-flow branches with two
            connections to different elements in it.
            This should result in two end to end connections from the original source to the elements
            in the subflow
        */
        #[test]
        fn collapse_two_connections_from_flow_boundary() {
            let mut left_side = Connection {
                name: Name::from("left"),
                from: Route::from("/f1"),
                to: Route::from("/f2/a"),
                from_io: IO::new("String", "/f1"),
                to_io: IO::new("String", "/f2/a"),
                level: 0,
            };
            left_side.from_io.set_flow_io(IOType::FunctionIO);
            left_side.to_io.set_flow_io(IOType::FlowInput);

            let mut right_side_one = Connection {
                name: Name::from("right1"),
                from: Route::from("/f2/a"),
                to: Route::from("/f2/value1"),
                from_io: IO::new("String", "/f2/a"),
                to_io: IO::new("String", "/f2/value1"),
                level: 1,
            };
            right_side_one.from_io.set_flow_io(IOType::FlowInput);
            right_side_one.to_io.set_flow_io(IOType::FunctionIO);

            let mut right_side_two = Connection {
                name: Name::from("right2"),
                from: Route::from("/f2/a"),
                to: Route::from("/f2/value2"),
                from_io: IO::new("String", "/f2/a"),
                to_io: IO::new("String", "/f2/value2"),
                level: 1,
            };
            right_side_two.from_io.set_flow_io(IOType::FlowInput);
            right_side_two.to_io.set_flow_io(IOType::FunctionIO);

            let connections = vec![left_side, right_side_one, right_side_two];
            assert_eq!(3, connections.len());

            let collapsed = collapse_connections(&connections);
            assert_eq!(2, collapsed.len());
            assert_eq!(Route::from("/f1"), *collapsed[0].from_io.route());
            assert_eq!(Route::from("/f2/value1"), *collapsed[0].to_io.route());
            assert_eq!(Route::from("/f1"), *collapsed[1].from_io.route());
            assert_eq!(Route::from("/f2/value2"), *collapsed[1].to_io.route());
        }

        #[test]
        fn collapse_connection_into_sub_flow() {
            let mut first_level = Connection {
                name: Name::from("value-to-f1:a at context level"),
                from: Route::from("/value"),
                to: Route::from("/flow1/a"),
                from_io: IO::new("String", "/value"),
                to_io: IO::new("String", "/flow1/a"),
                level: 0,
            };
            first_level.from_io.set_flow_io(IOType::FunctionIO);
            first_level.to_io.set_flow_io(IOType::FlowInput);

            let mut second_level = Connection {
                name: Name::from("subflow_connection"),
                from: Route::from("/flow1/a"),
                to: Route::from("/flow1/flow2/a"),
                from_io: IO::new("String", "/flow1/a"),
                to_io: IO::new("String", "/flow1/flow2/a"),
                level: 1,
            };
            second_level.from_io.set_flow_io(IOType::FlowInput);
            second_level.to_io.set_flow_io(IOType::FlowInput);

            let mut third_level = Connection {
                name: Name::from("sub_subflow_connection"),
                from: Route::from("/flow1/flow2/a"),
                to: Route::from("/flow1/flow2/func/in"),
                from_io: IO::new("String", "/flow1/flow2/a"),
                to_io: IO::new("String", "/flow1/flow2/func/in"),
                level: 2,
            };
            third_level.from_io.set_flow_io(IOType::FlowInput);
            third_level.to_io.set_flow_io(IOType::FunctionIO);

            let connections = vec![first_level, second_level, third_level];

            let collapsed = collapse_connections(&connections);
            assert_eq!(1, collapsed.len());
            assert_eq!(Route::from("/value"), *collapsed[0].from_io.route());
            assert_eq!(
                Route::from("/flow1/flow2/func/in"),
                *collapsed[0].to_io.route()
            );
        }

        #[test]
        fn does_not_collapse_a_non_connection() {
            let one = test_connection();

            let other = Connection {
                name: Name::from("right"),
                from: Route::from("/f3/a"),
                to: Route::from("/f4/a"),
                from_io: IO::new("String", "/f3/a"),
                to_io: IO::new("String", "/f4/a"),
                level: 0,
            };

            let connections = vec![one, other];
            let collapsed = collapse_connections(&connections);
            assert_eq!(collapsed.len(), 2);
        }
    }
}
