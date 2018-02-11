use strfmt::Result;
use strfmt::strfmt;
use std::collections::HashMap;

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

pub fn contents(vars: &HashMap<String, &str>, external_crates: Vec<&str>) -> Result<String>{
    let mut content = String::new();

    for external_crate in external_crates {
        content.push_str(external_crate);
    }

    content.push_str(&strfmt(MAIN_TEMPLATE, &vars).unwrap());

    Ok(content)
}
