use model::flow::Flow;
use super::gatherer;
use super::connector;
use generator::generate::GenerationTables;

/// Take a hierarchical flow definition in memory and compile it, generating code that implements
/// the flow, including links to the flowrlib runtime library and library functions used in the
/// flowstdlib standard library. It takes an optional bool dump option to dump to standard output
/// some of the intermediate values and operations during the compilation process.
pub fn compile(flow: &Flow) -> Result<GenerationTables, String> {
    let mut tables = GenerationTables::new();

    gatherer::add_entries(flow, &mut tables);
    gatherer::index_runnables(&mut tables.runnables);
    tables.collapsed_connections = connector::collapse_connections(&tables.connections);
    connector::routes_table(&mut tables);
    connector::set_runnable_outputs(&mut tables)?;
    connector::check_connections(&mut tables)?;

    Ok(tables)
}