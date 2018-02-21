use strfmt::strfmt;
use std::io::Result;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use std::io::{Error, ErrorKind};

use compiler::compile::CompilerTables;
use model::function::Function;

const MAIN_PREFIX: &'static str = "
#[macro_use]
extern crate log;

extern crate flowrlib;
use flowrlib::execution::execute;

mod runnables;
use runnables::get_runnables;
";

const MAIN_SUFFIX: &'static str = "
extern crate simplog;
use simplog::simplog::SimpleLogger;

fn main() {{
    SimpleLogger::init(None);
    info!(\"'{{}}' version '{{}}'\", env!(\"CARGO_PKG_NAME\"), env!(\"CARGO_PKG_VERSION\"));
    execute(get_runnables());
}}
";
pub fn create(src_dir: &PathBuf, vars: &mut HashMap<String, &str>, tables: &CompilerTables) -> Result<()> {
    let mut file = src_dir.clone();
    file.push("main.rs");
    let mut main_rs = File::create(&file)?;

    let mut content = String::new();
    content.push_str(&crates(&tables.libs));
    content.push_str(MAIN_PREFIX);
    content.push_str(&modules(&tables.functions)?);
    content.push_str(&strfmt(MAIN_SUFFIX, &vars).unwrap());

    main_rs.write_all(content.as_bytes())
}

fn crates(libs: &HashSet<String>) -> String {
    let mut crates_string = String::new();
    for lib in libs {
        crates_string.push_str(&format!("extern crate {};\n", lib));
    }
    crates_string
}

// add local function files as modules, using the file_stem portion of the file name of the function
// e.g. "mod reverse;"
fn modules(functions: &Vec<Function>) -> Result<String> {
    let mut modules_string = String::new();

    // Find all the functions that are not loaded from libraries
    for function in functions {
        if function.lib_reference.is_none() {
            let mut source = function.source_url.to_file_path()
                .map_err(|_e| Error::new(ErrorKind::InvalidData, "Could not convert to file path"))?;
            let module = source.file_stem().unwrap();
            modules_string.push_str(&format!("mod {};\n", module.to_str().unwrap()));
        }
    }

    Ok(modules_string)
}
