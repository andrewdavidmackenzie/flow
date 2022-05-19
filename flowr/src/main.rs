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

use clap::{App, AppSettings, Arg, ArgMatches};
use log::{error, info, warn};
use simpath::Simpath;
use simplog::SimpleLogger;
use url::Url;

use flowcore::meta_provider::{MetaProvider, Provider};
use flowcore::model::submission::Submission;
use flowcore::url_helper::url_from_string;
use flowrlib::coordinator::Coordinator;
use flowrlib::info as flowrlib_info;
use flowrlib::loader::Loader;

#[cfg(feature = "debugger")]
use crate::cli_debug_client::CliDebugClient;
#[cfg(feature = "debugger")]
use crate::cli_debug_server::CliDebugServer;
use crate::cli_runtime_client::CliRuntimeClient;
use crate::cli_runtime_server::CliServer;
use crate::client_server::{ClientConnection, DONT_WAIT, Method, ServerConnection, ServerInfo, WAIT};
#[cfg(feature = "debugger")]
use crate::debug_messages::DebugServerMessage;
#[cfg(feature = "debugger")]
use crate::DebugServerMessage::{BlockBreakpoint, DataBreakpoint, ExecutionEnded, ExecutionStarted,
                                ExitingDebugger, JobCompleted, JobError, Panic, PriorToSendingJob,
                                WaitingForCommand};
use crate::DebugServerMessage::Resetting;
use crate::errors::*;
use crate::runtime_messages::{ClientMessage, ServerMessage};

// Test helper functions
pub(crate) mod test_helper;

/// message_queue implementation of the communications between the runtime client, debug client and
/// the runtime server and debug server.
pub mod client_server;

/// runtime_messages is the enum for the different messages sent back and fore between the client
/// and server implementation of the CLI context functions
pub mod runtime_messages; // TODO see if can keep private or even remove

#[cfg(feature = "debugger")]
mod cli_debug_client;
mod cli_runtime_client;

#[cfg(feature = "debugger")]
mod cli_debug_server;
mod cli_runtime_server;

/// We'll put our errors in an `errors` module, and other modules in this crate will
/// `use crate::errors::*;` to get access to everything `error_chain` creates.
pub mod errors;
mod context;

/// 'debug' defines structs passed between the Server and the Client regarding debug events
/// and client responses to them
#[cfg(feature = "debugger")]
mod debug_messages;

/// `RUNTIME_SERVICE_NAME` is the name of the runtime services and can be used to discover it by name
pub const RUNTIME_SERVICE_NAME: &str = "runtime._flowr._tcp.local";
/// `DEBUG_SERVICE_NAME` is the name of the runtime services and can be used to discover it by name
#[cfg(feature = "debugger")]
pub const DEBUG_SERVICE_NAME: &str = "debug._flowr._tcp.local";

/// The `Coordinator` of flow execution can run in one of these three modes:
/// - `ClientOnly`      - only as a client to submit flows for execution to a server
/// - `ServerOnly`      - only as a server waiting for submissions for execution from a client
/// - `ClientAndServer` - as both Client and Server, in separate threads
#[derive(PartialEq, Clone, Debug)]
pub enum Mode {
    /// `Coordinator` mode where it runs as just a client for a server running in another process
    ClientOnly,
    /// `Coordinator` mode where it runs as just a server, clients must run in another process
    ServerOnly,
    /// `Coordinator` mode where a single process runs as a client and s server in different threads
    ClientAndServer,
}

