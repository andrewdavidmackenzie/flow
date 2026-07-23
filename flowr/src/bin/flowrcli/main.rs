#![deny(clippy::unwrap_used, clippy::expect_used)]

//! `flowr` is a command line flow runner for running `flow` programs.
//!
//! It reads a compiled [`FlowManifest`][flowcore::model::flow_manifest::FlowManifest] produced by a
//! flow compiler, such as `flowc`, that describes the graph of communicating functions that
//! constitute the flow program.
//!
//! Use `flowr --help` or `flowr -h` at the command line to see the command line options
//!
//! The [cli] module implements a set of `context functions`, adapted to Terminal IO and local
//! File System, that allow the flow program to interact with the environment where it is being run.
//!
//! Depending on the command line options supplied `flowr` executes the
//! [`Coordinator`][flowrlib::coordinator::Coordinator] of flow execution in a background thread,
//! or the [`cli::cli_client`] in the main thread (where the interaction with STDIO and
//! File System happens) or both. They communicate via network messages using the
//! [`SubmissionHandler`][flowrlib::submission_handler::SubmissionHandler] to submit flows for execution,
//! and interchanging [`ClientMessages`][crate::cli::coordinator_message::ClientMessage]
//! and [`CoordinatorMessages`][crate::cli::coordinator_message::CoordinatorMessage] for execution of context
//! interaction in the client, as requested by functions running in the coordinator's
//! [`Executors`][flowrlib::executor::Executor]

use core::str::FromStr;
use std::io::Write;
use std::path::PathBuf;
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{env, thread};

use clap::{Arg, ArgMatches, Command};
use env_logger::Builder;
use log::{error, info, trace, LevelFilter};
use portpicker::pick_unused_port;
use simpath::Simpath;
use url::Url;

use cli::cli_client::CliRuntimeClient;
#[cfg(feature = "submission")]
use cli::cli_submission_handler::CLISubmissionHandler;
use cli::coordinator_message::ClientMessage;
use flowcore::errors::{Result, ResultExt};
use flowcore::meta_provider::MetaProvider;
use flowcore::model::flow_manifest::FlowManifest;
use flowcore::model::submission::Submission;
use flowcore::provider::Provider;
use flowcore::url_helper::url_from_string;
use flowrlib::connections::{ClientConnection, CoordinatorConnection};
use flowrlib::coordinator::Coordinator;
#[cfg(feature = "debugger")]
use flowrlib::debug_zmq_handler::DebugZmqHandler;
use flowrlib::discovery::{
    create_service_daemon, discover_service, discover_service_on, register_service,
    shutdown_service_daemon, unregister_service, ServiceDaemon,
};
use flowrlib::dispatcher::Dispatcher;
use flowrlib::executor::Executor;
use flowrlib::info as flowrlib_info;
#[cfg(feature = "debugger")]
use flowrlib::services::DEBUG_SERVICE_NAME;
use flowrlib::services::{
    CONTROL_SERVICE_NAME, COORDINATOR_SERVICE_NAME, JOB_SERVICE_NAME, RESULTS_JOB_SERVICE_NAME,
};

/// Include the module that implements the context functions
mod context;

/// provides the `context functions` for interacting with the execution environment from a flow,
/// plus client-[Coordinator][flowrlib::coordinator::Coordinator] implementations of
/// [`flowrlib::submission_handler`] for executing them on different threads
/// from the [`Coordinator`][`flowrlib::coordinator::Coordinator`]
mod cli;

/// provides [Error][errors::Error] that other modules in this crate will `use crate::errors::*;` to get
/// access to everything `error_chain` creates.
mod errors;

/// Main for flowr binary - call `run()` and print any error that results or exit silently if OK
fn main() {
    let result = run();
    let _ = std::io::stdout().flush();

    if let Err(ref e) = result {
        error!("{e}");
        for e in e.iter().skip(1) {
            error!("caused by: {e}");
        }

        // The backtrace is generated if env var `RUST_BACKTRACE` is set to `1` or `full`
        if let Some(backtrace) = e.backtrace() {
            error!("backtrace: {backtrace:?}");
        }

        exit(1);
    }
}

