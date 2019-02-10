use model::flow::Flow;
use super::gatherer;
use super::connector;
use generator::generate::GenerationTables;
use super::checker;

/// Take a hierarchical flow definition in memory and compile it, generating code that implements
/// the flow, including links to the flowrlib runtime library and library functions used in the
/// flowstdlib standard library. It takes an optional bool dump option to dump to standard output
/// some of the intermediate values and operations during the compilation process.
pub fn compile(flow: &Flow) -> Result<GenerationTables, String> {
    let mut tables = GenerationTables::new();

    gatherer::gather_runnables_and_connections(flow, &mut tables);
    gatherer::index_runnables(&mut tables.runnables);
    tables.collapsed_connections = connector::collapse_connections(&tables.connections);
    connector::routes_table(&mut tables);
    connector::set_runnable_outputs(&mut tables)?;
    connector::check_connections(&mut tables)?;
    checker::check_process_inputs(&mut tables)?;

    Ok(tables)
}