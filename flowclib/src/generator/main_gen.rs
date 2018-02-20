use strfmt::Result as FmtResult;
use strfmt::strfmt;
use std::io::Result;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

const MAIN_TEMPLATE: &'static str = "
#[macro_use]
extern crate log;

extern crate flowrlib;

use flowrlib::execution::execute;

mod runnables;

use runnables::get_runnables;

extern crate simplog;
use simplog::simplog::SimpleLogger;

fn main() {{
    SimpleLogger::init(None);
    info!(\"'{{}}' version '{{}}'\", env!(\"CARGO_PKG_NAME\"), env!(\"CARGO_PKG_VERSION\"));
    execute(get_runnables());
}}
";

pub fn create(src_dir: &PathBuf, vars: &HashMap<String, &str>, libs: &HashSet<String>) -> Result<()> {
    let mut file = src_dir.clone();
    file.push("main.rs");
    let mut main_rs = File::create(&file)?;
    main_rs.write_all(contents(&vars, libs).unwrap().as_bytes())
}

fn contents(vars: &HashMap<String, &str>, libs: &HashSet<String>) -> FmtResult<String> {
    let mut content = String::new();
    content.push_str(&crates(&libs));
    content.push_str(&strfmt(MAIN_TEMPLATE, &vars).unwrap());
    Ok(content)
}

fn crates(libs: &HashSet<String>) -> String {
    let mut crates_string = String::new();
    for lib in libs {
        crates_string.push_str(&format!("extern crate {};\n", lib));
    }
    crates_string
}