/// For the lib provider, libraries maybe installed in multiple places in the file system.
/// In order to find the content, a `FLOW_LIB_PATH` environment variable can be configured with a
/// list of directories in which to look for the library in question.
fn get_lib_search_path(search_path_additions: &[String]) -> Simpath {
    let mut lib_search_path = Simpath::new_with_separator("FLOW_LIB_PATH", ',');

    for additions in search_path_additions {
        lib_search_path.add(additions);
        info!("'{additions}' added to the Library Search Path");
    }

    if lib_search_path.is_empty() {
        if let Some(default_path) = flowcore::dirs::lib_dir() {
            lib_search_path.add(&default_path.to_string_lossy());
        }
    }

    lib_search_path
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
            .map(std::string::ToString::to_string)
            .collect()
    } else {
        vec![]
    };
    let lib_search_path = get_lib_search_path(&lib_dirs);
    let num_threads = num_threads(&matches);

    if matches.get_flag("client") {
        client_only(
            &matches,
            lib_search_path,
            #[cfg(feature = "debugger")]
            debug_this_flow,
        )?;
    } else if matches.get_flag("server") {
        coordinator_only(num_threads, lib_search_path, native_flowstdlib)?;
    } else {
        client_and_coordinator(
            num_threads,
            lib_search_path,
            native_flowstdlib,
            &matches,
            #[cfg(feature = "debugger")]
            debug_this_flow,
        )?;
    }

    Ok(())
}

/// Start just a [Coordinator][flowrlib::coordinator::Coordinator] in the calling thread.
fn coordinator_only(
    num_threads: usize,
    lib_search_path: Simpath,
    native_flowstdlib: bool,
) -> Result<()> {
    let mdns = create_service_daemon()?;

    let coordinator_port = pick_unused_port().chain_err(|| "No ports free")?;
    let coordinator_connection =
        CoordinatorConnection::new(COORDINATOR_SERVICE_NAME, coordinator_port)?;
    let blocking_io_port = pick_unused_port().chain_err(|| "No ports free")?;
    let blocking_io_connection = CoordinatorConnection::new("blocking-io", blocking_io_port)?;
    let mut fullnames = vec![register_service(
        &mdns,
        COORDINATOR_SERVICE_NAME,
        coordinator_port,
    )?];

    #[cfg(feature = "debugger")]
    let debug_port = pick_unused_port().chain_err(|| "No ports free")?;
    #[cfg(feature = "debugger")]
    let debug_server_connection = CoordinatorConnection::new(DEBUG_SERVICE_NAME, debug_port)?;
    #[cfg(feature = "debugger")]
    fullnames.push(register_service(&mdns, DEBUG_SERVICE_NAME, debug_port)?);

    #[cfg(feature = "debugger")]
    eprintln!("Debug server listening on port {debug_port}. Connect with: flowrdb --address localhost:{debug_port}");

    // Signal to the parent process (e.g. test harness) that the server is ready
    println!("ready");

    info!("Starting coordinator in main thread");
    let result = coordinator(
        &mdns,
        num_threads,
        lib_search_path,
        native_flowstdlib,
        coordinator_connection,
        blocking_io_connection,
        #[cfg(feature = "debugger")]
        debug_server_connection,
        true,
    );

    if let Err(e) = shutdown_service_daemon(&mdns, &fullnames) {
        error!("Could not shut down mDNS daemon: {e}");
    }

    result?;

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
    #[cfg(feature = "debugger")] debug_this_flow: bool,
) -> Result<()> {
    let mdns = Arc::new(create_service_daemon()?);

    let runtime_port = pick_unused_port().chain_err(|| "No ports free")?;
    let coordinator_connection =
        CoordinatorConnection::new(COORDINATOR_SERVICE_NAME, runtime_port)?;
    let blocking_io_port = pick_unused_port().chain_err(|| "No ports free")?;
    let blocking_io_connection = CoordinatorConnection::new("blocking-io", blocking_io_port)?;

    let mut fullnames = vec![register_service(
        &mdns,
        COORDINATOR_SERVICE_NAME,
        runtime_port,
    )?];

    #[cfg(feature = "debugger")]
    let debug_port = pick_unused_port().chain_err(|| "No ports free")?;
    #[cfg(feature = "debugger")]
    let debug_connection = CoordinatorConnection::new(DEBUG_SERVICE_NAME, debug_port)?;
    #[cfg(feature = "debugger")]
    fullnames.push(register_service(&mdns, DEBUG_SERVICE_NAME, debug_port)?);

    #[cfg(feature = "debugger")]
    if debug_this_flow {
        eprintln!("Debug server listening on port {debug_port}. Connect with: flowrdb --address localhost:{debug_port}");
    }

    let coordinator_lib_search_path = lib_search_path.clone();
    let coordinator_mdns = Arc::clone(&mdns);

    info!("Starting coordinator in background thread");
    let coordinator_handle = thread::spawn(move || {
        if let Err(e) = coordinator(
            &coordinator_mdns,
            num_threads,
            coordinator_lib_search_path,
            native_flowstdlib,
            coordinator_connection,
            blocking_io_connection,
            #[cfg(feature = "debugger")]
            debug_connection,
            false,
        ) {
            error!("Coordinator thread exited with error: {e}");
        }
    });

    let coordinator_address = discover_service_on(&mdns, COORDINATOR_SERVICE_NAME)?;
    let runtime_client_connection = ClientConnection::new(&coordinator_address)?;
    let blocking_io_address = format!("127.0.0.1:{blocking_io_port}");
    let blocking_io_client = ClientConnection::new(&blocking_io_address)?;

    let result = client(
        matches,
        lib_search_path,
        runtime_client_connection,
        blocking_io_client,
        #[cfg(feature = "debugger")]
        debug_this_flow,
    );

    let _ = coordinator_handle.join();

    // Both the client (this thread) and the coordinator (now joined) are done using
    // the daemon - unregister our services (send "goodbye" packets) and shut it down.
    if let Err(e) = shutdown_service_daemon(&mdns, &fullnames) {
        error!("Could not shut down mDNS daemon: {e}");
    }

    result
}

