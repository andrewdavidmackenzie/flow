//! `flowdb` is a standalone debugger client for flow programs.

use clap::{Arg, Command};

/// Parse command line arguments
fn parse_args() -> clap::ArgMatches {
    Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about("A debugger client for flow programs")
        .arg(
            Arg::new("verbosity")
                .short('v')
                .long("verbosity")
                .num_args(1)
                .value_name("LEVEL")
                .help("Set verbosity level (error, warn, info, debug, trace)"),
        )
        .get_matches()
}

fn main() {
    let matches = parse_args();

    let default_level = "error";
    let verbosity = matches
        .get_one::<String>("verbosity")
        .map_or(default_level, String::as_str);

    std::env::set_var("RUST_LOG", verbosity);
    env_logger::init();

    println!("flowdb {}", env!("CARGO_PKG_VERSION"));
}
