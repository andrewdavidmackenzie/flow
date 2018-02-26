use model::flow::Flow;
use super::gatherer;
use super::optimizer;
use super::connector;
use generator::code_gen::CodeGenTables;

/// Take a hierarchical flow definition in memory and compile it, generating code that implements
/// the flow, including links to the flowrlib runtime library and library functions used in the
/// flowstdlib standard library. It takes an optional bool dump option to dump to standard output
/// some of the intermediate values and operations during the compilation process.
pub fn compile(flow: &mut Flow) -> CodeGenTables {
    let mut tables = CodeGenTables::new();
    gatherer::add_entries(flow, &mut tables);

    tables.connections = optimizer::collapse_connections(&tables.connections);
    optimizer::prune_tables(&mut tables);
    connector::connect(&mut tables);

    tables
}