/// Create a new `Coordinator`, preload any libraries in native format that we want to have before
/// loading a flow, and its library references, then enter the `submission_loop()` accepting and
/// executing flows submitted for execution, executing each one using the `Coordinator`
#[allow(clippy::too_many_arguments)]
fn coordinator(
    mdns: &ServiceDaemon,
    num_threads: usize,
    lib_search_path: Simpath,
    native_flowstdlib: bool,
    coordinator_connection: CoordinatorConnection,
    blocking_io_connection: CoordinatorConnection,
    #[cfg(feature = "debugger")] debug_connection: CoordinatorConnection,
    loop_forever: bool,
) -> Result<()> {
    // Two channels: non-blocking IO (stdout, etc.) and blocking IO (readline, stdin).
    // Each has its own bridge thread and ZMQ socket so they operate independently.
    let (nonblocking_tx, nonblocking_rx) = std::sync::mpsc::channel();
    let (blocking_tx, blocking_rx) = std::sync::mpsc::channel();
    let (submission_tx, submission_rx) = std::sync::mpsc::channel();
    let context_io = context::ContextIO::new(blocking_tx, nonblocking_tx);

    // Non-blocking bridge: handles stdout, stderr, file, image, args, and protocol messages
    let nonblocking_bridge = thread::spawn(move || {
        coordinator_bridge(coordinator_connection, nonblocking_rx, submission_tx);
    });

    // Blocking bridge: handles readline and stdin (can block indefinitely)
    let blocking_bridge = thread::spawn(move || {
        blocking_io_bridge(blocking_io_connection, blocking_rx);
    });

    #[cfg(feature = "debugger")]
    let mut debug_server = DebugZmqHandler {
        debug_server_connection: debug_connection,
    };

    let provider =
        Arc::new(MetaProvider::new(lib_search_path, PathBuf::default())) as Arc<dyn Provider>;

    let ports = get_four_ports()?;
    trace!("Announcing three job queues and a control socket on ports: {ports:?}");
    let job_queues = get_bind_addresses(ports);
    let dispatcher = Dispatcher::new(&job_queues)?;
    let fullnames = [
        register_service(mdns, JOB_SERVICE_NAME, ports.0)?,
        register_service(mdns, RESULTS_JOB_SERVICE_NAME, ports.2)?,
        register_service(mdns, CONTROL_SERVICE_NAME, ports.3)?,
    ];

    let (job_source_name, context_job_source_name, results_sink, control_socket) =
        get_connect_addresses(ports);

    let mut executor = Executor::new();
    #[cfg(feature = "flowstdlib")]
    if native_flowstdlib {
        executor.add_lib(
            flowstdlib::manifest::get()
                .chain_err(|| "Could not get 'native' flowstdlib manifest")?,
            Url::parse("memory://")?,
        )?;
    }
    executor.start(
        &provider,
        num_threads,
        &job_source_name,
        &results_sink,
        &control_socket,
    );

    let mut context_executor = Executor::new();
    context_executor.add_lib(
        context::get_manifest(context_io.clone())?,
        Url::parse("memory://")?,
    )?;
    context_executor.start(
        &provider,
        2,
        &context_job_source_name,
        &results_sink,
        &control_socket,
    );

    #[cfg(feature = "submission")]
    let mut submitter = CLISubmissionHandler::new(context_io, submission_rx);

    let mut coordinator = Coordinator::new(
        dispatcher,
        #[cfg(feature = "submission")]
        &mut submitter,
        #[cfg(feature = "debugger")]
        &mut debug_server,
    );

    #[cfg(feature = "submission")]
    let result = coordinator.submission_loop(loop_forever);
    #[cfg(not(feature = "submission"))]
    let result: Result<()> = Ok(());

    // Unregister our job/results/control services (send "goodbye" packets). The daemon
    // itself is shared with the caller, which is responsible for shutting it down once
    // it has also unregistered its own (runtime/debug) services.
    for fullname in &fullnames {
        unregister_service(mdns, fullname);
    }

    let _ = nonblocking_bridge.join();
    let _ = blocking_bridge.join();

    result
}

