#![deny(missing_docs)]
// TODO re-instate #![warn(clippy::unwrap_used)]
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
//! or the [gui::client] in the main thread (where the interaction with STDIO and
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
use iced::{Alignment, alignment, Application, Command, Element, Length, Settings, Theme};
use iced::executor;
use iced::widget::{button, Column, container, Row, text, text_input};
use iced::widget::scrollable::Scrollable;
use log::{info, LevelFilter, trace, warn};
use log::error;
use portpicker::pick_unused_port;
use simpath::Simpath;
use url::Url;

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
use gui::client::Client;
use gui::client_connection::ClientConnection;
use gui::coordinator_connection::CoordinatorConnection;
use gui::debug_client::CliDebugClient;
use gui::debug_handler::CliDebugHandler;
use gui::debug_message::DebugServerMessage;
use gui::debug_message::DebugServerMessage::*;
use gui::submission_handler::CLISubmissionHandler;

use crate::errors::*;
use crate::gui::client_connection::discover_service;
use crate::gui::coordinator_connection::{COORDINATOR_SERVICE_NAME, DEBUG_SERVICE_NAME,
                                         enable_service_discovery};
use crate::gui::coordinator_message::ClientMessage;

/// provides the `context functions` for interacting with the execution environment from a flow,
/// plus client-[Coordinator][flowrlib::coordinator::Coordinator] implementations of
/// [flowrlib::submission_handler] for executing them on different threads
/// from the [Coordinator][flowrlib::coordinator::Coordinator]
mod gui;

/// provides [Error][errors::Error] that other modules in this crate will `use crate::errors::*;`
/// to get access to everything `error_chain` creates.
mod errors;

#[derive(Debug, Clone)]
enum Message {
    Connected(State),
    Start,
    UrlChanged(String),
    FlowArgsChanged(String)
}

enum FlowIde {
    Disconnected,
    Connected(State)
}

#[derive(Debug, Clone)]
struct State {
    coordinator_address: String,
    discovery_port: u16,
    flow_manifest_url: String,
    flow_args: String,
    parallel_jobs_limit: Option<usize>, // TODO read from settings or UI
    debug_this_flow: bool,
}

/// Main for flowide binary - call `run()` and print any error that results or exit silently if OK
fn main() -> Result<()>{
    match FlowIde::run(Settings {
        antialiasing: true,
        ..Settings::default()
    }) {
        Err(ref e) => {
            error!("{e}");
            exit(1);
        }
        Ok(_) => exit(0),
    }
}

fn connecting_message<'a>() -> Element<'a, Message> {
    container(
        text("Connecting...")
            .horizontal_alignment(alignment::Horizontal::Center)
            .size(50),
    )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_y()
        .center_x()
        .into()
}

fn connected(state: &State) -> Element<Message> {
    // .on_submit(), .on_paste(), .width()
    let url = text_input("Flow location (relative, or absolute)",
                         &state.flow_manifest_url)
        .on_input(Message::UrlChanged);
    let args = text_input("Space separated flow arguments",
                          &state.flow_args)
        .on_input(Message::FlowArgsChanged);
    let play = button("Play").on_press(Message::Start);
    let commands = Row::new()
        .spacing(10)
        .align_items(Alignment::End)
        .push(url)
        .push(args)
        .push(play);
    let stdout = text("bla bla bla\nbla bla bla\nbla bla bla\nbla bla bla\n");
    let stdout_col = Column::new().padding(5).push(stdout);
    let stdout_scroll = Scrollable::new(stdout_col);
    let stdout_header = text("STDOUT");
    let stdout_outer = Column::new().padding(5)
        .push(stdout_header)
        .push(stdout_scroll);
    let main = Column::new().spacing(10)
        .push(commands)
        .push(stdout_outer);
    container(main)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(10)
        .into()
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
        .arg(Arg::new("flow-args")
            .num_args(0..)
            .trailing_var_arg(true)
            .help("A list of arguments to pass to the flow."));

    app.get_matches()
}

impl Application for FlowIde {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let matches = get_matches();

        // init logging
        let default = String::from("error");
        let verbosity = matches.get_one::<String>("verbosity").unwrap_or(&default);
        let level = LevelFilter::from_str(verbosity).unwrap_or(LevelFilter::Error);
        let mut builder = Builder::from_default_env();
        builder.filter_level(level).init();

