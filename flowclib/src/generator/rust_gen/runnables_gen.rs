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
use super::function;
use super::value;

const RUNNABLES_PREFIX: &'static str = "
// Flow Run-time library references
use flowrlib::runnable::Runnable;
{value_used}
{function_used}

// Rust std library references
use std::sync::{{Arc, Mutex}};\n";

const GET_RUNNABLES: &'static str = "
pub fn get_runnables() -> Vec<Arc<Mutex<Runnable>>> {{
    let mut runnables = Vec::<Arc<Mutex<Runnable>>>::with_capacity({num_runnables});\n\n";

const RUNNABLES_SUFFIX: &'static str = "
    runnables
}";

pub fn create(src_dir: &PathBuf, tables: &CompilerTables)
              -> Result<()> {
    let lib_refs = lib_refs(&tables.lib_references);
    let mut file = src_dir.clone();
    file.push("runnables.rs");
    let mut runnables_rs = File::create(&file)?;
    runnables_rs.write_all(contents(tables, &lib_refs).unwrap().as_bytes())
}

fn contents(tables: &CompilerTables, lib_refs: &Vec<String>) -> Result<String> {
    let num_runnables = &(tables.values.len() + tables.functions.len()).to_string();
    let mut vars = HashMap::new();
    if tables.values.len() > 0 {
        vars.insert("value_used".to_string(), "use flowrlib::value::Value;");
    } else {
        vars.insert("value_used".to_string(), "");
    }
    if tables.functions.len() > 0 {
        vars.insert("function_used".to_string(), "use flowrlib::function::Function;");
    } else {
        vars.insert("function_used".to_string(), "");
    }

    let mut content = strfmt(RUNNABLES_PREFIX, &vars).unwrap();

    content.push_str("\n// Library functions\n");
    for lib_ref in lib_refs {
        content.push_str(&lib_ref);
    }

    // Add "use" statements for functions referenced
    content.push_str(&usages(&tables.functions).unwrap());

    // add declaration of runnables array etc - parameterized by the number of runnables
    vars = HashMap::<String, &str>::new();
    vars.insert("num_runnables".to_string(), num_runnables );
    content.push_str(&strfmt(GET_RUNNABLES, &vars).unwrap());

    // Generate code for each of the values
    for value in &tables.values {
        let run_str = format!("    runnables.push(Arc::new(Mutex::new({})));\n",
                              value::to_code(&value));
        content.push_str(&run_str);
    }

    // Generate code for each of the functions
    for function in &tables.functions {
        let run_str = format!("    runnables.push(Arc::new(Mutex::new({})));\n",
                              function::to_code(&function));
        content.push_str(&run_str);
    }

    // return the array of runnables - No templating in this part (at the moment)
    content.push_str(RUNNABLES_SUFFIX);

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