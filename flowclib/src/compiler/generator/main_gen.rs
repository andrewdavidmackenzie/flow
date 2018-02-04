use strfmt::Result;
use strfmt::strfmt;
use std::collections::HashMap;

const MAIN_TEMPLATE: &'static str = "
#[macro_use]
extern crate log;

extern crate flowrlib;
extern crate flowstdlib;

use flowrlib::execution::execute;

mod runnables;

use runnables::get_runnables;

extern crate simplog;
use simplog::simplog::SimpleLogger;

fn main() {{
    SimpleLogger::init(None);
    init_logging();
    info!(\"'{{}}' version '{{}}'\", env!(\"CARGO_PKG_NAME\"), env!(\"CARGO_PKG_VERSION\"));
    execute(get_runnables());
}}
";

pub fn main_file_contents(vars: &HashMap<String, &str>) -> Result<String>{
    strfmt(MAIN_TEMPLATE, &vars)
}
