use generator::code_gen::CodeGenTables;

/*
    Drop the following combinations, with warnings:
    - values that don't have connections from them.
    - values that have only outputs and are not initialized.
    - functions that don't have connections from at least one output.
    - functions that don't have connections to all their inputs.
*/
// TODO implement this
pub fn prune_tables(_tables: &mut CodeGenTables) {}