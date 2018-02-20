use strfmt::Result as FmtResult;
use std::io::Result;
use strfmt::strfmt;
use std::collections::HashMap;
use flowrlib::runnable::Runnable;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use std::collections::HashSet;

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

pub fn create(src_dir: &PathBuf, vars: &HashMap<String, &str>,
                    runnables: &Vec<Box<Runnable>>, lib_references: &HashSet<String>)
                    -> Result<()> {
    let lib_refs = lib_refs(lib_references);
    let mut file = src_dir.clone();
    file.push("runnables.rs");
    let mut runnables_rs = File::create(&file)?;
    runnables_rs.write_all(contents(vars, &lib_refs, runnables)
        .unwrap().as_bytes())
}

fn contents(vars: &HashMap<String, &str>,
                lib_references: &Vec<String>,
                runnables: &Vec<Box<Runnable>>) -> FmtResult<String> {
    let mut content = strfmt(RUNNABLES_PREFIX, &vars)?;

    content.push_str("\n// Library functions\n");
    for lib_ref in lib_references {
        content.push_str(&lib_ref);
    }

    let get_runnables = strfmt(GET_RUNNABLES, &vars)?;
    content.push_str(&get_runnables);

    for runnable in runnables {
        let run_str = format!("    runnables.push(Arc::new(Mutex::new({})));\n", runnable.to_code());
        content.push_str(&run_str);
    }

    let suffix = strfmt(RUNNABLES_SUFFIX, &vars)?;
    content.push_str(&suffix);
    Ok(content)
}

fn lib_refs(libs_references: &HashSet<String>) -> Vec<String> {
    let mut lib_refs: Vec<String> = Vec::new();
    for lib_ref in libs_references {
        let lib_use = str::replace(&lib_ref, "/", "::");
        lib_refs.push(format!("use {};\n", lib_use));
    }

    lib_refs
}