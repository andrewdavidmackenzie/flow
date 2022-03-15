use std::collections::{HashMap, HashSet};

use serde_derive::Serialize;
use url::Url;

use flowcore::model::connection::Connection;
use flowcore::model::function_definition::FunctionDefinition;
use flowcore::model::name::HasName;
use flowcore::model::output_connection::Source::{Input, Output};
use flowcore::model::output_connection::Source;
use flowcore::model::route::{HasRoute, Route};

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
    /// Create a new set of `GenerationTables` for use in compiling a flow
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

/// Construct two look-up tables that can be used to find the index of a function in the functions table,
/// and the index of it's input - using the input route or it's output route
pub fn create_routes_table(tables: &mut CompilerTables) {
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