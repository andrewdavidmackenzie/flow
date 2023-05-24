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
use std::process::exit;

use clap::{Arg, ArgMatches};
use clap::Command as ClapCommand;
use env_logger::Builder;
use iced::{Alignment, alignment, Application, Command, Element, Length, Settings, Subscription, Theme};
use iced::executor;
use iced::widget::{button, Column, container, Row, text, text_input};
use iced::widget::scrollable::Scrollable;
use log::{info, LevelFilter, warn};
use log::error;
use simpath::Simpath;
use url::Url;

use flowcore::url_helper::url_from_string;
use flowrlib::info as flowrlib_info;
use gui::client_connection::ClientConnection;
use gui::coordinator_connection::CoordinatorConnection;
use gui::debug_message::DebugServerMessage;
use gui::debug_message::DebugServerMessage::*;

use crate::coordinator::GuiCoordinator;
use crate::errors::*;
use crate::gui::client::Client;
use crate::gui::coordinator_message::CoordinatorMessage;

/// provides the `context functions` for interacting with the execution environment from a flow,
/// plus client-[Coordinator][flowrlib::coordinator::Coordinator] implementations of
/// [flowrlib::submission_handler] for executing them on different threads
/// from the [Coordinator][flowrlib::coordinator::Coordinator]
mod gui;

/// module that runs a coordinator in background
mod coordinator;

/// provides [Error][errors::Error] that other modules in this crate will `use crate::errors::*;`
/// to get access to everything `error_chain` creates.
mod errors;