/// Bridge thread that owns the ZMQ `CoordinatorConnection` and serializes all
/// communication between context functions/submission handler and the client.
#[allow(clippy::needless_pass_by_value)]
fn coordinator_bridge(
    mut connection: CoordinatorConnection,
    context_rx: std::sync::mpsc::Receiver<context::ContextRequest>,
    submission_tx: std::sync::mpsc::Sender<flowcore::model::submission::Submission>,
) {
    use crate::cli::coordinator_message::{ClientMessage, CoordinatorMessage};
    use flowrlib::connections::WAIT;
    use log::debug;

    debug!("[BRIDGE] started");

    while let Ok(request) = context_rx.recv() {
        let is_exiting = matches!(request.message, CoordinatorMessage::CoordinatorExiting(_));
        let wait_for_submission = matches!(request.message, CoordinatorMessage::Invalid);

        debug!(
            "[BRIDGE] received: {}, wait_sub={wait_for_submission}, exiting={is_exiting}",
            request.message
        );

        if wait_for_submission {
            debug!("[BRIDGE] waiting for submission on ZMQ...");
            loop {
                match connection.receive::<ClientMessage>(WAIT) {
                    Ok(ClientMessage::ClientSubmission(submission)) => {
                        if submission_tx.send(*submission).is_err() {
                            return;
                        }
                        break;
                    }
                    Ok(ClientMessage::ClientExiting(_)) => {
                        if let Some(response_tx) = request.response_tx {
                            let _ = response_tx.send(ClientMessage::ClientExiting(Ok(())));
                        }
                        return;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        error!("Bridge: error receiving submission: {e}");
                        return;
                    }
                }
            }
            if let Some(response_tx) = request.response_tx {
                let _ = response_tx.send(ClientMessage::Ack);
            }
            continue;
        }

        if is_exiting {
            // The exit may have already been handled in the response path
            // (ClientExiting received as response to FlowEnd). In that case
            // the ZMQ socket already sent CoordinatorExiting. Just ack and return.
            if let Some(response_tx) = request.response_tx {
                let _ = response_tx.send(ClientMessage::Ack);
            }
            return;
        }

        debug!("[BRIDGE] sending on ZMQ...");
        if let Err(e) = connection.send(request.message) {
            debug!("[BRIDGE] send failed: {e}");
            error!("Bridge: failed to send to client: {e}");
            if let Some(response_tx) = request.response_tx {
                let _ = response_tx.send(ClientMessage::Error(format!("{e}")));
            }
            continue;
        }

        debug!("[BRIDGE] waiting for ZMQ response...");
        match connection.receive::<ClientMessage>(WAIT) {
            Ok(ClientMessage::ClientExiting(_)) => {
                debug!("[BRIDGE] client exited, completing REP cycle");
                let _ = connection.send(CoordinatorMessage::CoordinatorExiting(Ok(())));
                if let Some(response_tx) = request.response_tx {
                    let _ = response_tx.send(ClientMessage::ClientExiting(Ok(())));
                }
            }
            Ok(response) => {
                if let Some(response_tx) = request.response_tx {
                    let _ = response_tx.send(response);
                }
            }
            Err(e) => {
                error!("Bridge: failed to receive from client: {e}");
                if let Some(response_tx) = request.response_tx {
                    let _ = response_tx.send(ClientMessage::Error(format!("{e}")));
                }
            }
        }
    }
}

