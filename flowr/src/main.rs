#![deny(missing_docs)]
#![warn(clippy::unwrap_used)]
//! `flowr` is the flow runner for the terminal. It reads a `flow` `Manifest` produced
//! by a flow compiler, such as `flowc`` that describes the network of collaborating functions
//! that are executed to execute the flow.
//!
//! Use `flowr` or `flowr --help` or `flowr -h` at the comment line to see the command line options
//!
//! The `context` module implements a set of 'Context' functions used by a runtime for flow execution.
//! This particular implementation of this set of functions is a "CLI" one that interacts with the
//! terminal for STDIO and the File system for files.

use std::{env, thread};
use std::path::PathBuf;
use std::process::exit;
use std::sync::{Arc, Mutex};

use clap::{Arg, ArgMatches, Command};
use log::{info, trace, warn};
use simpath::Simpath;
use simplog::SimpleLogger;
use url::Url;

#[cfg(feature = "debugger")]
use cli::cli_debug_client::CliDebugClient;
#[cfg(feature = "debugger")]
use cli::cli_debug_server::CliDebugServer;
use cli::cli_runtime_client::CliRuntimeClient;
use cli::cli_submitter::CLISubmitter;
#[cfg(feature = "debugger")]
use cli::client_server::ClientConnection;
use cli::client_server::ServerConnection;
use cli::client_server::ServerInfo;
#[cfg(feature = "debugger")]
use cli::debug_server_message::DebugServerMessage;
#[cfg(feature = "debugger")]
use cli::debug_server_message::DebugServerMessage::{BlockBreakpoint, DataBreakpoint, ExecutionEnded, ExecutionStarted,
                                                    ExitingDebugger, JobCompleted, JobError, Panic, PriorToSendingJob,
                                                    Resetting, WaitingForCommand};
use cli::runtime_messages::ClientMessage;
use flowcore::errors::*;
use flowcore::meta_provider::MetaProvider;
use flowcore::model::flow_manifest::FlowManifest;
use flowcore::model::submission::Submission;
use flowcore::provider::Provider;
use flowcore::url_helper::url_from_string;
use flowrlib::coordinator::Coordinator;
use flowrlib::executor::Executor;
use flowrlib::info as flowrlib_info;

/// We'll put our errors in an `errors` module, and other modules in this crate will
/// `use crate::errors::*;` to get access to everything `error_chain` creates.
pub mod errors;

#[cfg(feature = "debugger")]
mod cli;

/// The `Coordinator` of flow execution can run in one of these three modes:
/// - `ClientOnly`      - only as a client to submit flows for execution to a server
/// - `ServerOnly`      - only as a server waiting for submissions for execution from a client
/// - `ClientAndServer` - as both Client and Server, in separate threads
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Mode {
    #[cfg(feature = "debugger")]
    /// `Coordinator` mode where it runs as just a client for a server running in another process
    ClientOnly,
    /// `Coordinator` mode where it runs as just a server, clients must run in another process
    ServerOnly,
    /// `Coordinator` mode where a single process runs as a client and s server in different threads
    ClientAndServer,
}

// `RUNTIME_SERVICE_NAME` is the name of the runtime services and can be used to discover it by name
const RUNTIME_SERVICE_NAME: &str = "runtime._flowr._tcp.local";
const RUNTIME_SERVICE_PORT: u16 = 5555;

// `DEBUG_SERVICE_NAME` is the name of the runtime services and can be used to discover it by name
#[cfg(feature = "debugger")]
const DEBUG_SERVICE_NAME: &str = "debug._flowr._tcp.local";
const DEBUG_SERVICE_PORT: u16 = 5556;

/// Main for flowr binary - call `run()` and print any error that results or exit silently if OK
fn main() {
    match run() {
        Err(ref e) => {
            eprintln!("{e}");
            for e in e.iter().skip(1) {
                eprintln!("caused by: {e}");
            }
            exit(1);
        }
        Ok(_) => exit(0),
    }
}

/// For the lib provider, libraries maybe installed in multiple places in the file system.
/// In order to find the content, a FLOW_LIB_PATH environment variable can be configured with a
/// list of directories in which to look for the library in question.
///
/// Using the "FLOW_LIB_PATH" environment variable attempt to locate the library's root folder
/// in the file system.
fn set_lib_search_path(search_path_additions: &[String]) -> Result<Simpath> {
    let mut lib_search_path = Simpath::new_with_separator("FLOW_LIB_PATH", ',');

    if env::var("FLOW_LIB_PATH").is_err() && search_path_additions.is_empty() {
        warn!("'FLOW_LIB_PATH' is not set, and no LIB_DIRS supplied, so it is possible libraries referenced will not be found");
    }

    for additions in search_path_additions {
        lib_search_path.add(additions);
        info!("'{}' added to the Library Search Path", additions);
    }

    Ok(lib_search_path)
}

