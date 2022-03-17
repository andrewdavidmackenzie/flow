//! This module is responsible for parsing the flow tree and gathering information into a set of
//! flat tables that the compiler can use for code generation.

use log::{debug, warn};

use flowcore::model::connection::{Connection, DIRECT_CONNECTION_PRIORITY};
use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::function_definition::FunctionDefinition;
use flowcore::model::io::{IO, IOType};
use flowcore::model::name::HasName;
use flowcore::model::output_connection::Source::{Input, Output};
use flowcore::model::process::Process::FlowProcess;
use flowcore::model::process::Process::FunctionProcess;
use flowcore::model::route::HasRoute;
use flowcore::model::route::Route;

use crate::compiler::compile::CompilerTables;
use crate::errors::*;

/// Recursively go through the flow hierarchy, harvesting out functions and connections within
/// each flow into the `CompilerTables` that will be used in later compiler phases.
pub fn gather_functions_and_connections(flow: &FlowDefinition, tables: &mut CompilerTables) -> Result<()> {
    _gather_functions_and_connections(flow, tables)?;

    index_functions(&mut tables.functions);

    create_routes_table(tables);

    Ok(())
}

fn _gather_functions_and_connections(flow: &FlowDefinition, tables: &mut CompilerTables) -> Result<()> {
    // Add Connections from this flow hierarchy to the connections table
    let mut connections = flow.connections.clone();
    tables.connections.append(&mut connections);

    // Do the same for all subprocesses referenced from this one
    for subprocess in &flow.subprocesses {
        match subprocess.1 {
            FlowProcess(ref flow) => {
                _gather_functions_and_connections(flow, tables)?; // recurse
            }
            FunctionProcess(ref function) => {
                // Add Functions from this flow to the table of functions
                tables.functions.push(function.clone());
            }
        }
    }

    // Add the library references of this flow into the tables list
    let lib_refs = &flow.lib_references;
    tables.libs.extend(lib_refs.iter().cloned());

    // Add the context references of this flow into the tables list
    let context_refs = &flow.context_references;
    tables.context_functions.extend(context_refs.iter().cloned());

    Ok(())
}

// Give each function a unique index
fn index_functions(functions: &mut [FunctionDefinition]) {
    for (index, function) in functions.iter_mut().enumerate() {
        function.set_id(index);
    }
}

