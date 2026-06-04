//! `flowdb` is a standalone debugger client for flow programs.
//!
//! It connects to a flow runner's debug server (started with `flowrcli --debugger`)
//! and provides an interactive REPL for setting breakpoints, stepping through
//! execution, and inspecting runtime state.

use std::process::exit;

use clap::{Arg, Command};
use log::{error, info};

use flowcore::discovery::discover_service;
use flowcore::services::DEBUG_SERVICE_NAME;
use flowdblib::client_connection::ClientConnection;
use flowdblib::debug_client::DebugClient;

/// Parse command line arguments
fn parse_args() -> clap::ArgMatches {
    Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about("A debugger client for flow programs")
        .arg(
            Arg::new("address")
                .short('a')
                .long("address")
                .num_args(1)
                .value_name("HOST:PORT")
                .help("Connect directly to a debug server at HOST:PORT"),
        )
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

    let server_address = if let Some(address) = matches.get_one::<String>("address") {
        address.clone()
    } else {
        println!("Discovering debug server via mDNS...");
        match discover_service(DEBUG_SERVICE_NAME) {
            Ok(address) => {
                println!("Found debug server at: {address}");
                address
            }
            Err(e) => {
                error!("Could not discover debug server: {e}");
                eprintln!(
                    "Could not discover debug server. Is flowrcli running with --debugger?\n\
                    You can also connect directly with: flowdb --address HOST:PORT"
                );
                exit(1);
            }
        }
    };

    info!("Connecting to debug server at: {server_address}");

    let connection = match ClientConnection::new(&server_address) {
        Ok(conn) => conn,
        Err(e) => {
            error!("Could not connect to debug server: {e}");
            eprintln!("Could not connect to debug server at {server_address}: {e}");
            exit(1);
        }
    };

    let client = match DebugClient::new(connection) {
        Ok(client) => client,
        Err(e) => {
            error!("Could not create debug client: {e}");
            eprintln!("Could not create debug client: {e}");
            exit(1);
        }
    };

    println!(
        "flowdb {} - connected to {server_address}",
        env!("CARGO_PKG_VERSION")
    );
    println!("Type 'h' or 'help' for available commands.\n");

    client.debug_client_loop();
}