fn run() -> Result<()> {
    let matches = get_matches();

    let verbosity = matches.get_one::<String>("verbosity").map(|s| s.as_str());
    SimpleLogger::init_prefix_timestamp(verbosity, true, false);

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
    let lib_search_path = set_lib_search_path(&lib_dirs)?;

    let mode = if matches.get_flag("client") {
        Mode::ClientOnly
    } else if matches.get_flag("server") {
        Mode::ServerOnly
    } else {
        Mode::ClientAndServer
    };

    info!("Starting 'flowr' in {:?} mode", mode);

    let num_threads = num_threads(&matches);

    match mode {
        Mode::ServerOnly => server_only(num_threads, lib_search_path, native_flowstdlib)?,
        #[cfg(feature = "debugger")]
        Mode::ClientOnly => client_only(
            matches,
            lib_search_path,
            #[cfg(feature = "debugger")]
            debug_this_flow,
        )?,
        Mode::ClientAndServer => client_and_server(
            num_threads,
            lib_search_path,
            native_flowstdlib,
            matches,
            #[cfg(feature = "debugger")]
            debug_this_flow,
        )?,
    }

    Ok(())
}

// Start just a server - by running a Coordinator in the calling thread.
fn server_only(num_threads: usize, lib_search_path: Simpath, native_flowstdlib: bool) -> Result<()> {
    let runtime_server_connection = ServerConnection::new(RUNTIME_SERVICE_NAME, None)?;
    #[cfg(feature = "debugger")]
    let debug_server_connection = ServerConnection::new(DEBUG_SERVICE_NAME, None)?;

    info!("Starting 'flowr' server process in main thread");
    server(
        num_threads,
        lib_search_path,
        native_flowstdlib,
        runtime_server_connection,
        #[cfg(feature = "debugger")] debug_server_connection,
        true,
    )?;

    info!("'flowr' server process has exited");

    Ok(())
}

// Start a Server by running a Coordinator in a background thread, then start clients in the
// calling thread
fn client_and_server(
    num_threads: usize,
    lib_search_path: Simpath,
    native: bool,
    matches: ArgMatches,
    #[cfg(feature = "debugger")]
    debug_this_flow: bool,
) -> Result<()> {
    let runtime_server_connection = ServerConnection::new(RUNTIME_SERVICE_NAME, None)?;
    #[cfg(feature = "debugger")]
    let debug_server_connection = ServerConnection::new(DEBUG_SERVICE_NAME, None)?;

    let mut context_server_info = ServerInfo::new(RUNTIME_SERVICE_NAME, None, RUNTIME_SERVICE_PORT);
    #[cfg(feature = "debugger")]
    let mut debug_server_info = ServerInfo::new(DEBUG_SERVICE_NAME, None, DEBUG_SERVICE_PORT);

    let server_lib_search_path = lib_search_path.clone();

    thread::spawn(move || {
        info!("Starting 'flowr' server in background thread");
        let _ = server(
            num_threads,
            server_lib_search_path,
            native,
            runtime_server_connection,
            #[cfg(feature = "debugger")] debug_server_connection,
            false,
        );
    });

    #[cfg(feature = "debugger")]
    let control_c_client_connection = if debug_this_flow {
        Some(ClientConnection::new(&mut context_server_info)?)
    } else {
        None
    };

    let runtime_client_connection = ClientConnection::new(&mut context_server_info)?;

    client(
        matches,
        lib_search_path,
        runtime_client_connection,
        #[cfg(feature = "debugger")] control_c_client_connection,
        #[cfg(feature = "debugger")] debug_this_flow,
        #[cfg(feature = "debugger")] &mut debug_server_info,
    )
}

// Create a new `Coordinator`, pre-load any libraries in native format that we want to have before
// loading a flow and it's library references, then enter the `submission_loop()` accepting and
// executing flows submitted for execution, executing each one using the `Coordinator`
fn server(
    num_threads: usize,
    lib_search_path: Simpath,
    native_flowstdlib: bool,
    runtime_server_connection: ServerConnection,
    #[cfg(feature = "debugger")] debug_server_connection: ServerConnection,
    loop_forever: bool,
) -> Result<()> {
    let server_connection = Arc::new(Mutex::new(runtime_server_connection));

    #[cfg(feature = "debugger")]
    let mut debug_server = CliDebugServer {
        debug_server_connection
    };

    let provider = Arc::new(MetaProvider::new(lib_search_path,
                                         PathBuf::from("/")
    )) as Arc<dyn Provider>;

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

    executor.add_lib(
        cli::cli_context::get_manifest(server_connection.clone())?,
        Url::parse("memory://")? // Statically linked library has no resolved Url
    )?;
    executor.start(provider.clone(), num_threads, true, true)?;

    let mut submitter = CLISubmitter {
        runtime_server_connection: server_connection,
    };

    let mut coordinator = Coordinator::new(
        &mut submitter,
        #[cfg(feature = "debugger")] &mut debug_server
    )?;

    coordinator.submission_loop(loop_forever)?;

    Ok(())
}