/// Bridge thread for blocking IO (`readline`, `stdin`).
/// Simpler than `coordinator_bridge` — no submission handling, just forwards
/// requests and responses on its own ZMQ socket.
#[allow(clippy::needless_pass_by_value)]
fn blocking_io_bridge(
    mut connection: CoordinatorConnection,
    context_rx: std::sync::mpsc::Receiver<context::ContextRequest>,
) {
    use crate::cli::coordinator_message::ClientMessage;
    use flowrlib::connections::WAIT;

    // Receive the initial Ack that starts the REQ/REP cycle
    if connection.receive::<ClientMessage>(WAIT).is_err() {
        return;
    }

    while let Ok(request) = context_rx.recv() {
        if let Err(e) = connection.send(request.message) {
            error!("Blocking bridge: failed to send: {e}");
            if let Some(response_tx) = request.response_tx {
                let _ = response_tx.send(ClientMessage::Error(format!("{e}")));
            }
            continue;
        }

        match connection.receive::<ClientMessage>(WAIT) {
            Ok(response) => {
                if let Some(response_tx) = request.response_tx {
                    let _ = response_tx.send(response);
                }
            }
            Err(e) => {
                error!("Blocking bridge: failed to receive: {e}");
                if let Some(response_tx) = request.response_tx {
                    let _ = response_tx.send(ClientMessage::Error(format!("{e}")));
                }
            }
        }
    }
}

/// Start only a client in the calling thread. Discover the remote Coordinator using service discovery
fn client_only(
    matches: &ArgMatches,
    lib_search_path: Simpath,
    #[cfg(feature = "debugger")] debug_this_flow: bool,
) -> Result<()> {
    let coordinator_address = discover_service(COORDINATOR_SERVICE_NAME)?;
    let client_connection = ClientConnection::new(&coordinator_address)?;
    // In remote mode, blocking IO uses the same connection (no concurrency yet)
    let blocking_io_client = ClientConnection::new(&coordinator_address)?;

    client(
        matches,
        lib_search_path,
        client_connection,
        blocking_io_client,
        #[cfg(feature = "debugger")]
        debug_this_flow,
    )
}

