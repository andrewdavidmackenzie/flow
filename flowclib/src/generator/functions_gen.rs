use std::path::PathBuf;
use compiler::compile::CompilerTables;

pub fn copy(_src_dir: &PathBuf, tables: &CompilerTables) {
    // Find all the functions that are not loaded from libraries
    for function in &tables.functions {
        if function.lib_reference.is_none() {
            println!("Function = {:?}", function.lib_reference);
        }
    }
}