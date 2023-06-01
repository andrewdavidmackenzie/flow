#![deny(missing_docs)]
#![warn(clippy::unwrap_used)]
//! `flowr` is a command line flow runner for running `flow` programs.
//!
//! It reads a compiled [FlowManifest][flowcore::model::flow_manifest::FlowManifest] produced by a
//! flow compiler, such as `flowc`, that describes the graph of communicating functions that
//! constitute the flow program.
//!
//! Use `flowr --help` or `flowr -h` at the command line to see the command line options
//!
//! The [cli] module implements a set of `context functions`, adapted to Terminal IO and local
//! File System, that allow the flow program to interact with the environment where it is being run.
//!
//! Depending on the command line options supplied `flowr` executes the
//! [Coordinator][flowrlib::coordinator::Coordinator] of flow execution in a background thread,
//! or the [cli::cli_client] in the main thread (where the interaction with STDIO and
//! File System happens) or both. They communicate via network messages using the
//! [SubmissionHandler][flowrlib::submission_handler::SubmissionHandler] to submit flows for execution,
//! and interchanging [ClientMessages][crate::cli::coordinator_message::ClientMessage]
//! and [CoordinatorMessages][crate::cli::coordinator_message::CoordinatorMessage] for execution of context
//! interaction in the client, as requested by functions running in the coordinator's
//! [Executors][flowrlib::executor::Executor]

use core::str::FromStr;
use std::{env, thread};
use std::path::PathBuf;
use std::process::exit;
use std::sync::{Arc, Mutex};

use clap::{Arg, ArgMatches, Command};
use cli::cli_client::CliRuntimeClient;
#[cfg(feature = "debugger")]
use cli::cli_debug_client::CliDebugClient;
#[cfg(feature = "debugger")]
use cli::cli_debug_handler::CliDebugHandler;
use cli::cli_submission_handler::CLISubmissionHandler;
#[cfg(feature = "debugger")]
use cli::connections::ClientConnection;
use cli::connections::CoordinatorConnection;
use cli::coordinator_message::ClientMessage;
#[cfg(feature = "debugger")]
use cli::debug_message::DebugServerMessage;
#[cfg(feature = "debugger")]
use cli::debug_message::DebugServerMessage::{BlockBreakpoint, DataBreakpoint, ExecutionEnded, ExecutionStarted,
                                             ExitingDebugger, JobCompleted, JobError, Panic, PriorToSendingJob,
                                             Resetting, WaitingForCommand};
use env_logger::Builder;
use flowcore::errors::*;
use flowcore::meta_provider::MetaProvider;
use flowcore::model::flow_manifest::FlowManifest;
use flowcore::model::submission::Submission;
use flowcore::provider::Provider;
use flowcore::url_helper::url_from_string;
use flowrlib::coordinator::Coordinator;
use flowrlib::dispatcher::Dispatcher;
use flowrlib::executor::Executor;
use flowrlib::info as flowrlib_info;
use flowrlib::services::{CONTROL_SERVICE_NAME, JOB_QUEUES_DISCOVERY_PORT, JOB_SERVICE_NAME,
                         RESULTS_JOB_SERVICE_NAME};
use log::{info, LevelFilter, trace, warn};
use portpicker::pick_unused_port;
use simpath::Simpath;
use url::Url;

use crate::cli::connections::{COORDINATOR_SERVICE_NAME, DEBUG_SERVICE_NAME,
                              discover_service, enable_service_discovery};

/// Include the module that implements the context functions
mod context;

/// provides the `context functions` for interacting with the execution environment from a flow,
/// plus client-[Coordinator][flowrlib::coordinator::Coordinator] implementations of
/// [flowrlib::submission_handler] for executing them on different threads
/// from the [Coordinator][flowrlib::coordinator::Coordinator]
mod cli;

/// provides [Error][errors::Error] that other modules in this crate will `use crate::errors::*;` to get
/// access to everything `error_chain` creates.
mod errors;

/// Main for flowr binary - call `run()` and print any error that results or exit silently if OK
fn main() {
    match run() {
        Err(ref e) => {
            eprintln!("{e}");
            for e in e.iter().skip(1) {
                eprintln!("caused by: {e}");
            }

            // The backtrace is generated if env var `RUST_BACKTRACE` is set to `1` or `full`
            if let Some(backtrace) = e.backtrace() {
                eprintln!("backtrace: {backtrace:?}");
            }

            exit(1);
        }
        Ok(_) => exit(0),
    }
}

