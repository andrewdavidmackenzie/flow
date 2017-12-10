#[macro_use]
extern crate log;
use log::LogLevelFilter;
use log::SetLoggerError;

extern crate flowrlib;
extern crate flowstdlib;

use flowrlib::execution::execute;

mod runnables;
mod simple_logger;

use runnables::get_runnables;
use simple_logger::SimpleLogger;

fn init_logging() -> Result<(), SetLoggerError> {
    log::set_logger(|max_log_level| {
        max_log_level.set(LogLevelFilter::Warn);
        Box::new(SimpleLogger)
    })
}

fn main() {
    init_logging().unwrap();

    info!("'{}' version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    // TODO some standard inputs that are passed to main as arguments
    // a library function to help parse them?

    execute(get_runnables());
}