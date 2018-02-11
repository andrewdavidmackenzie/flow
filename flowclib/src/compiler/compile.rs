use model::flow::Flow;
use model::value::Value;
use model::function::Function;
use model::connection::Connection;
use flowrlib::runnable::Runnable;
use super::gatherer;
use super::optimizer;
use super::runnables;

/// Take a hierarchical flow definition in memory and compile it, generating code that implements
/// the flow, including links to the flowrlib runtime library and library functions used in the
/// flowstdlib standard library. It takes an optional bool dump option to dump to standard output
/// some of the intermediate values and operations during the compilation process.
pub fn compile(flow: &mut Flow) ->
    (Vec<Connection>, Vec<Value>, Vec<Function>, Vec<Box<Runnable>>) {
    let mut connection_table: Vec<Connection> = Vec::new();
    let mut value_table: Vec<Value> = Vec::new();
    let mut function_table: Vec<Function> = Vec::new();
    let mut libs_table: Vec<String> = Vec::new();
    let mut lib_references_table: Vec<String> = Vec::new();
    gatherer::add_entries(&mut connection_table, &mut value_table, &mut function_table,
                &mut libs_table, &mut lib_references_table, flow);

    connection_table = optimizer::collapse_connections(&connection_table);
    optimizer::prune_tables(&mut connection_table, &mut value_table, &mut function_table);

    let runnables = runnables::create(&value_table, &function_table, &connection_table);

    (connection_table, value_table, function_table, runnables)
}