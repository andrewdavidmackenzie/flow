#![deny(missing_docs)]
#![warn(clippy::unwrap_used)]
//! `flowide` is a GUI flow runner for running `flow` programs.
//!
//! It reads a compiled [FlowManifest][flowcore::model::flow_manifest::FlowManifest] produced by a
//! flow compiler, such as `flowc`, that describes the graph of communicating functions that
//! constitute the flow program.
//!
//! Use `flowide --help` or `flowide -h` at the command line to see the command line options
//!
//! The [gui] module implements a set of `context functions`, adapted to a GUI runner
//! that allow the flow program to interact with the environment where it is being run.
//!
//! Depending on the command line options supplied `flowide` executes the
//! [Coordinator][flowrlib::coordinator::Coordinator] of flow execution in a background thread,
//! or the [gui::cli_client] in the main thread (where the interaction with STDIO and
//! File System happens) or both. They communicate via network messages using the
//! [SubmissionHandler][flowrlib::submission_handler::SubmissionHandler] to submit flows for execution,
//! and interchanging [ClientMessages][crate::gui::coordinator_message::ClientMessage]
//! and [CoordinatorMessages][crate::gui::coordinator_message::CoordinatorMessage] for execution of context
//! interaction in the client, as requested by functions running in the coordinator's
//! [Executors][flowrlib::executor::Executor]

use core::str::FromStr;
use std::{env, thread};
use std::path::PathBuf;
use std::process::exit;
use std::sync::{Arc, Mutex};

use clap::{Arg, ArgMatches};
use clap::Command as ClapCommand;
use env_logger::Builder;
use log::{info, LevelFilter, trace, warn};
use portpicker::pick_unused_port;
use simpath::Simpath;
use url::Url;

use gui::cli_client::CliRuntimeClient;
use gui::cli_debug_client::CliDebugClient;
use gui::cli_debug_handler::CliDebugHandler;
use gui::cli_submission_handler::CLISubmissionHandler;
use gui::connections::ClientConnection;
use gui::connections::CoordinatorConnection;
//use gui::coordinator_message::ClientMessage;
use gui::debug_message::DebugServerMessage;
use gui::debug_message::DebugServerMessage::*;
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

use crate::gui::connections::{COORDINATOR_SERVICE_NAME, DEBUG_SERVICE_NAME,
                              discover_service, enable_service_discovery};

use iced::{Application, Command, Element, Settings, Theme};
use iced::widget::{button, row, text};

use iced::executor;

use crate::errors::*;
use crate::gui::coordinator_message::ClientMessage;

/// provides the `context functions` for interacting with the execution environment from a flow,
/// plus client-[Coordinator][flowrlib::coordinator::Coordinator] implementations of
/// [flowrlib::submission_handler] for executing them on different threads
/// from the [Coordinator][flowrlib::coordinator::Coordinator]
mod gui;

/// provides [Error][errors::Error] that other modules in this crate will `use crate::errors::*;`
/// to get access to everything `error_chain` creates.
mod errors;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum Message {
    Start,
}

struct FlowIde {
    client: CliRuntimeClient,
    flow_manifest_url: Url,
    flow_args: Vec<String>,
    parallel_jobs_limit: Option<usize>,
}

/// Main for flowide binary - call `run()` and print any error that results or exit silently if OK
fn main() -> crate::errors::Result<()>{
    match FlowIde::run(Settings {
        antialiasing: true,
        ..Settings::default()
    }) {
        Err(ref e) => {
            eprintln!("{e}");
            exit(1);
        }
        Ok(_) => exit(0),
    }
}

impl Application for FlowIde {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let matches = Self::get_matches();

        let default = String::from("error");
        let verbosity = matches.get_one::<String>("verbosity").unwrap_or(&default);
        let level = LevelFilter::from_str(verbosity).unwrap_or(LevelFilter::Error);
        let mut builder = Builder::from_default_env();
        builder.filter_level(level).init();