/// Main for flowr binary - call `run()` and print any error that results or exit silently if OK
fn main() {
    match run() {
        Err(ref e) => {
            eprintln!("{}", e);
            for e in e.iter().skip(1) {
                eprintln!("caused by: {}", e);
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

/// # Example Submission of a flow for execution to the Coordinator
///
/// Instantiate the Coordinator server that receives the submitted flows to be executed, specifying
/// Create a `Submission` for the flow to be executed.
/// Create a `ClientConnection` to the `Coordinator` server
/// Send a `Submission` to the Coordinator to be executed
///
/// ```no_run
/// use std::sync::{Arc, Mutex};
/// use std::io;
/// use std::io::Write;
/// use flowrlib::coordinator::{Coordinator, Submission, Mode, RUNTIME_SERVICE_NAME, DEBUG_SERVICE_NAME};
/// use std::process::exit;
/// use flowcore::model::flow_manifest::FlowManifest;
/// use flowcore::model::metadata::MetaData;
/// use flowcontext_cli::runtime_messages::ClientMessage::ClientSubmission; // TODO
/// use simpath::Simpath;
/// use url::Url;
/// use flowcontext_cli::client_server::{ClientConnection, ServerConnection, ServerInfo, Method};
/// use flowrlib::runtime_messages::{ServerMessage, ClientMessage};
///
/// let runtime_server_connection = ServerConnection::new(RUNTIME_SERVICE_NAME, Method::Tcp(None)).unwrap();
/// let debug_server_connection = ServerConnection::new(DEBUG_SERVICE_NAME, Method::Tcp(None)).unwrap();
/// let mut runtime_server_info = runtime_server_connection.get_server_info().clone();///
///
/// // Spawn a thread where we will run the submission loop to receive submissions and execute them
/// std::thread::spawn(move || {
///     let mut coordinator = Coordinator::new(
///                                 runtime_server_connection,
///                                 #[cfg(feature = "debugger")] debug_server_connection,
///                                 1 /* num_threads */);
///
///     coordinator.submission_loop(
///         Simpath::new("fake path") /* lib_search_path */,
///         true /* native */,
///         false /* loop_forever */
///     ).expect("Problem in Submission loop");
///     });
///
/// let mut submission = Submission::new(&Url::parse("file:///temp/fake.toml").unwrap(),
///                                     1 /* num_parallel_jobs */,
///                                     true /* debug this flow's execution */);
/// let runtime_client_connection = ClientConnection::new(&mut runtime_server_info).unwrap();
/// runtime_client_connection.send(ClientSubmission(submission)).unwrap();
/// exit(0);
/// ```
fn run() -> Result<()> {
    info!(
        "'{}' version {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    info!("'flowrlib' version {}", flowrlib_info::version());

    let matches = get_matches();

    SimpleLogger::init_prefix_timestamp(matches.value_of("verbosity"), true, false);

    #[cfg(feature = "debugger")]
    let debug_this_flow = matches.is_present("debugger");
    let native_flowstdlib = matches.is_present("native");
    let lib_dirs = if matches.is_present("lib_dir") {
        matches
            .values_of("lib_dir")
            .chain_err(|| "Could not get the list of 'LIB_DIR' options specified")?
            .map(|s| s.to_string())
            .collect()
    } else {
        vec![]
    };
    let lib_search_path = set_lib_search_path(&lib_dirs)?;

    let mode = if matches.is_present("client") {
        Mode::ClientOnly
    } else if matches.is_present("server") {
        Mode::ServerOnly
    } else {
        Mode::ClientAndServer
    };
    info!("Starting 'flowr' in {:?} mode", mode);

    let num_threads = num_threads(&matches);

    match mode {
        Mode::ServerOnly => server_only(num_threads, lib_search_path, native_flowstdlib)?,
        Mode::ClientOnly => client_only(
            matches,
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

fn load_native_libs(
    native_flowstdlib: bool,
    loader: &mut Loader,
    provider: &dyn Provider,
    server_connection: Arc<Mutex<ServerConnection>>,
) -> Result<()> {
    // Add the native context functions to functions available for use by the flow
    loader.add_lib(
            provider,
            context::get_manifest(server_connection)?,
            &Url::parse("context://")?,
        )?;

    // if the command line options request loading native implementation of available native libs
    // if not, the native implementation is not loaded and later when a flow is loaded it's library
    // references will be resolved and those libraries (WASM implementations) will be loaded at runtime
    if native_flowstdlib {
        #[cfg(feature = "flowstdlib")]
        loader.add_lib(
                provider,
                flowstdlib::manifest::get_manifest()
                    .chain_err(|| "Could not get 'native' flowstdlib manifest")?,
                &Url::parse("lib://flowstdlib")?,
            )?;
    }

    Ok(())
}

// Start just a server - by running a Coordinator in the calling thread.
fn server_only(num_threads: usize, lib_search_path: Simpath, native_flowstdlib: bool) -> Result<()> {
    let runtime_server_connection = ServerConnection::new(RUNTIME_SERVICE_NAME, Method::Tcp(None))?;
    #[cfg(feature = "debugger")]
    let debug_server_connection = ServerConnection::new(DEBUG_SERVICE_NAME, Method::Tcp(None))?;

    info!("Starting 'flowr' server process in main thread");
    server(
        num_threads,
        lib_search_path,
        native_flowstdlib,
        runtime_server_connection,
        #[cfg(feature = "debugger")]
        debug_server_connection,
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
) -> flowcore::errors::Result<()> {
    let runtime_server_connection = ServerConnection::new(RUNTIME_SERVICE_NAME, Method::InProc(None))?;
    #[cfg(feature = "debugger")]
    let debug_server_connection = ServerConnection::new(DEBUG_SERVICE_NAME, Method::InProc(None))?;

    let mut runtime_server_info = runtime_server_connection.get_server_info().clone();
    #[cfg(feature = "debugger")]
    let mut debug_server_info = debug_server_connection.get_server_info().clone();

    thread::spawn(move || {
        info!("Starting 'flowr' server in background thread");
        let _ = server(
            num_threads,
            lib_search_path,
            native,
            runtime_server_connection,
            #[cfg(feature = "debugger")]
            debug_server_connection,
            false,
        );
    });

    #[cfg(feature = "debugger")]
    let control_c_client_connection = if debug_this_flow {
        Some(ClientConnection::new(&mut runtime_server_info)?)
    } else {
        None
    };

    let runtime_client_connection = ClientConnection::new(&mut runtime_server_info)?;

    client(
        matches,
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
    let mut loader = Loader::new();
    let provider = MetaProvider::new(lib_search_path, PathBuf::from("/"));

    let server_connection = Arc::new(Mutex::new(runtime_server_connection));

    load_native_libs(
        native_flowstdlib,
        &mut loader,
        &provider,
        server_connection.clone(),
    )?;

    let mut server = CliServer {
        runtime_server_connection: server_connection,
    };

    #[cfg(feature = "debugger")]
    let mut debug_server = CliDebugServer {
        debug_server_connection
    };

    let mut coordinator = Coordinator::new(num_threads, &mut server,
                                           #[cfg(feature = "debugger")] &mut debug_server);

    coordinator.submission_loop(
        loader,
        provider,
        loop_forever,
    )?;

    Ok(())
}

// Start only a client in the calling thread. Since we are *only* starting a client in this
// process, we don't have server information, so we create a set of ServerInfo from command
// line options for the server address and known service names and ports.
fn client_only(
    matches: ArgMatches,
    #[cfg(feature = "debugger")] debug_this_flow: bool,
) -> flowcore::errors::Result<()> {
    let mut runtime_server_info = ServerInfo::new(
        RUNTIME_SERVICE_NAME,
        Method::Tcp(
        matches
            .value_of("address")
            .map(|s| s.to_string())
            .map(|name| (name, 5555)),
    ));
    #[cfg(feature = "debugger")]
    let mut debug_server_info = ServerInfo::new(
        DEBUG_SERVICE_NAME,
        Method::Tcp(
        matches
            .value_of("address")
            .map(|s| s.to_string())
            .map(|name| (name, 5556)),
    ));

    #[cfg(feature = "debugger")]
        let control_c_client_connection = if debug_this_flow {
        Some(ClientConnection::new(&mut runtime_server_info)?)
    } else {
        None
    };

    let runtime_client_connection = ClientConnection::new(&mut runtime_server_info)?;

    client(
        matches,
        runtime_client_connection,
        #[cfg(feature = "debugger")] control_c_client_connection,
        #[cfg(feature = "debugger")] debug_this_flow,
        #[cfg(feature = "debugger")] &mut debug_server_info,
    )
}

// Start the clients that talks to the server thread or process
fn client(
    matches: ArgMatches,
    runtime_client_connection: ClientConnection,
    #[cfg(feature = "debugger")]
    control_c_client_connection: Option<ClientConnection>,
    #[cfg(feature = "debugger")] debug_this_flow: bool,
    #[cfg(feature = "debugger")] debug_server_info: &mut ServerInfo,
) -> flowcore::errors::Result<()> {
    let flow_manifest_url = parse_flow_url(&matches)?;
    let flow_args = get_flow_args(&matches, &flow_manifest_url);
    let max_parallel_jobs = num_parallel_jobs(&matches);
    let submission = Submission::new(
        &flow_manifest_url,
        max_parallel_jobs,
        #[cfg(feature = "debugger")]
        debug_this_flow,
    );

    #[cfg(feature = "debugger")]
    if debug_this_flow {
        let debug_client_connection = ClientConnection::new(debug_server_info)?;
        let debug_client = CliDebugClient::new(debug_client_connection);
        let _ = thread::spawn(move || {
            debug_client.debug_client_loop();
        });
    }

    let runtime_client = CliRuntimeClient::new(
        flow_args,
        #[cfg(feature = "metrics")]
        matches.is_present("metrics"),
    );

    info!("Client sending submission to server");
    runtime_client_connection.send(ClientMessage::ClientSubmission(submission))?;

     runtime_client.event_loop(runtime_client_connection,
            #[cfg(feature = "debugger")]
                               control_c_client_connection
     )
}

// Determine the number of threads to use to execute flows, with a default of the number of cores
// in the device, or any override from the command line.
fn num_threads(matches: &ArgMatches) -> usize {
    let mut num_threads: usize = 0;

    if let Some(value) = matches.value_of("threads") {
        if let Ok(threads) = value.parse::<usize>() {
            if threads < 1 {
                error!("Minimum number of additional threads is '1', \
                so option has been overridden to be '1'");
                num_threads = 1;
            } else {
                num_threads = threads;
            }
        }
    }

    if num_threads == 0 {
        num_threads = thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    }

    num_threads
}

// Determine the number of parallel jobs to be run in parallel based on a default of 2 times
// the number of cores in the device, or any override from the command line.
fn num_parallel_jobs(
    matches: &ArgMatches,
) -> usize {
    let mut num_jobs: usize = 0;

    if let Some(value) = matches.value_of("jobs") {
        if let Ok(jobs) = value.parse::<usize>() {
            if jobs < 1 {
                error!("Minimum number of parallel jobs is '0', \
                so option of '{jobs}' has been overridden to be '1'");
                num_jobs = 1;
            } else {
                num_jobs = jobs;
            }
        }
    }

    if num_jobs == 0 {
        num_jobs = thread::available_parallelism().map(|n| 2 * n.get()).unwrap_or(1);
    }

    num_jobs
}

// Parse the command line arguments using clap
fn get_matches<'a>() -> ArgMatches<'a> {
    let app = App::new(env!("CARGO_PKG_NAME"))
        .setting(AppSettings::TrailingVarArg)
        .version(env!("CARGO_PKG_VERSION"));

    let app = app.arg(Arg::with_name("jobs")
        .short("j")
        .long("jobs")
        .takes_value(true)
        .value_name("MAX_JOBS")
        .help("Set maximum number of jobs that can be running in parallel)"))
        .arg(Arg::with_name("lib_dir")
            .short("L")
            .long("libdir")
            .number_of_values(1)
            .multiple(true)
            .value_name("LIB_DIR|BASE_URL")
            .help("Add a directory or base Url to the Library Search path"))
        .arg(Arg::with_name("threads")
            .short("t")
            .long("threads")
            .takes_value(true)
            .value_name("THREADS")
            .help("Set number of threads to use to execute jobs (min: 1, default: cores available)"))
        .arg(Arg::with_name("verbosity")
            .short("v")
            .long("verbosity")
            .takes_value(true)
            .value_name("VERBOSITY_LEVEL")
            .help("Set verbosity level for output (trace, debug, info, warn, error (default))"))
        .arg(Arg::with_name("flow-manifest")
            .help("the file path of the 'flow' manifest file")
            .required(false)
            .index(1))
        .arg(Arg::with_name("flow-arguments")
            .multiple(true)
            .help("A list of arguments to pass to the flow when executed."));

    let app = app.arg(
        Arg::with_name("server")
            .short("s")
            .long("server")
            .help("Launch as flowr server"),
    );

    let app = app.arg(
        Arg::with_name("client")
            .short("c")
            .long("client")
            .conflicts_with("server")
            .help("Start flowr as a client to connect to a flowr server"),
    );

    let app = app.arg(
        Arg::with_name("address")
            .short("a")
            .long("address")
            .takes_value(true)
            .value_name("ADDRESS")
            .conflicts_with("server")
            .help("The IP address of the flowr server to connect to"),
    );

    #[cfg(feature = "debugger")]
    let app = app.arg(
        Arg::with_name("debugger")
            .short("d")
            .long("debugger")
            .help("Enable the debugger when running a flow"),
    );

    #[cfg(feature = "metrics")]
    let app = app.arg(
        Arg::with_name("metrics")
            .short("m")
            .long("metrics")
            .help("Calculate metrics during flow execution and print them out when done"),
    );

    #[cfg(not(feature = "wasm"))]
    let app = app.arg(
        Arg::with_name("native")
            .short("n")
            .long("native")
            .conflicts_with("client")
            .help("Link with native (not WASM) version of flowstdlib"),
    );

    app.get_matches()
}

// Parse the command line arguments passed onto the flow itself
fn parse_flow_url(matches: &ArgMatches) -> flowcore::errors::Result<Url> {
    let cwd_url = Url::from_directory_path(env::current_dir()?)
        .map_err(|_| "Could not form a Url for the current working directory")?;
    url_from_string(&cwd_url, matches.value_of("flow-manifest"))
}

// Set environment variable with the args this will not be unique, but it will be used very
// soon and removed
fn get_flow_args(matches: &ArgMatches, flow_manifest_url: &Url) -> Vec<String> {
    // arg #0 is the flow url
    let mut flow_args: Vec<String> = vec![flow_manifest_url.to_string()];

    // append any other arguments for the flow passed from the command line
    if let Some(args) = matches.values_of("flow-arguments") {
        flow_args.extend(args.map(|arg| arg.to_string()));
    }

    flow_args
}