        info!("'{}' version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        info!("'flowrlib' version {}", flowrlib_info::version());

        let lib_dirs = if matches.contains_id("lib_dir") {
            matches
                .get_many::<String>("lib_dir").unwrap() // TODO add to UI
                .map(|s| s.to_string())
                .collect()
        } else {
            vec![]
        };

        let (coordinator_address, discovery_port) = State::start_coordinator(
            State::num_threads(&matches),
            State::get_lib_search_path(&lib_dirs).unwrap(), // TODO
            matches.get_flag("native"),
        ).unwrap(); // TODO

        let flow_manifest_url =  matches.get_one::<String>("flow-manifest")
            .unwrap_or(&"".into()).to_string();
        let flow_args = match matches.get_many::<String>("flow-args") {
            Some(values) => {
                println!("values {:?}", values);
                values.map(|s| s.to_string())
                    .collect::<Vec<String>>().join(" ")
            },
            None => String::new()
        };

        let flowide = FlowIde::Connected(
            State {
                coordinator_address,
                discovery_port,
                flow_manifest_url,
                flow_args,
                parallel_jobs_limit: matches.get_one::<usize>("jobs").map(|i| i.to_owned()),
                debug_this_flow: matches.get_flag("debugger"),
            }
        );

        (flowide, Command::none())
    }

    fn title(&self) -> String {
        String::from("FlowIde")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match self {
            FlowIde::Disconnected => {
                match message {
                    Message::Connected(state) => *self = FlowIde::Connected(state),
                    _ => error!("Unexpected message: {:?} when in Disconnected state", message),
                }
            },
            FlowIde::Connected(state) =>
                match message {
                    Message::Start => {
                        // TODO start as a Command in background and send a Started message
                        let client = State::client(
                            ClientConnection::new(&state.coordinator_address)
                                .unwrap(), // TODO
                            state.debug_this_flow,
                            state.discovery_port,
                        ).unwrap(); // TODO
                        state.submit(client, state.debug_this_flow).unwrap(); // TODO
                    },
                    Message::FlowArgsChanged(value) => state.flow_args = value,
                    Message::UrlChanged(value) => state.flow_manifest_url = value,
                    Message::Connected(_) => error!("Should not get Connected message when in Connected state"),
                }
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        match self {
            FlowIde::Disconnected => connecting_message(),
            FlowIde::Connected(state) => connected(state),
        }
    }
}

impl State {
    /// Create absolute file:// Url for flow location
    fn flow_url(&self) -> flowcore::errors::Result<Url> {
        let cwd_url = Url::from_directory_path(env::current_dir()?)
            .map_err(|_| "Could not form a Url for the current working directory")?;
        url_from_string(&cwd_url, Some(&self.flow_manifest_url))
    }

    /// Create array of strings that are the args to the flow
    fn get_flow_args(&self) -> Vec<String> {
        // arg #0 is the flow url
        let mut flow_args: Vec<String> = vec![self.flow_manifest_url.clone()];
        let additional_args : Vec<String> = self.flow_args.split(' ')
            .map(|s| s.to_string()).collect();
        flow_args.extend(additional_args);
        flow_args
    }

    fn submit(&mut self, mut client: Client, debug_this_flow: bool) -> Result<()> {
        let provider = &MetaProvider::new(Simpath::new(""),
                                          PathBuf::default()) as &dyn Provider;
        let url = self.flow_url()?;
        let (flow_manifest, _) = FlowManifest::load(provider, &url)?;
        let submission = Submission::new(
            flow_manifest,
            self.parallel_jobs_limit,
            None, // No timeout waiting for job results
            debug_this_flow, // TODO move to setting
        );

        let args = self.get_flow_args();
        client.set_args(&args);
        client.set_display_metrics(true);

        info!("Client sending submission to coordinator");
        client.send(ClientMessage::ClientSubmission(submission))?;

        trace!("Entering client event loop");
        Ok(client.event_loop()?)
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

    /// Start a [Coordinator][flowrlib::coordinator::Coordinator] in a background thread,
    /// then start a client in the calling thread
    fn start_coordinator(
        num_threads: usize,
        lib_search_path: Simpath,
        native_flowstdlib: bool,
    ) -> Result<(String, u16)> {
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

        Ok((discover_service(discovery_port, COORDINATOR_SERVICE_NAME)?, discovery_port))
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
        discovery_port: u16,
    ) -> Result<Client> {
        trace!("Creating CliRuntimeClient");
        let client = Client::new(runtime_client_connection);

        if debug_this_flow {
            let debug_server_address = discover_service(discovery_port,
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
    fn get_four_ports() -> Result<(u16, u16, u16, u16)> {
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
}