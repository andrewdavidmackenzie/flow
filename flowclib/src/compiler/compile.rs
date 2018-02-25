use model::flow::Flow;
use model::value::Value;
use model::function::Function;
use model::connection::Connection;
use super::gatherer;
use super::optimizer;
use super::connector;
use std::collections::HashSet;

pub struct CompilerTables {
    pub connections: Vec<Connection>,
    pub values: Vec<Value>,
    pub functions: Vec<Function>,
    pub libs: HashSet<String>,
    pub lib_references: HashSet<String>,
}

impl CompilerTables {
    pub fn new() -> Self {
        CompilerTables {
            connections: Vec::new(),
            values: Vec::new(),
            functions: Vec::new(),
            libs: HashSet::new(),
            lib_references: HashSet::new(),
        }
    }
}

/// Take a hierarchical flow definition in memory and compile it, generating code that implements
/// the flow, including links to the flowrlib runtime library and library functions used in the
/// flowstdlib standard library. It takes an optional bool dump option to dump to standard output
/// some of the intermediate values and operations during the compilation process.
pub fn compile(flow: &mut Flow) -> CompilerTables {
    let mut tables = CompilerTables::new();
    gatherer::add_entries(flow, &mut tables);

    tables.connections = optimizer::collapse_connections(&tables.connections);
    optimizer::prune_tables(&mut tables);
    connector::connect(&mut tables);

    tables
}