/// For the lib provider, libraries maybe installed in multiple places in the file system.
/// In order to find the content, a FLOW_LIB_PATH environment variable can be configured with a
/// list of directories in which to look for the library in question.
fn get_lib_search_path(search_path_additions: &[String]) -> Result<Simpath> {
    let mut lib_search_path = Simpath::new_with_separator("FLOW_LIB_PATH", ',');

    for additions in search_path_additions {
        lib_search_path.add(additions);
        info!("'{}' added to the Library Search Path", additions);
    }

    if lib_search_path.is_empty() {
        warn!("'$FLOW_LIB_PATH' not set and no LIB_DIRS supplied. Libraries may not be found.");
    }

    Ok(lib_search_path)
}

/// Run `flowr`. After setting up logging and parsing the command line arguments invoke `flowrlib`
/// and return any errors found.
fn run() -> Result<()> {
    let matches = get_matches();

    let default = String::from("error");
    let verbosity = matches.get_one::<String>("verbosity").unwrap_or(&default);
    let level = LevelFilter::from_str(verbosity).unwrap_or(LevelFilter::Error);
    let mut builder = Builder::from_default_env();
    builder.filter_level(level).init();

    info!(
        "'{}' version {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    info!("'flowrlib' version {}", flowrlib_info::version());

    #[cfg(feature = "debugger")]
    let debug_this_flow = matches.get_flag("debugger");
    let native_flowstdlib = matches.get_flag("native");
    let lib_dirs = if matches.contains_id("lib_dir") {
        matches
            .get_many::<String>("lib_dir")
            .chain_err(|| "Could not get the list of 'LIB_DIR' options specified")?
            .map(|s| s.to_string())
            .collect()
    } else {
        vec![]
    };
    let lib_search_path = get_lib_search_path(&lib_dirs)?;
    let num_threads = num_threads(&matches);

    if let Some(discovery_port) = matches.get_one::<u16>("client") {
        client_only(
            &matches,
            lib_search_path,
            #[cfg(feature = "debugger")] debug_this_flow,
            discovery_port,
        )?;
    } else if matches.get_flag("server") {
        coordinator_only(
            num_threads,
            lib_search_path,
            native_flowstdlib,
        )?;
    } else {
        client_and_coordinator(
            num_threads,
            lib_search_path,
            native_flowstdlib,
            &matches,
            #[cfg(feature = "debugger")] debug_this_flow,
        )?;
    };

    Ok(())
}

/// Start just a [Coordinator][flowrlib::coordinator::Coordinator] in the calling thread.
fn coordinator_only(
                num_threads: usize,
                lib_search_path: Simpath,
                native_flowstdlib: bool,
                ) -> Result<()> {
    let coordinator_port = pick_unused_port().chain_err(|| "No ports free")?;
    let coordinator_connection = CoordinatorConnection::new(COORDINATOR_SERVICE_NAME,
                                                            coordinator_port)?;
    let discovery_port = pick_unused_port().chain_err(|| "No ports free")?;
    enable_service_discovery(discovery_port, COORDINATOR_SERVICE_NAME,
                             coordinator_port)?;

    #[cfg(feature = "debugger")]
    let debug_port = pick_unused_port().chain_err(|| "No ports free")?;
    #[cfg(feature = "debugger")]
    let debug_server_connection = CoordinatorConnection::new(DEBUG_SERVICE_NAME,
                                                             debug_port)?;
    #[cfg(feature = "debugger")]
    enable_service_discovery(discovery_port, DEBUG_SERVICE_NAME,debug_port)?;

    println!("{discovery_port}");

    info!("Starting coordinator in main thread");
    coordinator(
        num_threads,
        lib_search_path,
        native_flowstdlib,
        coordinator_connection,
        #[cfg(feature = "debugger")] debug_server_connection,
        true,
    )?;

    info!("'flowr' coordinator has exited");

    Ok(())
}

/// Start a [Coordinator][flowrlib::coordinator::Coordinator] in a background thread,
/// then start a client in the calling thread
fn client_and_coordinator(
    num_threads: usize,
    lib_search_path: Simpath,
    native_flowstdlib: bool,
    matches: &ArgMatches,
    #[cfg(feature = "debugger")]
    debug_this_flow: bool,
) -> Result<()> {
    let runtime_port = pick_unused_port().chain_err(|| "No ports free")?;
    let coordinator_connection = CoordinatorConnection::new(COORDINATOR_SERVICE_NAME,
                                                            runtime_port)?;

    let discovery_port = pick_unused_port().chain_err(|| "No ports free")?;
    enable_service_discovery(discovery_port, COORDINATOR_SERVICE_NAME, runtime_port)?;

    #[cfg(feature = "debugger")]
    let debug_port = pick_unused_port().chain_err(|| "No ports free")?;
    #[cfg(feature = "debugger")]
    let debug_connection = CoordinatorConnection::new(DEBUG_SERVICE_NAME,
                                                      debug_port)?;
    enable_service_discovery(discovery_port, DEBUG_SERVICE_NAME, debug_port)?;

    let coordinator_lib_search_path = lib_search_path.clone();

    info!("Starting coordinator in background thread");
    thread::spawn(move || {
        let _ = coordinator(
            num_threads,
            coordinator_lib_search_path,
            native_flowstdlib,
            coordinator_connection,
            #[cfg(feature = "debugger")] debug_connection,
            false,
        );
    });

    let coordinator_address = discover_service(discovery_port, COORDINATOR_SERVICE_NAME)?;

    let runtime_client_connection = ClientConnection::new(&coordinator_address)?;

    client(
        matches,
        lib_search_path,
        runtime_client_connection,
        #[cfg(feature = "debugger")] debug_this_flow,
        #[cfg(feature = "debugger")] discovery_port,
    )
}

/// Create a new `Coordinator`, pre-load any libraries in native format that we want to have before
/// loading a flow and it's library references, then enter the `submission_loop()` accepting and
/// executing flows submitted for execution, executing each one using the `Coordinator`
fn coordinator(
    num_threads: usize,
    lib_search_path: Simpath,
    native_flowstdlib: bool,
    coordinator_connection: CoordinatorConnection,
    #[cfg(feature = "debugger")] debug_connection: CoordinatorConnection,
    loop_forever: bool,
) -> Result<()> {
    let connection = Arc::new(Mutex::new(coordinator_connection));

    #[cfg(feature = "debugger")]
    let mut debug_server = CliDebugHandler { debug_server_connection: debug_connection };

    let provider = Arc::new(MetaProvider::new(lib_search_path,
                                         PathBuf::from("/"))) as Arc<dyn Provider>;

    let ports = get_four_ports()?;
    trace!("Announcing three job queues and a control socket on ports: {ports:?}");
    let job_queues = get_bind_addresses(ports);
    let dispatcher = Dispatcher::new(job_queues)?;
    enable_service_discovery(JOB_QUEUES_DISCOVERY_PORT, JOB_SERVICE_NAME, ports.0)?;
    enable_service_discovery(JOB_QUEUES_DISCOVERY_PORT, RESULTS_JOB_SERVICE_NAME, ports.2)?;
    enable_service_discovery(JOB_QUEUES_DISCOVERY_PORT, CONTROL_SERVICE_NAME, ports.3)?;

    let (job_source_name, context_job_source_name, results_sink, control_socket) =
        get_connect_addresses(ports);

    let mut executor = Executor::new()?;
    // if the command line options request loading native implementation of available native libs
    // if not, the native implementation is not loaded and later when a flow is loaded it's library
    // references will be resolved and those libraries (WASM implementations) will be loaded at runtime
    if native_flowstdlib {
        executor.add_lib(
            flowstdlib::manifest::get_manifest()
                .chain_err(|| "Could not get 'native' flowstdlib manifest")?,
            Url::parse("memory://")? // Statically linked library has no resolved Url
        )?;
    }
    executor.start(provider.clone(), num_threads,
                   &job_source_name,
                   &results_sink,
                   &control_socket,
    );

    let mut context_executor = Executor::new()?;
    context_executor.add_lib(
        context::get_manifest(connection.clone())?,
        Url::parse("memory://")? // Statically linked library has no resolved Url
    )?;
    context_executor.start(provider, 1,
                           &context_job_source_name,
                           &results_sink,
                            &control_socket,
    );

    let mut submitter = CLISubmissionHandler::new(connection);

    let mut coordinator = Coordinator::new(
        dispatcher,
        &mut submitter,
        #[cfg(feature = "debugger")] &mut debug_server
    )?;

    coordinator.submission_loop(loop_forever)?;

    Ok(())
}

/// Start only a client in the calling thread. Discover the remote Coordinator using service discovery
fn client_only(
    matches: &ArgMatches,
    lib_search_path: Simpath,
    #[cfg(feature = "debugger")] debug_this_flow: bool,
    discovery_port: &u16,
) -> Result<()> {
    let coordinator_address = discover_service(*discovery_port, COORDINATOR_SERVICE_NAME)?;
    let client_connection = ClientConnection::new(&coordinator_address)?;

    client(
        matches,
        lib_search_path,
        client_connection,
        #[cfg(feature = "debugger")] debug_this_flow,
        #[cfg(feature = "debugger")] *discovery_port,
    )
}

/// Start the clients that talks to the coordinator
#[cfg(feature = "debugger")]
fn client(
    matches: &ArgMatches,
    lib_search_path: Simpath,
    client_connection: ClientConnection,
    #[cfg(feature = "debugger")] debug_this_flow: bool,
    #[cfg(feature = "debugger")] discovery_port: u16,
) -> Result<()> {
    // keep an Arc Mutex protected set of override args that debug client can override
    let override_args = Arc::new(Mutex::new(Vec::<String>::new()));

    let flow_manifest_url = parse_flow_url(matches)?;
    let provider = MetaProvider::new(lib_search_path, PathBuf::from("/"));
    let (flow_manifest, _) = FlowManifest::load(&provider, &flow_manifest_url)?;

    let flow_args = get_flow_args(matches, &flow_manifest_url);
    let parallel_jobs_limit = matches.get_one::<usize>("jobs")
        .map(|i| i.to_owned());
    let submission = Submission::new(
        flow_manifest,
        parallel_jobs_limit,
        None, // No timeout waiting for job results
        #[cfg(feature = "debugger")] debug_this_flow,
    );

    trace!("Creating CliRuntimeClient");
    let client = CliRuntimeClient::new(
        flow_args,
        override_args.clone(),
        #[cfg(feature = "metrics")] matches.get_flag("metrics"),
    );

    #[cfg(feature = "debugger")]
    if debug_this_flow {
        let debug_server_address = discover_service(discovery_port,
                                                    DEBUG_SERVICE_NAME)?;
        let debug_client_connection = ClientConnection::new(&debug_server_address)?;
        let debug_client = CliDebugClient::new(debug_client_connection,
            override_args);
        let _ = thread::spawn(move || {
            debug_client.debug_client_loop();
        });
    }

    info!("Client sending submission to coordinator");
    client_connection.send(ClientMessage::ClientSubmission(submission))?;

    trace!("Entering client event loop");
    client.event_loop(client_connection)
}

/// Determine the number of threads to use to execute flows
/// - default (if value is not provided on the command line)of the number of cores
fn num_threads(matches: &ArgMatches) -> usize {
    match matches.get_one::<usize>("threads") {
        Some(num_threads) => *num_threads,
        None => thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
    }
}

/// Parse the command line arguments using clap
fn get_matches() -> ArgMatches {
    let app = Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"));

    #[cfg(feature = "debugger")]
        let app = app.arg(
        Arg::new("debugger")
            .short('d')
            .long("debugger")
            .action(clap::ArgAction::SetTrue)
            .help("Enable the debugger when running a flow"),
    );

    #[cfg(feature = "metrics")]
        let app = app.arg(
        Arg::new("metrics")
            .short('m')
            .long("metrics")
            .action(clap::ArgAction::SetTrue)
            .help("Calculate metrics during flow execution and print them out when done"),
    );

    #[cfg(not(feature = "wasm"))]
        let app = app.arg(
        Arg::new("native")
            .short('n')
            .long("native")
            .action(clap::ArgAction::SetTrue)
            .conflicts_with("client")
            .help("Link with native (not WASM) version of flowstdlib"),
    );

    let app = app
        .arg(Arg::new("server")
             .short('s')
             .long("server")
             .action(clap::ArgAction::SetTrue)
             .conflicts_with("client")
             .help("Launch only a Coordinator (no client)"),
        )
        .arg(Arg::new("client")
             .short('c')
             .long("client")
             .number_of_values(1)
             .value_parser(clap::value_parser!(u16))
             .conflicts_with("server")
             .help("Launch only a client (no coordinator) to connect to a remote coordinator"),
        )
        .arg(Arg::new("jobs")
            .short('j')
            .long("jobs")
            .number_of_values(1)
            .value_parser(clap::value_parser!(usize))
            .value_name("MAX_JOBS")
            .help("Set maximum number of jobs that can be running in parallel)"))
        .arg(Arg::new("lib_dir")
            .short('L')
            .long("libdir")
            .num_args(0..)
            .number_of_values(1)
            .value_name("LIB_DIR|BASE_URL")
            .help("Add a directory or base Url to the Library Search path"))
        .arg(Arg::new("threads")
            .short('t')
            .long("threads")
            .number_of_values(1)
            .value_parser(clap::value_parser!(usize))
            .value_name("THREADS")
            .help("Set number of threads to use to execute jobs (min: 1, default: cores available)"))
        .arg(Arg::new("verbosity")
            .short('v')
            .long("verbosity")
            .number_of_values(1)
            .value_name("VERBOSITY_LEVEL")
            .help("Set verbosity level for output (trace, debug, info, warn, default: error)"))
        .arg(Arg::new("flow-manifest")
            .num_args(1)
            .help("the file path of the 'flow' manifest file"))
        .arg(Arg::new("flow_args")
            .num_args(0..)
            .trailing_var_arg(true)
            .help("A list of arguments to pass to the flow."));

    app.get_matches()
}

/// Parse the command line arguments passed onto the flow itself
fn parse_flow_url(matches: &ArgMatches) -> Result<Url> {
    let cwd_url = Url::from_directory_path(env::current_dir()?)
        .map_err(|_| "Could not form a Url for the current working directory")?;
    url_from_string(&cwd_url, matches.get_one::<String>("flow-manifest")
        .map(|s| s.as_str()))
}

/// Set environment variable with the args this will not be unique, but it will be used very
/// soon and removed
fn get_flow_args(matches: &ArgMatches, flow_manifest_url: &Url) -> Vec<String> {
    // arg #0 is the flow url
    let mut flow_args: Vec<String> = vec![flow_manifest_url.to_string()];

    // append any other arguments for the flow passed from the command line
    let additional_args = match matches.get_many::<String>("flow_args") {
        Some(strings) => strings.map(|s| s.to_string()).collect(),
        None => vec![]
    };

    flow_args.extend(additional_args);

    flow_args
}

// Return addresses and ports to be used for each of the three queues
// - (general) job source
// - context job source
// - results sink
// - control messages
fn get_connect_addresses(ports: (u16, u16, u16, u16)) -> (String, String, String, String) {
    (
        format!("tcp://127.0.0.1:{}", ports.0),
        format!("tcp://127.0.0.1:{}", ports.1),
        format!("tcp://127.0.0.1:{}", ports.2),
        format!("tcp://127.0.0.1:{}", ports.3),
    )
}

// Return addresses to bind to for
// - (general) job source
// - context job source
// - results sink
// - control messages
fn get_bind_addresses(ports: (u16, u16, u16, u16)) -> (String, String, String, String) {
    (
        format!("tcp://*:{}", ports.0),
        format!("tcp://*:{}", ports.1),
        format!("tcp://*:{}", ports.2),
        format!("tcp://*:{}", ports.3),
    )
}

// Return four free ports to use for client-coordinator message queues
fn get_four_ports() -> Result<(u16, u16, u16, u16)> {
    Ok((pick_unused_port().chain_err(|| "No ports free")?,
        pick_unused_port().chain_err(|| "No ports free")?,
        pick_unused_port().chain_err(|| "No ports free")?,
        pick_unused_port().chain_err(|| "No ports free")?,
    ))
}