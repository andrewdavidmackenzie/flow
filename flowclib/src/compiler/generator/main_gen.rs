use strfmt::Result;
use strfmt::strfmt;
use std::collections::HashMap;

const MAIN_TEMPLATE: &'static str = "
#[macro_use]
extern crate log;
use log::LogLevelFilter;
use log::SetLoggerError;

extern crate flowrlib;
extern crate flowstdlib;

use flowrlib::execution::execute;
use flowrlib::simple_logger;

mod runnables;

use runnables::get_runnables;
use simple_logger::SimpleLogger;

fn init_logging() -> Result<(), SetLoggerError> {{
    log::set_logger(|max_log_level| {{
        max_log_level.set(LogLevelFilter::Info);
        Box::new(SimpleLogger)
    }})
}}

fn main() {{
    init_logging().unwrap();
    info!(\"'{{}}' version '{{}}'\", env!(\"CARGO_PKG_NAME\"), env!(\"CARGO_PKG_VERSION\"));
    execute(get_runnables());
}}
";

pub fn main_file_contents(vars: &HashMap<String, &str>) -> Result<String>{
    strfmt(MAIN_TEMPLATE, &vars)
}
