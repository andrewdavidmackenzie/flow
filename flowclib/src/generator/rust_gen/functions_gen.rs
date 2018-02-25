use std::path::PathBuf;
use std::io::{Error, ErrorKind};
use std::io::Result;
use std::fs;

use compiler::compile::CompilerTables;

pub fn copy(src_dir: &PathBuf, tables: &CompilerTables) -> Result<()> {
    // Find all the functions that are not loaded from libraries
    for function in &tables.functions {
        if function.lib_reference.is_none() {
            let mut source = function.source_url.to_file_path()
                .map_err(|_e| Error::new(ErrorKind::InvalidData,"Could not convert to file path"))?;
            source.set_extension("rs");
            let mut dest = src_dir.clone();
            dest.push(source.file_name().unwrap());
            fs::copy(source, dest)?;
        }
    }
    Ok(())
}