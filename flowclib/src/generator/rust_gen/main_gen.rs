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
extern crate log;

extern crate flowrlib;
use flowrlib::execution::execute;
use std::process::exit;

mod runnables;
use runnables::get_runnables;
";

const MAIN_SUFFIX: &'static str = "
extern crate simplog;
#[macro_use]
extern crate serde_json;
extern crate clap;
use clap::{{App, Arg, ArgMatches}};
use simplog::simplog::SimpleLogger;

fn main() {{
    let matches = get_matches();
    SimpleLogger::init(matches.value_of(\"log\"));
    info!(\"'{{}}' version '{{}}'\", env!(\"CARGO_PKG_NAME\"), env!(\"CARGO_PKG_VERSION\"));
    execute(get_runnables());
    exit(0);
}}

fn get_matches<'a>() -> ArgMatches<'a> {{
    App::new(env!(\"CARGO_PKG_NAME\"))
        .arg(Arg::with_name(\"log\")
            .short(\"l\")
            .long(\"log\")
            .takes_value(true)
            .value_name(\"LOG_LEVEL\")
            .help(\"Set log level for output (trace, debug, info, warn, error (default))\"))
        .get_matches()
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
        crates_string.push_str(&format!("extern crate {};\n", lib));
    }
    crates_string
}

// add local function files as modules, using the file_stem portion of the file name of the function
// e.g. "mod reverse;"
fn modules(runnables: &Vec<Box<Runnable>>) -> Result<String> {
    let mut modules_string = String::new();

    // Find all the functions that are not loaded from libraries
    for runnable in runnables {
        if let Some(source_url) = runnable.source_url() {
            let source = source_url.to_file_path()
                .map_err(|_e| Error::new(ErrorKind::InvalidData, "Could not convert to file path"))?;
            let module = source.file_stem().unwrap();
            modules_string.push_str(&format!("mod {};\n", module.to_str().unwrap()));
        }
    }

    Ok(modules_string)
}
