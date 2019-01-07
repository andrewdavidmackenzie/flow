use strfmt::strfmt;
use std::io::Result;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use std::io::{Error, ErrorKind};
use generator::code_gen::CodeGenTables;
use model::runnable::Runnable;

const MAIN_PREFIX: &'static str = "
#[macro_use]
extern crate serde_json;
extern crate flowrlib;
use flowrlib::startup::start;
use flowrlib::execution::execute;
use std::process::exit;

mod runnables;
use runnables::get_runnables;
";

const MAIN_SUFFIX: &'static str = "
fn main() {{
    start();
    execute(get_runnables());
    exit(0);
}}
";

pub fn create(src_dir: &PathBuf, vars: &mut HashMap<String, &str>, tables: &CodeGenTables) -> Result<()> {
    let mut file = src_dir.clone();
    file.push("main.rs");
    let mut main_rs = File::create(&file)?;

    let mut content = String::new();
    content.push_str(&crates(&tables.libs));
    content.push_str(MAIN_PREFIX);
    content.push_str(&modules(&tables.runnables)?);
    content.push_str(&strfmt(MAIN_SUFFIX, &vars).unwrap());

    main_rs.write_all(content.as_bytes())
}

fn crates(libs: &HashSet<String>) -> String {
    let mut crates_string = String::new();
    for lib in libs {
        // We are already importing flowrlib - so avoid a double import when some of it's
        // library functions are used by the flow being generated
        if lib != "flowrlib" {
            crates_string.push_str(&format!("extern crate {};\n", lib));
        }
    }
    crates_string
}

// add local function files as modules, using the file_stem portion of the file name of the function
// e.g. "mod reverse;"
fn modules(runnables: &Vec<Box<Runnable>>) -> Result<String> {
    let mut modules_string = String::new();
    let mut modules_declared = HashSet::new();

    // Find all the functions that are not loaded from libraries and add 'mod' declarations for them in main.rs
    for runnable in runnables {
        if let Some(source_url) = runnable.source_url() {
            let source = source_url.to_file_path()
                .map_err(|_e| Error::new(ErrorKind::InvalidData, "Could not convert to file path"))?;
            let module = source.file_stem().unwrap();
            let module_name = module.to_str().unwrap().to_string();
            // Don't add the same module twice
            if modules_declared.insert(module_name.clone()) {
                modules_string.push_str(&format!("mod {};\n", module_name));
            }
        }
    }

    Ok(modules_string)
}