#[allow(dead_code)] // TODO send Coordinator found message and handle coordinator messages
#[derive(Debug, Clone)]
enum Message {
    CoordinatorFound((String, u16)), // coordinator_address, discovery_port
    SubmitFlow,
    UrlChanged(String),
    FlowArgsChanged(String),
    Coordinator(CoordinatorMessage), // Message received from Coordinator
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

fn not_connected<'a>() -> Element<'a, Message> {
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

struct FlowIde {
    flow_manifest_url: String,
    flow_args: String,
    parallel_jobs_limit: Option<usize>, // TODO read from settings or UI
    debug_this_flow: bool,
    native_flowstdlib: bool,
    num_threads: usize,
    #[allow(dead_code)]
    lib_dirs: Vec<String>,
    gui_coordinator: GuiCoordinator,
    stdout: Vec<String>,
}

// Implement the iced Application trait for FlowIde
impl Application for FlowIde {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    /// Create the FlowIde app and populate fields with options passed on the command line
    fn new(_flags: ()) -> (Self, Command<Message>) {
        let matches = Self::parse_cli_args();

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

        let lib_search_path = FlowIde::lib_search_path(&lib_dirs).unwrap(); // TODO

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

        // TODO read from settings or UI
        let parallel_jobs_limit = matches.get_one::<usize>("jobs").map(|i| i.to_owned());

        // TODO make a UI setting
        let debug_this_flow = matches.get_flag("debugger");

        // TODO make a UI setting
        let native_flowstdlib = matches.get_flag("native");

        // TODO make a UI setting
        let num_threads = FlowIde::num_threads(&matches);

        let mut flowide = FlowIde {
            flow_manifest_url,
            flow_args,
            parallel_jobs_limit,
            debug_this_flow,
            native_flowstdlib,
            lib_dirs,
            num_threads,
            gui_coordinator: GuiCoordinator::Unknown,
            stdout: Vec::new(),
        };

        flowide.gui_coordinator = GuiCoordinator::Found(coordinator::start(flowide.num_threads,
                                                                           lib_search_path,
                                                                           flowide.native_flowstdlib));

        // TODO ability to connect to an already running coordinator. Maybe try and detect one?
        (flowide, Command::none())
//         Command::perform(coordinator::start(num_threads,
//                                             lib_search_path,
//                                             native_flowstdlib), |_| Message::CoordinatorFound))
    }

    fn title(&self) -> String {
        String::from("FlowIde")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match self.gui_coordinator {
            GuiCoordinator::Unknown => {
                match message {
                    Message::CoordinatorFound(coordinator_info) => self.gui_coordinator = GuiCoordinator::Found(coordinator_info),
                    _ => error!("Unexpected message: {:?} when in Disconnected state", message),
                }
            },
            GuiCoordinator::Found(ref coordinator_info) =>
                match message {
                    Message::SubmitFlow => {
                        // TODO start as a Command in background and send a Started message

                        let client_connection = ClientConnection::new(&coordinator_info.0)
                            .unwrap(); // TODO

                        let mut client = Client::new(client_connection);
                        client.set_args(&self.flow_arg_vec());
                        client.set_display_metrics(true);

                        if self.debug_this_flow {
                            let _ = GuiCoordinator::debug_client(client.override_args(),
                                                                 coordinator_info.1);
                        }

                        let url = self.flow_url().unwrap(); // TODO
                        // Submit the flow to the coordinator for execution using the
                        let _ = GuiCoordinator::submit(client,
                                                        url,
                                                  self.parallel_jobs_limit,
                                                  self.debug_this_flow); // TODO
                    },
                    Message::FlowArgsChanged(value) => self.flow_args = value,
                    Message::UrlChanged(value) => self.flow_manifest_url = value,
                    Message::CoordinatorFound(_) => error!("Unexpected Message CoordinatorFound"),
                    Message::Coordinator(coord_msg) => {
                        self.process_coordinator_message(coord_msg)
                    }
                }
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        // .on_submit(), .on_paste(), .width()
        let url = text_input("Flow location (relative, or absolute)",
                             &self.flow_manifest_url)
            .on_input(Message::UrlChanged);
        let args = text_input("Space separated flow arguments",
                              &self.flow_args)
            .on_input(Message::FlowArgsChanged);
        // TODO disable until loaded flow
        let play = button("Play").on_press(Message::SubmitFlow);
        let commands = Row::new()
            .spacing(10)
            .align_items(Alignment::End)
            .push(url)
            .push(args)
            .push(play);

        let coordinator = match &self.gui_coordinator {
            GuiCoordinator::Unknown => not_connected(),
            GuiCoordinator::Found(coordinator_info) => self.connected(coordinator_info),
        };

        let main = Column::new().spacing(10)
            .push(commands)
            .push(coordinator);
        container(main)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(10)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(1000)).map(|_| {
            Message::Coordinator(CoordinatorMessage::Stdout("Tick".into()))
        })
    }

    // TODO
//    fn subscription(&self) -> Subscription<Message> {
//        coordinator::connect().map(Message::CoordinatorMessage)
//    }
}

// TODO move to a settings struct?
impl FlowIde {
    fn connected<'a>(&self, _coordinator_info: &(String, u16)) -> Element<'a, Message> {
        let stdout_col = Column::with_children(
            self.stdout
                .iter()
                .cloned()
                .map(text)
                .map(Element::from)
                .collect(),
        ).padding(1);
        let stdout_scroll = Scrollable::new(stdout_col);
        let stdout_header = text("STDOUT");
        Column::new().padding(5)
            .push(stdout_header)
            .push(stdout_scroll).into()
    }

    /// Parse the command line arguments using clap
    fn parse_cli_args() -> ArgMatches {
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

    /// Create absolute file:// Url for flow location - using the contents of UI field
    fn flow_url(&self) -> flowcore::errors::Result<Url> {
        let cwd_url = Url::from_directory_path(env::current_dir()?)
            .map_err(|_| "Could not form a Url for the current working directory")?;
        url_from_string(&cwd_url, Some(&self.flow_manifest_url))
    }

    /// Create array of strings that are the args to the flow
    fn flow_arg_vec(&self) -> Vec<String> {
        // arg #0 is the flow url
        let mut flow_args: Vec<String> = vec![self.flow_manifest_url.clone()];
        let additional_args : Vec<String> = self.flow_args.split(' ')
            .map(|s| s.to_string()).collect();
        flow_args.extend(additional_args);
        flow_args
    }

    /// For the lib provider, libraries maybe installed in multiple places in the file system.
    /// In order to find the content, a FLOW_LIB_PATH environment variable can be configured with a
    /// list of directories in which to look for the library in question.
    fn lib_search_path(search_path_additions: &[String]) -> Result<Simpath> {
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

    /// Determine the number of threads to use to execute flows
    /// - default (if value is not provided on the command line) of the number of cores
    fn num_threads(matches: &ArgMatches) -> usize {
        match matches.get_one::<usize>("threads") {
            Some(num_threads) => *num_threads,
            None => thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
        }
    }

    fn process_coordinator_message(&mut self, message: CoordinatorMessage) {
        match message {
            CoordinatorMessage::FlowStart => {}
            CoordinatorMessage::FlowEnd(metrics) => {println!("{}", metrics)}
            CoordinatorMessage::CoordinatorExiting(_) => {}
            CoordinatorMessage::Stdout(string) => self.stdout.push(string),
            CoordinatorMessage::Stderr(_) => {}
            CoordinatorMessage::GetStdin => {}
            CoordinatorMessage::GetLine(_) => {}
            CoordinatorMessage::GetArgs => {}
            CoordinatorMessage::Read(_) => {}
            CoordinatorMessage::Write(_, _) => {}
            CoordinatorMessage::PixelWrite(_, _, _, _) => {}
            CoordinatorMessage::StdoutEof => {}
            CoordinatorMessage::StderrEof => {}
            CoordinatorMessage::Invalid => {}
        }

    }
}