        info!("'{}' version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        info!("'flowrlib' version {}", flowrlib_info::version());

        let lib_dirs = if matches.contains_id("lib_dir") {
            matches
                .get_many::<String>("lib_dir").unwrap() // TODO
                .map(|s| s.to_string())
                .collect()
        } else {
            vec![]
        };

        let client = Self::client_and_coordinator(
            Self::num_threads(&matches),
            Self::get_lib_search_path(&lib_dirs).unwrap(), // TODO
            matches.get_flag("native"),
            matches.get_flag("debugger"),
        ).unwrap(); // TODO

        let flow_manifest_url =  Self::parse_flow_url(&matches).unwrap(); // TODO
        let flow_args = Self::get_flow_args(&matches, &flow_manifest_url);

        let flowide = FlowIde {
            client,
            flow_manifest_url,
            flow_args,
            parallel_jobs_limit: matches.get_one::<usize>("jobs").map(|i| i.to_owned())
        };

        (flowide, Command::none())
    }

    fn title(&self) -> String {
        String::from("FlowIde")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Start => {
                let _ = self.submit(false);
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        row![
            text(self.flow_manifest_url.to_string()),
            text(format!("{:?}", self.flow_args)),
            button("Play").on_press(Message::Start)
        ].into()
    }
}

impl FlowIde {
    /// For the lib provider, libraries maybe installed in multiple places in the file system.
    /// In order to find the content, a FLOW_LIB_PATH environment variable can be configured with a
    /// list of directories in which to look for the library in question.
    fn get_lib_search_path(search_path_additions: &[String]) -> crate::errors::Result<Simpath> {
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

    /// Start a [Coordinator][flowrlib::coordinator::Coordinator] in a background thread,
    /// then start a client in the calling thread
    fn client_and_coordinator(
        num_threads: usize,
        lib_search_path: Simpath,
        native_flowstdlib: bool,
        debug_this_flow: bool,
    ) -> Result<CliRuntimeClient> {
        let runtime_port = pick_unused_port().chain_err(|| "No ports free")?;
        let coordinator_connection = CoordinatorConnection::new(COORDINATOR_SERVICE_NAME,
                                                                runtime_port)?;

        let discovery_port = pick_unused_port().chain_err(|| "No ports free")?;
        enable_service_discovery(discovery_port, COORDINATOR_SERVICE_NAME, runtime_port)?;

        let debug_port = pick_unused_port().chain_err(|| "No ports free")?;
        let debug_connection = CoordinatorConnection::new(DEBUG_SERVICE_NAME,
                                                          debug_port)?;
        enable_service_discovery(discovery_port, DEBUG_SERVICE_NAME, debug_port)?;

        info!("Starting coordinator in background thread");
        thread::spawn(move || {
            let _ = Self::coordinator(
                num_threads,
                lib_search_path,
                native_flowstdlib,
                coordinator_connection,
                debug_connection,
                false,
            );
        });

        let coordinator_address = discover_service(discovery_port, COORDINATOR_SERVICE_NAME)?;

        Self::client(
            ClientConnection::new(&coordinator_address)?,
            debug_this_flow,
            discovery_port,
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
        debug_connection: CoordinatorConnection,
        loop_forever: bool,
    ) -> Result<()> {
        let connection = Arc::new(Mutex::new(coordinator_connection));

        let mut debug_server = CliDebugHandler { debug_server_connection: debug_connection };

        let provider = Arc::new(MetaProvider::new(lib_search_path,
                                                  PathBuf::from("/"))) as Arc<dyn Provider>;

        let ports = Self::get_four_ports()?;
        trace!("Announcing three job queues and a control socket on ports: {ports:?}");
        let job_queues = Self::get_bind_addresses(ports);
        let dispatcher = Dispatcher::new(job_queues)?;
        enable_service_discovery(JOB_QUEUES_DISCOVERY_PORT, JOB_SERVICE_NAME, ports.0)?;
        enable_service_discovery(JOB_QUEUES_DISCOVERY_PORT, RESULTS_JOB_SERVICE_NAME, ports.2)?;
        enable_service_discovery(JOB_QUEUES_DISCOVERY_PORT, CONTROL_SERVICE_NAME, ports.3)?;

        let (job_source_name, context_job_source_name, results_sink, control_socket) =
            Self::get_connect_addresses(ports);

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
            gui::get_manifest(connection.clone())?,
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
            &mut debug_server
        )?;

        Ok(coordinator.submission_loop(loop_forever)?)
    }

    /// Create the client that talks to the coordinator
    fn client(
        runtime_client_connection: ClientConnection,
        debug_this_flow: bool,
        discovery_post: u16,
    ) -> Result<CliRuntimeClient> {
        trace!("Creating CliRuntimeClient");
        let client = CliRuntimeClient::new(runtime_client_connection);

        if debug_this_flow {
            let debug_server_address = discover_service(discovery_post,
                                                        DEBUG_SERVICE_NAME)?;
            let debug_client_connection = ClientConnection::new(&debug_server_address)?;
            let debug_client = CliDebugClient::new(debug_client_connection,
                                                   client.override_args());
            let _ = thread::spawn(move || {
                debug_client.debug_client_loop();
            });
        }

        Ok(client)
    }

    fn submit(&mut self, debug_this_flow: bool) -> Result<()> {
        let provider = &MetaProvider::new(Simpath::new(""),
                                          PathBuf::default())
            as &dyn Provider;
        let (flow_manifest, _) = FlowManifest::load(provider, &self.flow_manifest_url)?;
        let submission = Submission::new(
            flow_manifest,
            self.parallel_jobs_limit,
            None, // No timeout waiting for job results
            debug_this_flow,
        );

        self.client.set_args(&self.flow_args);
        self.client.set_display_metrics(true);

        info!("Client sending submission to coordinator");
        self.client.send(ClientMessage::ClientSubmission(submission))?;

        trace!("Entering client event loop");
        Ok(self.client.event_loop()?)
    }

    /// Return addresses and ports to be used for each of the three queues
    /// - (general) job source
    /// - context job source
    /// - results sink
    /// - control messages
    fn get_connect_addresses(ports: (u16, u16, u16, u16)) -> (String, String, String, String) {
        (
            format!("tcp://127.0.0.1:{}", ports.0),
            format!("tcp://127.0.0.1:{}", ports.1),
            format!("tcp://127.0.0.1:{}", ports.2),
            format!("tcp://127.0.0.1:{}", ports.3),
        )
    }

    /// Return addresses to bind to for
    /// - (general) job source
    /// - context job source
    /// - results sink
    /// - control messages
    fn get_bind_addresses(ports: (u16, u16, u16, u16)) -> (String, String, String, String) {
        (
            format!("tcp://*:{}", ports.0),
            format!("tcp://*:{}", ports.1),
            format!("tcp://*:{}", ports.2),
            format!("tcp://*:{}", ports.3),
        )
    }

    /// Return four free ports to use for client-coordinator message queues
    fn get_four_ports() -> crate::errors::Result<(u16, u16, u16, u16)> {
        Ok((pick_unused_port().chain_err(|| "No ports free")?,
            pick_unused_port().chain_err(|| "No ports free")?,
            pick_unused_port().chain_err(|| "No ports free")?,
            pick_unused_port().chain_err(|| "No ports free")?,
        ))
    }

    /// Determine the number of threads to use to execute flows
    /// - default (if value is not provided on the command line) of the number of cores
    fn num_threads(matches: &ArgMatches) -> usize {
        match matches.get_one::<usize>("threads") {
            Some(num_threads) => *num_threads,
            None => thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
        }
    }

    /// Parse the command line arguments using clap
    fn get_matches() -> ArgMatches {
        let app = ClapCommand::new(env!("CARGO_PKG_NAME"))
            .version(env!("CARGO_PKG_VERSION"));

        let app = app.arg(
            Arg::new("debugger")
                .short('d')
                .long("debugger")
                .action(clap::ArgAction::SetTrue)
                .help("Enable the debugger when running a flow"),
        );

        #[cfg(not(feature = "wasm"))]
            let app = app.arg(
            Arg::new("native")
                .short('n')
                .long("native")
                .action(clap::ArgAction::SetTrue)
                .help("Link with native (not WASM) version of flowstdlib"),
        );

        let app = app
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
    fn parse_flow_url(matches: &ArgMatches) -> flowcore::errors::Result<Url> {
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
}