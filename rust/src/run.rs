extern crate flowlib;
use flowlib::info;

/*
use flow::description;
use flow::execution;
*/

#[macro_use]
extern crate log;
extern crate log4rs;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    log4rs::init_file("log.toml", Default::default()).unwrap();
    info!("Logging started using 'log4rs', see log.toml for configuration details");
	info!("Running flow: ");
    println!("Flow 'run' version: {}", VERSION);
	println!("Flow Library version: {}", info::version());

    /*

    validate model (see check)

    load flow definition from file specified in arguments
        - load any referenced to included flows also

    construct overall list of functions

    construct list of connections

    construct initial list of all functions able to produce output
        - start from external sources at level 0

    do
        - identify all functions which receive input from active sources
        - execute all those functions
        - functions producing output added to list of active sources
    while functions pending input

     */
}