// Start only a client in the calling thread. Since we are *only* starting a client in this
// process, we don't have server information, so we create a set of ServerInfo from command
// line options for the server address and known service names and ports.
#[cfg(feature = "debugger")]
fn client_only(
    matches: ArgMatches,
    lib_search_path: Simpath,
    #[cfg(feature = "debugger")] debug_this_flow: bool,
) -> Result<()> {
    let mut context_server_info = ServerInfo::new(
        RUNTIME_SERVICE_NAME,
        matches.get_one::<String>("address")
            .map(|s| s.as_str()),
        RUNTIME_SERVICE_PORT);
    #[cfg(feature = "debugger")]
    let mut debug_server_info = ServerInfo::new(
        DEBUG_SERVICE_NAME,
        matches.get_one::<String>("address")
            .map(|s| s.as_str()),
        DEBUG_SERVICE_PORT);

    #[cfg(feature = "debugger")]
        let control_c_client_connection = if debug_this_flow {
        Some(ClientConnection::new(&mut context_server_info)?)
    } else {
        None
    };

    let context_client_connection = ClientConnection::new(&mut context_server_info)?;

    client(
        matches,
        lib_search_path,
        context_client_connection,
        #[cfg(feature = "debugger")] control_c_client_connection,
        #[cfg(feature = "debugger")] debug_this_flow,
        #[cfg(feature = "debugger")] &mut debug_server_info,
    )
}

// Start the clients that talks to the server thread or process
#[cfg(feature = "debugger")]
fn client(
    matches: ArgMatches,
    lib_search_path: Simpath,
    runtime_client_connection: ClientConnection,
    #[cfg(feature = "debugger")]
    control_c_client_connection: Option<ClientConnection>,
    #[cfg(feature = "debugger")] debug_this_flow: bool,
    #[cfg(feature = "debugger")] debug_server_info: &mut ServerInfo,
) -> Result<()> {
    // keep an Arc Mutex protected set of override args that debug client can override
    let override_args = Arc::new(Mutex::new(Vec::<String>::new()));

    let flow_manifest_url = parse_flow_url(&matches)?;
    let provider = MetaProvider::new(lib_search_path, PathBuf::from("/"));
    let (flow_manifest, _) = FlowManifest::load(&provider, &flow_manifest_url)?;

    let flow_args = get_flow_args(&matches, &flow_manifest_url);
    let parallel_jobs_limit = matches.get_one::<usize>("jobs")
        .map(|i| i.to_owned());
    let submission = Submission::new(
        flow_manifest,
        parallel_jobs_limit,
        None, // No timeout waiting for job results
        #[cfg(feature = "debugger")] debug_this_flow,
    );

    trace!("Creating CliRuntimeClient");
    let runtime_client = CliRuntimeClient::new(
        flow_args,
        override_args.clone(),
        #[cfg(feature = "metrics")] matches.get_flag("metrics"),
    );

    #[cfg(feature = "debugger")]
    if debug_this_flow {
        let debug_client_connection = ClientConnection::new(debug_server_info)?;
        let debug_client = CliDebugClient::new(debug_client_connection,
            override_args);
        let _ = thread::spawn(move || {
            debug_client.debug_client_loop();
        });
    }

    info!("Client sending submission to server");
    runtime_client_connection.send(ClientMessage::ClientSubmission(submission))?;

    runtime_client.event_loop(
                                runtime_client_connection,
            #[cfg(feature = "debugger")] control_c_client_connection
    )?;

    Ok(())
}

// Determine the number of threads to use to execute flows
// - default (if value is not provided on the command line)of the number of cores
fn num_threads(matches: &ArgMatches) -> usize {
    match matches.get_one::<usize>("threads") {
        Some(num_threads) => *num_threads,
        None => thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
    }
}

// Parse the command line arguments using clap
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
            .help("Launch as flowr server"),
        )
        .arg(Arg::new("client")
            .short('c')
            .long("client")
            .action(clap::ArgAction::SetTrue)
            .conflicts_with("server")
            .help("Start flowr as a client to connect to a flowr server"),
        )
        .arg(Arg::new("address")
            .short('a')
            .long("address")
            .number_of_values(1)
            .value_name("ADDRESS")
            .conflicts_with("server")
            .help("The IP address of the flowr server to connect to"),
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

// Parse the command line arguments passed onto the flow itself
fn parse_flow_url(matches: &ArgMatches) -> Result<Url> {
    let cwd_url = Url::from_directory_path(env::current_dir()?)
        .map_err(|_| "Could not form a Url for the current working directory")?;
    url_from_string(&cwd_url, matches.get_one::<String>("flow-manifest")
        .map(|s| s.as_str()))
}

// Set environment variable with the args this will not be unique, but it will be used very
// soon and removed
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
