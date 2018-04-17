use clap::{App, Arg, ArgMatches, AppSettings};
use simplog::simplog::SimpleLogger;
use std::env;

pub fn start() {
    let matches = get_matches();
    // Set anvironment variable with the args
    // this will not be unique, but it will be used very soon and removed
    if let Some(flow_args) = matches.values_of("flow_args") {
        let args: Vec<&str> = flow_args.collect();
        env::set_var("FLOW_ARGS", args.join(" "));
    }
    SimpleLogger::init(matches.value_of("log"));
    info!("'{}' version '{}'", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
}

fn get_matches<'a>() -> ArgMatches<'a> {
    App::new(env!("CARGO_PKG_NAME"))
        .setting(AppSettings::TrailingVarArg)
        .arg(Arg::with_name("log")
            .short("l")
            .long("log")
            .takes_value(true)
            .value_name("LOG_LEVEL")
            .help("Set log level for output (trace, debug, info, warn, error (default))"))
        .arg(Arg::with_name("flow_args")
            .multiple(true))
        .get_matches()
}