/// Start the client that talks to the coordinator
fn client(
    matches: &ArgMatches,
    lib_search_path: Simpath,
    client_connection: ClientConnection,
    blocking_io_client: ClientConnection,
    #[cfg(feature = "debugger")] debug_this_flow: bool,
) -> Result<()> {
    let override_args = Arc::new(Mutex::new(Vec::<String>::new()));

    let flow_manifest_url = parse_flow_url(matches)?;
    let provider = MetaProvider::new(lib_search_path, PathBuf::default());
    let (flow_manifest, _) = FlowManifest::load(&provider, &flow_manifest_url)?;

    let flow_args = get_flow_args(matches, &flow_manifest_url);
    let parallel_jobs_limit = matches
        .get_one::<usize>("jobs")
        .map(std::borrow::ToOwned::to_owned);
    let job_timeout = matches
        .get_one::<u64>("job-timeout")
        .map(|secs| Duration::from_secs(*secs));
    let submission = Submission::new(
        flow_manifest,
        parallel_jobs_limit,
        job_timeout,
        #[cfg(feature = "debugger")]
        debug_this_flow,
    );

    trace!("Creating CliRuntimeClient");
    let client = CliRuntimeClient::new(
        flow_args,
        override_args,
        #[cfg(feature = "metrics")]
        matches.get_flag("metrics"),
    );

    info!("Client sending submission to coordinator");
    client_connection.send(ClientMessage::ClientSubmission(Box::new(submission)))?;

    // Start the blocking IO REQ/REP cycle with an initial Ack
    blocking_io_client.send(ClientMessage::Ack)?;

    trace!("Entering client event loop");
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Could not create tokio runtime: {e}"))?;
    rt.block_on(client.event_loop(client_connection, blocking_io_client))
}

/// Determine the number of threads to use to execute flows
/// - default (if value is not provided on the command line)of the number of cores
#[allow(clippy::redundant_closure_for_method_calls)]
fn num_threads(matches: &ArgMatches) -> usize {
    match matches.get_one::<usize>("threads") {
        Some(num_threads) => *num_threads,
        None =>
        {
            #[allow(clippy::redundant_closure)]
            thread::available_parallelism().map_or(1, |n| n.get())
        }
    }
}

/// Parse the command line arguments using clap
fn get_matches() -> ArgMatches {
    let app = Command::new(env!("CARGO_PKG_NAME")).version(env!("CARGO_PKG_VERSION"));

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

    #[cfg(feature = "flowstdlib")]
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
             .action(clap::ArgAction::SetTrue)
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
        .arg(Arg::new("job-timeout")
            .long("job-timeout")
            .number_of_values(1)
            .value_parser(clap::value_parser!(u64).range(1..))
            .value_name("SECONDS")
            .help("Set timeout in seconds for job execution (lost jobs are retried)"))
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
            .help("Set verbosity level for output (trace, debug, info, warn, error(default), off)"))
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
        .map_err(|()| "Could not form a Url for the current working directory")?;
    url_from_string(
        &cwd_url,
        matches
            .get_one::<String>("flow-manifest")
            .map(String::as_str),
    )
}

/// Set environment variable with the args this will not be unique, but it will be used very
/// soon and removed
fn get_flow_args(matches: &ArgMatches, flow_manifest_url: &Url) -> Vec<String> {
    // arg #0 is the flow url
    let mut flow_args: Vec<String> = vec![flow_manifest_url.to_string()];

    // append any other arguments for the flow passed from the command line
    let additional_args = match matches.get_many::<String>("flow_args") {
        Some(strings) => strings.map(std::string::ToString::to_string).collect(),
        None => vec![],
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
    Ok((
        pick_unused_port().chain_err(|| "No ports free")?,
        pick_unused_port().chain_err(|| "No ports free")?,
        pick_unused_port().chain_err(|| "No ports free")?,
        pick_unused_port().chain_err(|| "No ports free")?,
    ))
}
