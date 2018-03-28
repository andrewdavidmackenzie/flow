use model::flow::Flow;
use super::gatherer;
use super::optimizer;
use super::connector;
use generator::code_gen::CodeGenTables;
use std::mem::swap;

/// Take a hierarchical flow definition in memory and compile it, generating code that implements
/// the flow, including links to the flowrlib runtime library and library functions used in the
/// flowstdlib standard library. It takes an optional bool dump option to dump to standard output
/// some of the intermediate values and operations during the compilation process.
pub fn compile(flow: &Flow) -> Result<CodeGenTables, String> {
    let mut tables = CodeGenTables::new();

    gatherer::add_entries(flow, &mut tables);
    let mut collapsed_connections = connector::collapse_connections(&tables.connections);
    swap(&mut tables.connections , &mut collapsed_connections);

    connector::connect(&mut tables)?;
    optimizer::prune_tables(&mut tables);

    Ok(tables)
}