// Construct two look-up tables that can be used to find the index of a function in the functions table,
// and the index of it's input - using the input route or it's output route
fn create_routes_table(tables: &mut CompilerTables) {
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
/// Valid initiation points of collapsed connections are:
/// - FunctionIO (Output)
/// - FlowInput with an initializer
/// - FlowOutput with an initializer
///
/// Prerequisites for this compilation phase are:
/// - `tables.functions` is populated by `gather_functions_and_connections`
/// - `tables.functions` functions must be indexed by `gather_functions_and_connections`
/// - `tables.connections`is populated by `gather_functions_and_connections`
/// - `tables.destination_routes` is populated by `create_routes_table`
pub fn collapse_connections(tables: &mut CompilerTables) {
    let mut collapsed_connections: Vec<Connection> = Vec::new();

    debug!("Collapsing {} flow connections", tables.connections.len());

    for connection in &tables.connections {
        match connection.from_io().io_type() {
            // connection starts at a Function output
            &IOType::FunctionIO => {
                debug!("Trying to create connection from function output at '{}'",
                       connection.from_io().route());
                if connection.to_io().io_type() == &IOType::FunctionIO {
                    debug!("\tFound direct connection within the flow to a function input at '{}'",
                connection.to_io().route());
                    let mut collapsed_connection = connection.clone();
                    // Note: This maybe to the same function. Loopback connections will be detected
                    // and their priorities changed later in the process.
                    collapsed_connection.set_priority(DIRECT_CONNECTION_PRIORITY);
                    collapsed_connections.push(collapsed_connection);
                } else {
                    // If the connection enters or leaves this flow, then follow it to all destinations at function inputs
                    for (source_subroute, outermost_level, innermost_level, destination_io) in find_function_destinations(
                        Route::from(""),
                        connection.to_io().route(),
                        connection.level(),
                        &tables.connections,) {
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
                        collapsed_connection.set_priority(innermost_level - outermost_level + 1);
                        *collapsed_connection.to_io_mut() = destination_io;
                        debug!("\tIndirect connection {}", collapsed_connection);
                        collapsed_connections.push(collapsed_connection);
                    }
                }
            },

            // connection starts at a flow input or output that has an initializer on it
            IOType::FlowInput | IOType::FlowOutput if connection.from_io().get_initializer().is_some() => {
                // find the destination functions (the connection could split to multiple destinations)
                for (_, _, _, destination_io) in find_function_destinations(
                    Route::from(""),
                    connection.to_io().route(),
                    connection.level(),
                    &tables.connections,
                ) {
                    // get a mutable reference to the destination function and set the initializer on it
                    if let Some(&(destination_function_id, destination_input_index, _)) =
                        tables.destination_routes.get(destination_io.route()) {
                            if let Some(destination_function) = tables.functions.get_mut(destination_function_id) {
                                let _ = destination_function.set_initial_value(destination_input_index, connection.from_io().get_initializer());
                            }
                        }
                }
            },

            _ => { /* Nothing to do */ }
        }
    }

    debug!("Connections between functions: {}", collapsed_connections.len());

    tables.collapsed_connections = collapsed_connections;
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
) -> Vec<(Route, usize, usize, IO)> {
    let mut destinations = vec![];
    let mut outermost_level = from_level;
    let mut innermost_level = 0;

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

            // Avoid infinite recursion and stack overflow
            if next_connection.level() == next_level {
                // keep track of the level of the outermost flow level this connection goes through
                outermost_level = std::cmp::min(outermost_level, next_level);
                // keep track of the level of the innermost flow level this connection goes through
                innermost_level = std::cmp::max(innermost_level, next_level);

                // Add any subroute from this connection to the origin subroute accumulated so far
                let accumulated_source_subroute = prev_subroute.clone().extend(&subroute).clone();

                match *next_connection.to_io().io_type() {
                    IOType::FunctionIO => {
                        debug!("\t\tFound destination function input at '{}'",
                            next_connection.to_io().route());
                        // Found a destination that is a function, add it to the list
                        // TODO accumulate the source subroute that builds up as we go
                        destinations
                            .push((accumulated_source_subroute, outermost_level, innermost_level,
                                   next_connection.to_io().clone()));
                        found = true;
                    }
                    IOType::FlowInput => {
                        debug!("\t\tFollowing connection into sub-flow via '{}'", from_io_route);
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
                        debug!("\t\tFollowing connection out of flow via '{}'", from_io_route);
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

    if !found { // Some chains or sub-chains of connections maybe dead ends, without that being an error
        warn!("WARN: Connection from '{}' : did not find a destination Function Input", from_io_route);
    }

    destinations
}

#[cfg(test)]
mod test {
    use url::Url;

    use flowcore::model::connection::Connection;
    use flowcore::model::datatype::STRING_TYPE;
    use flowcore::model::function_definition::FunctionDefinition;
    use flowcore::model::io::{IO, IOType};
    use flowcore::model::name::Name;
    use flowcore::model::output_connection::{OutputConnection, Source};
    use flowcore::model::route::HasRoute;
    use flowcore::model::route::Route;

    use crate::compiler::compile::CompilerTables;

    use super::collapse_connections;

    #[test]
    fn collapse_drops_a_useless_connections() {
        let mut unused = Connection::new("/f1/a", "/f2/a");
        unused
            .connect(IO::new(vec!(STRING_TYPE.into()), "/f1/a"),
                     IO::new(vec!(STRING_TYPE.into()), "/f2/a"), 1)
            .expect("Could not connect IOs");
        unused.to_io_mut().set_io_type(IOType::FlowInput);

        let mut tables = CompilerTables::new();
        tables.connections = vec![unused];
        collapse_connections(&mut tables);
        assert_eq!(tables.collapsed_connections.len(), 0);
    }

    #[test]
    fn no_collapse_of_a_loopback_connection() {
        let mut only_connection = Connection::new("/function1/out", "/function1/in");
        only_connection
            .connect(
                IO::new(vec!(STRING_TYPE.into()), "/function1/out"),
                IO::new(vec!(STRING_TYPE.into()), "/function1/in"),
                0,
            ).expect("Could not connect IOs");
        only_connection.from_io_mut().set_io_type(IOType::FunctionIO);
        only_connection.to_io_mut().set_io_type(IOType::FunctionIO);

        let mut tables = CompilerTables::new();
        tables.connections = vec![only_connection];
        collapse_connections(&mut tables);
        assert_eq!(tables.collapsed_connections.len(), 1);
        assert_eq!(*tables.collapsed_connections[0].from_io().route(), Route::from("/function1/out"));
        assert_eq!(*tables.collapsed_connections[0].to_io().route(), Route::from("/function1/in"));
        assert_eq!(tables.collapsed_connections[0].get_priority(), 1); // Loopback set to 0 later in compilation
    }

    #[test]
    fn no_collapse_of_a_direct_connection() {
        let mut only_connection = Connection::new("/function1/out", "/function2/in");
        only_connection
            .connect(
                IO::new(vec!(STRING_TYPE.into()), "/function1/out"),
                IO::new(vec!(STRING_TYPE.into()), "/function2/in"),
                0,
            )
            .expect("Could not connect IOs");
        only_connection.from_io_mut().set_io_type(IOType::FunctionIO);
        only_connection.to_io_mut().set_io_type(IOType::FunctionIO);

        let mut tables = CompilerTables::new();
        tables.connections = vec![only_connection];
        collapse_connections(&mut tables);
        assert_eq!(tables.collapsed_connections.len(), 1);
        assert_eq!(*tables.collapsed_connections[0].from_io().route(), Route::from("/function1/out"));
        assert_eq!(*tables.collapsed_connections[0].to_io().route(), Route::from("/function2/in"));
        assert_eq!(tables.collapsed_connections[0].get_priority(), 1);
    }

    #[test]
    fn collapse_a_connection() {
        let mut left_side = Connection::new("/function1", "/flow2/a");
        left_side
            .connect(
                IO::new(vec!(STRING_TYPE.into()), "/function1"),
                IO::new(vec!(STRING_TYPE.into()), "/flow2/a"),
                0,
            )
            .expect("Could not connect IOs");
        left_side.from_io_mut().set_io_type(IOType::FunctionIO);
        left_side.to_io_mut().set_io_type(IOType::FlowInput);

        // This one goes to a flow but then nowhere, so should be dropped
        let mut extra_one = Connection::new("/flow2/a", "/flow2/f4/a");
        extra_one
            .connect(
                IO::new(vec!(STRING_TYPE.into()), "/flow2/a"),
                IO::new(vec!(STRING_TYPE.into()), "/flow2/f4/a"),
                1,
            )
            .expect("Could not connect IOs");
        extra_one.from_io_mut().set_io_type(IOType::FlowInput);
        extra_one.to_io_mut().set_io_type(IOType::FlowInput); // /flow2/f4 doesn't exist

        let mut right_side = Connection::new("/flow2/a", "/flow2/function3");
        right_side
            .connect(
                IO::new(vec!(STRING_TYPE.into()), "/flow2/a"),
                IO::new(vec!(STRING_TYPE.into()), "/flow2/function3"),
                1,
            )
            .expect("Could not connect IOs");
        right_side.from_io_mut().set_io_type(IOType::FlowInput);
        right_side.to_io_mut().set_io_type(IOType::FunctionIO);

        let mut tables = CompilerTables::new();
        tables.connections = vec![left_side, extra_one, right_side];
        collapse_connections(&mut tables);
        assert_eq!(tables.collapsed_connections.len(), 1);
        assert_eq!(*tables.collapsed_connections[0].from_io().route(), Route::from("/function1"));
        assert_eq!(*tables.collapsed_connections[0].to_io().route(), Route::from("/flow2/function3"));
        assert_eq!(tables.collapsed_connections[0].get_priority(), 2);
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
            .connect(IO::new(vec!(STRING_TYPE.into()), "/f1"),
                     IO::new(vec!(STRING_TYPE.into()), "/f2/a"), 0)
            .expect("Could not connect IOs");
        left_side.from_io_mut().set_io_type(IOType::FunctionIO);
        left_side.to_io_mut().set_io_type(IOType::FlowInput);

        let mut right_side_one = Connection::new("/f2/a", "/f2/value1");
        right_side_one
            .connect(
                IO::new(vec!(STRING_TYPE.into()), "/f2/a"),
                IO::new(vec!(STRING_TYPE.into()), "/f2/value1"),
                1,
            )
            .expect("Could not connect IOs");
        right_side_one.from_io_mut().set_io_type(IOType::FlowInput);
        right_side_one.to_io_mut().set_io_type(IOType::FunctionIO);

        let mut right_side_two = Connection::new("/f2/a", "/f2/value2");
        right_side_two
            .connect(
                IO::new(vec!(STRING_TYPE.into()), "/f2/a"),
                IO::new(vec!(STRING_TYPE.into()), "/f2/value2"),
                1,
            )
            .expect("Could not connect IOs");
        right_side_two.from_io_mut().set_io_type(IOType::FlowInput);
        right_side_two.to_io_mut().set_io_type(IOType::FunctionIO);

        let mut tables = CompilerTables::new();
        tables.connections = vec![left_side, right_side_one, right_side_two];
        collapse_connections(&mut tables);
        assert_eq!(2, tables.collapsed_connections.len());

        assert_eq!(*tables.collapsed_connections[0].from_io().route(), Route::from("/f1"));
        assert_eq!(*tables.collapsed_connections[0].to_io().route(), Route::from("/f2/value1"));
        assert_eq!(tables.collapsed_connections[0].get_priority(), 2);

        assert_eq!(*tables.collapsed_connections[1].from_io().route(), Route::from("/f1"));
        assert_eq!(*tables.collapsed_connections[1].to_io().route(), Route::from("/f2/value2"));
        assert_eq!(tables.collapsed_connections[1].get_priority(), 2);
    }

    #[test]
    fn collapse_connection_into_sub_flow() {
        let mut first_level = Connection::new("/function1/out", "/flow1/a");
        first_level
            .connect(
                IO::new(vec!(STRING_TYPE.into()), "/function1/out"),
                IO::new(vec!(STRING_TYPE.into()), "/flow1/a"),
                0,
            )
            .expect("Could not connect IOs");
        first_level.from_io_mut().set_io_type(IOType::FunctionIO);
        first_level.to_io_mut().set_io_type(IOType::FlowInput);

        let mut second_level = Connection::new("/flow1/a", "/flow1/flow2/a");
        second_level
            .connect(
                IO::new(vec!(STRING_TYPE.into()), "/flow1/a"),
                IO::new(vec!(STRING_TYPE.into()), "/flow1/flow2/a"),
                1,
            )
            .expect("Could not connect IOs");
        second_level.from_io_mut().set_io_type(IOType::FlowInput);
        second_level.to_io_mut().set_io_type(IOType::FlowInput);

        let mut third_level = Connection::new("/flow1/flow2/a", "/flow1/flow2/func/in");
        third_level
            .connect(
                IO::new(vec!(STRING_TYPE.into()), "/flow1/flow2/a"),
                IO::new(vec!(STRING_TYPE.into()), "/flow1/flow2/func/in"),
                2,
            )
            .expect("Could not connect IOs");
        third_level.from_io_mut().set_io_type(IOType::FlowInput);
        third_level.to_io_mut().set_io_type(IOType::FunctionIO);

        let mut tables = CompilerTables::new();
        tables.connections = vec![first_level, second_level, third_level];

        collapse_connections(&mut tables);
        assert_eq!(1, tables.collapsed_connections.len());

        assert_eq!(*tables.collapsed_connections[0].from_io().route(), Route::from("/function1/out"));
        assert_eq!(*tables.collapsed_connections[0].to_io().route(), Route::from("/flow1/flow2/func/in"));
        assert_eq!(tables.collapsed_connections[0].get_priority(), 2);
    }

    #[test]
    fn does_not_collapse_a_non_connection() {
        let mut one = Connection::new("/f1/a", "/f2/a");
        one.connect(IO::new(vec!(STRING_TYPE.into()), "/f1/a"),
                    IO::new(vec!(STRING_TYPE.into()), "/f2/a"), 1)
            .expect("Could not connect IOs");

        let mut other = Connection::new("/f3/a", "/f4/a");
        other
            .connect(IO::new(vec!(STRING_TYPE.into()), "/f3/a"),
                     IO::new(vec!(STRING_TYPE.into()), "/f4/a"), 1)
            .expect("Could not connect IOs");
        let mut tables = CompilerTables::new();
        tables.connections = vec![one, other];
        collapse_connections(&mut tables);
        assert_eq!(tables.collapsed_connections.len(), 2);
    }

    #[test]
    fn empty_index_test() {
        super::index_functions(&mut[]);
    }

    #[test]
    fn index_test() {
        let function = FunctionDefinition::new(
            Name::from("Stdout"),
            false,
            "context://stdio/stdout".to_string(),
            Name::from("print"),
            vec![],
            vec![IO::new(vec!(STRING_TYPE.into()), Route::default())],
            Url::parse("file:///fake/file").expect("Could not parse Url"),
            Route::from("/flow0/stdout"),
            None,
            Some(Url::parse("context://stdio/stdout").expect("Could not parse Url")),
            vec![OutputConnection::new(
                Source::default(),
                1,
                0,
                0,
                0,
                false,
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
                0,
            )],
            99,
            0,
        );

        let mut functions = vec![function.clone(), function];
        super::index_functions(&mut functions);
        assert_eq!(
            functions.get(0).expect("Could not get function").get_id(),
            0
        );
        assert_eq!(
            functions.get(1).expect("Could not get function").get_id(),
            1
        );
    }
}
