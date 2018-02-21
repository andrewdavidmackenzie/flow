use std::io::Result;
use strfmt::strfmt;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use std::collections::HashSet;
use std::io::{Error, ErrorKind};
use model::function::Function;
use compiler::compile::CompilerTables;

const RUNNABLES_PREFIX: &'static str = "
// Flow Run-time library references
use flowrlib::runnable::Runnable;
use flowrlib::value::Value;
use flowrlib::function::Function;

// Rust std library references
use std::sync::{{Arc, Mutex}};\n";

const GET_RUNNABLES: &'static str = "
pub fn get_runnables() -> Vec<Arc<Mutex<Runnable>>> {{
    let mut runnables = Vec::<Arc<Mutex<Runnable>>>::with_capacity(2);\n\n";

const RUNNABLES_SUFFIX: &'static str = "
    runnables
}}";

pub fn create(src_dir: &PathBuf, vars: &HashMap<String, &str>, tables: &CompilerTables)
              -> Result<()> {
    let lib_refs = lib_refs(&tables.lib_references);
    let mut file = src_dir.clone();
    file.push("runnables.rs");
    let mut runnables_rs = File::create(&file)?;
    runnables_rs.write_all(contents(vars, tables, &lib_refs).unwrap().as_bytes())
}

fn contents(vars: &HashMap<String, &str>, tables: &CompilerTables, lib_refs: &Vec<String>) -> Result<String> {
    let mut content = strfmt(RUNNABLES_PREFIX, &vars).unwrap();

    content.push_str("\n// Library functions\n");
    for lib_ref in lib_refs {
        content.push_str(&lib_ref);
    }

    content.push_str(&usages(&tables.functions).unwrap());
    content.push_str(&strfmt(GET_RUNNABLES, &vars).unwrap());

    for runnable in &tables.runnables {
        let run_str = format!("    runnables.push(Arc::new(Mutex::new({})));\n", runnable.to_code());
        content.push_str(&run_str);
    }

    let suffix = strfmt(RUNNABLES_SUFFIX, &vars).unwrap();
    content.push_str(&suffix);
    Ok(content)
}

// add use clauses for local functions filename::Functionname
// "use reverse::Reverse;"
fn usages(functions: &Vec<Function>) -> Result<String> {
    let mut usages_string = String::new();

    // Find all the functions that are not loaded from libraries
    for function in functions {
        if function.lib_reference.is_none() {
            let mut source = function.source_url.to_file_path()
                .map_err(|_e| Error::new(ErrorKind::InvalidData, "Could not convert to file path"))?;
            let usage = source.file_stem().unwrap();
            usages_string.push_str(&format!("use {}::{};\n",
                                            usage.to_str().unwrap(), function.name));
        }
    }

    Ok(usages_string)
}

fn lib_refs(libs_references: &HashSet<String>) -> Vec<String> {
    let mut lib_refs: Vec<String> = Vec::new();
    for lib_ref in libs_references {
        let lib_use = str::replace(&lib_ref, "/", "::");
        lib_refs.push(format!("use {};\n", lib_use));
    }

    lib_refs
}