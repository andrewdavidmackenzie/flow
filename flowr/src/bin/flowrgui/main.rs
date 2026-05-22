#![deny(clippy::unwrap_used, clippy::expect_used)]

//! `flowrgui` is a GUI flow runner for running `flow` programs.
//!
//! It reads a compiled [`FlowManifest`][flowcore::model::flow_manifest::FlowManifest] produced by a
//! flow compiler, such as `flowc`, that describes the graph of communicating functions that
//! constitute the flow program.
//!
//! Use `flowrgui --help` or `flowrgui -h` at the command line to see the command line options
//!
//! The [gui] module implements a set of `context functions`, adapted to a GUI runner
//! that allow the flow program to interact with the environment where it is being run.
//!
//! Depending on the command line options supplied `flowrgui` executes the
//! [`Coordinator`][flowrlib::coordinator::Coordinator] of flow execution in a background thread or
//! connects to an already running coordinator in another process.
//! Application and Coordinator (thread or process) communicate via network messages using the
//! [`SubmissionHandler`][flowrlib::submission_handler::SubmissionHandler] to submit flows for execution,
//! and interchanging [`ClientMessages`][crate::gui::client_message::ClientMessage]
//! and [`CoordinatorMessages`][crate::gui::coordinator_message::CoordinatorMessage] for execution of context
//! interaction in the client, as requested by functions running in the coordinator's
//! [`Executors`][flowrlib::executor::Executor]

use core::str::FromStr;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use std::{env, process, thread};

use clap::Command as ClapCommand;
use clap::{Arg, ArgMatches};
use env_logger::Builder;
use iced::widget::operation::{self, RelativeOffset};
use iced::widget::{center, mouse_area, opaque, stack, text_input, Button, Column, Id, Row, Text};
use iced::{Center, Element, Fill, Subscription, Task};
use iced_aw::Card;
use image::{ImageBuffer, Rgba, RgbaImage};
use log::{debug, info, trace, LevelFilter};
use simpath::Simpath;
use url::Url;

use flowcore::meta_provider::MetaProvider;
use flowcore::model::flow_manifest::FlowManifest;
use flowcore::model::submission::Submission;
use flowcore::provider::Provider;
use flowcore::url_helper::url_from_string;
use flowrlib::info as flowrlib_info;
use gui::coordinator_connection::CoordinatorConnection;
use gui::debug_message::DebugServerMessage;
use gui::debug_message::DebugServerMessage::{
    BlockBreakpoint, DataBreakpoint, ExecutionEnded, ExecutionStarted, ExitingDebugger,
    JobCompleted, JobError, Panic, PriorToSendingJob, Resetting, WaitingForCommand,
};

use crate::gui::client_message::ClientMessage;
use crate::gui::coordinator_message::CoordinatorMessage;
use crate::tabs::TabSet;

/// Include the module that implements the context functions
mod context;

/// provides the `context functions` for interacting with the execution environment from a flow,
/// plus client-[Coordinator][flowrlib::coordinator::Coordinator] implementations of
/// [`flowrlib::submission_handler`] for executing them on different threads
/// from the [`Coordinator`][`flowrlib::coordinator::Coordinator`]
mod gui;

/// module that runs a coordinator in background
mod connection_manager;

/// module with the different UI tabs
mod tabs;

/// provides [Error][errors::Error] that other modules in this crate will `use crate::errors::*;`
/// to get access to everything `error_chain` creates.
mod errors;

/// [Message] enum captures all the types of messages that are sent to and processed by the
/// `flowrgui` Iced Application
#[derive(Debug, Clone)]
pub enum Message {
    /// We lost contact with the coordinator
    CoordinatorDisconnected(String),
    /// The Coordinator sent to the client/App a Coordinator Message
    CoordinatorSent(CoordinatorMessage),
    /// The UI has requested to submit the flow to the Coordinator for execution
    SubmitFlow, // TODO put SubmissionSettings into this variant?
    /// The flow was successfully submitted to the Coordinator
    Submitted,
    /// An error occurred during flow submission
    SubmitError(String),
    /// The Url of the flow to run has been edited by the UI
    UrlChanged(String),
    /// The arguments to send to the flow when executed have been edited by the UI
    FlowArgsChanged(String),
    /// The max parallel jobs setting has been edited by the UI
    MaxJobsChanged(String),
    /// The UI has requested to submit the flow in debug mode
    DebugSubmitFlow,
    /// A different tab of stdio has been selected
    TabSelected(usize),
    /// Text has been entered into STDIN text box
    NewStdin(String),
    /// A new line entered for STDIN
    LineOfStdin(String),
    /// User clicked the EOF button to signal end of stdin
    SendEof,
    /// toggle to auto-scroll to bottom of STDIO has changed
    StdioAutoScrollTogglerChanged(Id, bool),
    /// Request to stop the currently running flow
    StopFlow,
    /// Clear the content of an output tab
    ClearTab(String),
    /// closing of the Modal was requested
    CloseModal,
}

#[allow(clippy::ignored_unit_patterns)]
enum CoordinatorState {
    Disconnected(String),
    Connected(tokio::sync::mpsc::Sender<ClientMessage>),
}

/// Main for flowrgui binary - call `run()` and print any error that results or exit silently if OK
fn main() -> iced::Result {
    iced::application(FlowrGui::new, FlowrGui::update, FlowrGui::view)
        .subscription(FlowrGui::subscription)
        .title(FlowrGui::title)
        .antialiasing(true)
        .run()
}

#[derive(Clone)]
struct SubmissionSettings {
    // TODO make native a UI setting
    // TODO num threads make a UI setting
    // TODO make lib search path a UI setting
    flow_manifest_url: String,
    flow_args: String,
    max_jobs_text: String,
    debug_this_flow: bool,
    display_metrics: bool,
    parallel_jobs_limit: Option<usize>,
}

/// Settings to use when starting a coordinator server
#[derive(Clone)]
pub struct ServerSettings {
    /// Should the coordinator use the natively linked flowstdlib library, or the wasm version
    native_flowstdlib: bool,
    /// How many executor threads should be used
    num_threads: usize,
    /// The path to search for libs when a lib reference is found
    lib_search_path: Simpath,
}

/// [`CoordinatorSettings`] captures the parameters to be used when creating a new Coordinator
#[derive(Clone)]
pub enum CoordinatorSettings {
    /// Start a server coordinator using the settings supplied
    Server(ServerSettings),
    /// Don't start a coordinator server, just discover existing one on this port
    ClientOnly(u16),
}

struct UiSettings {
    auto_start: bool,
    auto_exit: bool,
}

struct ImageReference {
    pub width: u32,
    pub height: u32,
    pub data: ImageBuffer<Rgba<u8>, Vec<u8>>,
}

#[allow(clippy::struct_excessive_bools)]
struct FlowrGui {
    submission_settings: SubmissionSettings,
    coordinator_settings: CoordinatorSettings,
    ui_settings: UiSettings,
    coordinator_state: CoordinatorState,
    tab_set: TabSet,
    running: bool,
    submitted: bool,
    show_modal: bool,
    modal_content: (String, String),
    pending_getline: bool,
}

impl FlowrGui {
    /// Create the flowrgui app and populate fields with options passed on the command line
    fn new() -> (Self, Task<Message>) {
        let settings = FlowrGui::initial_settings();

        let tab_set = TabSet::new();

        let flowrgui = FlowrGui {
            submission_settings: settings.0,
            coordinator_settings: settings.1,
            ui_settings: settings.2,
            coordinator_state: CoordinatorState::Disconnected("Starting".into()),
            tab_set,
            submitted: false,
            running: false,
            show_modal: false,
            modal_content: (String::new(), String::new()),
            pending_getline: false,
        };

        (flowrgui, Task::none())
    }

    #[allow(clippy::unused_self)]
    fn title(&self) -> String {
        String::from("flowrgui")
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::CoordinatorSent(CoordinatorMessage::Connected(sender)) => {
                self.coordinator_state = CoordinatorState::Connected(sender);
                if self.ui_settings.auto_start {
                    return Task::perform(Self::auto_submit(), |()| Message::SubmitFlow);
                }
            }
            Message::SubmitFlow => {
                if let CoordinatorState::Connected(sender) = &self.coordinator_state {
                    return Task::perform(
                        Self::submit(sender.clone(), self.submission_settings.clone()),
                        |result| match result {
                            Ok(()) => Message::Submitted,
                            Err(msg) => Message::SubmitError(msg),
                        },
                    );
                }
            }
            Message::Submitted => {
                self.tab_set.clear();
                self.submitted = true;
            }
            Message::SubmitError(msg) => {
                self.show_modal = true;
                self.modal_content = ("Error".into(), msg);
            }
            Message::StopFlow => {
                connection_manager::request_stop();
            }
            Message::FlowArgsChanged(value) => self.submission_settings.flow_args = value,
            Message::MaxJobsChanged(value) => {
                self.submission_settings.parallel_jobs_limit = value.trim().parse::<usize>().ok();
                self.submission_settings.max_jobs_text = value;
            }
            Message::DebugSubmitFlow => {
                if let CoordinatorState::Connected(sender) = &self.coordinator_state {
                    let mut settings = self.submission_settings.clone();
                    settings.debug_this_flow = true;
                    return Task::perform(Self::submit(sender.clone(), settings), |result| {
                        match result {
                            Ok(()) => Message::Submitted,
                            Err(msg) => Message::SubmitError(msg),
                        }
                    });
                }
            }
            Message::UrlChanged(value) => self.submission_settings.flow_manifest_url = value,
            Message::TabSelected(_)
            | Message::StdioAutoScrollTogglerChanged(_, _)
            | Message::ClearTab(_) => {
                return self.tab_set.update(message);
            }
            Message::CoordinatorSent(coord_msg) => {
                return self.process_coordinator_message(coord_msg);
            }
            Message::CloseModal => self.show_modal = false,
            Message::CoordinatorDisconnected(reason) => {
                self.coordinator_state = CoordinatorState::Disconnected(reason);
            }
            Message::NewStdin(text) => self.tab_set.stdin_tab.text_entered(text),
            Message::LineOfStdin(line) => {
                debug!("LineOfStdin: user entered line ({} chars)", line.len());
                self.tab_set.stdin_tab.new_line(line);
                if self.pending_getline {
                    if let Some(line) = self.tab_set.stdin_tab.get_line() {
                        debug!(
                            "LineOfStdin: responding to pending GetLine ({} chars)",
                            line.len()
                        );
                        self.send(ClientMessage::Line(line));
                    }
                    self.pending_getline = false;
                    self.tab_set.stdin_tab.waiting_for_input = false;
                }
            }
            Message::SendEof => {
                debug!("SendEof: user clicked EOF button");
                if self.pending_getline {
                    debug!("SendEof: responding to pending GetLine with EOF");
                    self.send(ClientMessage::GetLineEof);
                    self.pending_getline = false;
                    self.tab_set.stdin_tab.waiting_for_input = false;
                } else {
                    self.tab_set.stdin_tab.eof_signaled = true;
                }
            }
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let main_content = Column::new()
            .spacing(10)
            .push(self.command_row())
            .push(self.tab_set.view())
            .push(self.status_row())
            .padding(10);

        if self.show_modal {
            let modal_card = Card::new(
                Text::new(self.modal_content.clone().0),
                Text::new(self.modal_content.clone().1),
            )
            .foot(
                Row::new().spacing(10).padding(5).width(Fill).push(
                    Button::new(Text::new("OK").align_x(Center))
                        .width(Fill)
                        .on_press(Message::CloseModal),
                ),
            )
            .max_width(300.0);

            stack![
                main_content,
                opaque(mouse_area(center(opaque(modal_card))).on_press(Message::CloseModal))
            ]
            .into()
        } else {
            main_content.into()
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        connection_manager::subscribe(self.coordinator_settings.clone())
            .map(Message::CoordinatorSent)
    }
}

impl FlowrGui {
    #[allow(clippy::unused_async)]
    async fn auto_submit() {
        info!("Auto submitting flow");
    }

    // Submit the flow to the coordinator for execution
    async fn submit(
        sender: tokio::sync::mpsc::Sender<ClientMessage>,
        settings: SubmissionSettings,
    ) -> Result<(), String> {
        let url = Self::flow_url(&settings.flow_manifest_url)
            .map_err(|e| format!("Invalid flow URL '{}': {e}", settings.flow_manifest_url))?;

        let provider = &MetaProvider::new(Simpath::new(""), PathBuf::default()) as &dyn Provider;

        let (flow_manifest, _) = FlowManifest::load(provider, &url)
            .map_err(|e| format!("Could not load flow manifest: {e}"))?;

        let submission = Submission::new(
            flow_manifest,
            settings.parallel_jobs_limit,
            None,
            settings.debug_this_flow,
        );

        info!("Sending submission to Coordinator");
        sender
            .send(ClientMessage::ClientSubmission(submission))
            .await
            .map_err(|e| format!("Could not submit flow to coordinator: {e}"))
    }

    fn error(&mut self, msg: &str) {
        self.show_modal = true;
        self.modal_content = ("Error".into(), msg.to_string());
    }

    fn info(&mut self, msg: &str) {
        self.show_modal = true;
        self.modal_content = ("Info".into(), msg.to_string());
    }

    fn command_row(&self) -> Row<'_, Message> {
        let url = text_input(
            "Flow location (relative, or absolute)",
            &self.submission_settings.flow_manifest_url,
        )
        .on_input(Message::UrlChanged)
        .on_submit(Message::SubmitFlow);

        let args = text_input(
            "Space separated flow arguments",
            &self.submission_settings.flow_args,
        )
        .on_submit(Message::SubmitFlow)
        .on_input(Message::FlowArgsChanged)
        .on_paste(Message::FlowArgsChanged);

        let max_jobs = text_input("Max jobs", &self.submission_settings.max_jobs_text)
            .on_input(Message::MaxJobsChanged)
            .width(80);

        let can_run = matches!(self.coordinator_state, CoordinatorState::Connected(_))
            && !self.running
            && !self.submitted;

        let play = if self.running {
            Button::new("Stop").on_press(Message::StopFlow)
        } else {
            let mut btn = Button::new("Play");
            if can_run {
                btn = btn.on_press(Message::SubmitFlow);
            }
            btn
        };

        let mut debug_play = Button::new("Debug");
        if can_run {
            debug_play = debug_play.on_press(Message::DebugSubmitFlow);
        }

        Row::new()
            .spacing(10)
            .align_y(iced::alignment::Vertical::Bottom)
            .push(url)
            .push(args)
            .push(max_jobs)
            .push(play)
            .push(debug_play)
    }

    fn status_row(&self) -> Row<'_, Message> {
        let status = match &self.coordinator_state {
            CoordinatorState::Disconnected(reason) => format!("Disconnected({reason})"),
            CoordinatorState::Connected(_) => {
                let msg = match (self.submitted, self.running) {
                    (false, false) => "Ready",
                    (_, true) => "Running",
                    (true, false) => "Submitted",
                };
                format!("Connected({msg})")
            }
        };

        Row::new().push(Text::new(format!("Coordinator: {status}")))
    }

    // Create initial Settings structs for Submission and Coordinator from the CLI options
    fn initial_settings() -> (SubmissionSettings, CoordinatorSettings, UiSettings) {
        let matches = Self::parse_cli_args();

        // init logging
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

        let flow_manifest_url = matches
            .get_one::<String>("flow-manifest")
            .unwrap_or(&String::new())
            .clone();
        let flow_args = match matches.get_many::<String>("flow-args") {
            Some(values) => {
                println!("values {values:?}");
                values
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<String>>()
                    .join(" ")
            }
            None => String::new(),
        };

        // TODO read from settings or UI
        let parallel_jobs_limit = matches
            .get_one::<usize>("jobs")
            .map(std::borrow::ToOwned::to_owned);

        // TODO make a UI setting
        let debug_this_flow = matches.get_flag("debugger");

        let coordinator_settings = if let Some(port) = matches.get_one::<u16>("client") {
            CoordinatorSettings::ClientOnly(*port)
        } else {
            let lib_dirs = if matches.contains_id("lib_dir") {
                if let Some(dirs) = matches.get_many::<String>("lib_dir") {
                    dirs.map(std::string::ToString::to_string).collect()
                } else {
                    vec![]
                }
            } else {
                vec![]
            };

            let lib_search_path = FlowrGui::lib_search_path(&lib_dirs);

            let native_flowstdlib = matches.get_flag("native");

            let num_threads = FlowrGui::num_threads(&matches);

            let server_settings = ServerSettings {
                native_flowstdlib,
                num_threads,
                lib_search_path,
            };

            CoordinatorSettings::Server(server_settings)
        };

        let auto = matches.get_flag("auto");
        let auto_start = auto || matches.get_flag("auto-start");

        (
            SubmissionSettings {
                flow_manifest_url,
                flow_args,
                max_jobs_text: parallel_jobs_limit.map_or(String::new(), |n| n.to_string()),
                debug_this_flow,
                display_metrics: matches.get_flag("metrics"),
                parallel_jobs_limit,
            },
            coordinator_settings,
            UiSettings {
                auto_start,
                auto_exit: auto,
            },
        )
    }

    // Parse the command line arguments using clap
    fn parse_cli_args() -> ArgMatches {
        let app = ClapCommand::new(env!("CARGO_PKG_NAME")).version(env!("CARGO_PKG_VERSION"));

        let app = app.arg(
            Arg::new("debugger")
                .short('d')
                .long("debugger")
                .action(clap::ArgAction::SetTrue)
                .help("Enable the debugger when running a flow"),
        );

        #[cfg(feature = "flowstdlib")]
        let app = app.arg(
            Arg::new("native")
                .short('n')
                .long("native")
                .action(clap::ArgAction::SetTrue)
                .help("Link with native (not WASM) version of flowstdlib"),
        );

        let app = app.arg(
            Arg::new("client")
                .short('c')
                .long("client")
                .number_of_values(1)
                .value_parser(clap::value_parser!(u16))
                .help("Launch only a client (no coordinator) to connect to a remote coordinator"),
        );

        let app = app.arg(
            Arg::new("metrics")
                .short('m')
                .long("metrics")
                .action(clap::ArgAction::SetTrue)
                .help("Calculate metrics during flow execution and print them out when done"),
        );

        let app = app.arg(
            Arg::new("auto")
                .short('a')
                .long("auto")
                .action(clap::ArgAction::SetTrue)
                .help("Run any flow specified automatically on start-up. Exit automatically."),
        );

        let app = app.arg(
            Arg::new("auto-start")
                .long("auto-start")
                .action(clap::ArgAction::SetTrue)
                .help("Run the flow automatically on start-up, but stay open for interaction."),
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

    // Create absolute file:// Url for flow location - using the contents of UI field
    fn flow_url(flow_url_string: &str) -> flowcore::errors::Result<Url> {
        let cwd_url = Url::from_directory_path(env::current_dir()?)
            .map_err(|()| "Could not form a Url for the current working directory")?;
        url_from_string(&cwd_url, Some(flow_url_string))
    }

    // Create array of strings that are the args to the flow
    fn flow_arg_vec(&self) -> Vec<String> {
        // arg #0 is the flow url
        let mut flow_args: Vec<String> = vec![self.submission_settings.flow_manifest_url.clone()];
        let additional_args: Vec<String> = self
            .submission_settings
            .flow_args
            .split(' ')
            .map(std::string::ToString::to_string)
            .collect();
        flow_args.extend(additional_args);
        flow_args
    }

    // For the lib provider, libraries maybe installed in multiple places in the file system.
    // In order to find the content, a FLOW_LIB_PATH environment variable can be configured with a
    // list of directories in which to look for the library in question.
    fn lib_search_path(search_path_additions: &[String]) -> Simpath {
        let mut lib_search_path = Simpath::new_with_separator("FLOW_LIB_PATH", ',');

        for additions in search_path_additions {
            lib_search_path.add(additions);
            info!("'{additions}' added to the Library Search Path");
        }

        if lib_search_path.is_empty() {
            let home_dir = env::var("HOME").unwrap_or_else(|_| "Could not get $HOME".to_string());
            lib_search_path.add(&format!("{home_dir}/.flow/lib"));
        }

        lib_search_path
    }

    // Determine the number of threads to use to execute flows
    // - default (if value is not provided on the command line) of the number of cores
    #[allow(clippy::redundant_closure_for_method_calls)]
    fn num_threads(matches: &ArgMatches) -> usize {
        match matches.get_one::<usize>("threads") {
            Some(num_threads) => *num_threads,
            // Could be simplified to `std::num::NonZero::get`but generic NonZero is unstable
            None => thread::available_parallelism().map_or(1, |n| n.get()),
        }
    }

    fn send(&mut self, msg: ClientMessage) {
        if let CoordinatorState::Connected(ref sender) = self.coordinator_state {
            let _ = sender.try_send(msg);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn process_coordinator_message(&mut self, message: CoordinatorMessage) -> Task<Message> {
        match message {
            CoordinatorMessage::Connected(_) => {
                self.error("Coordinator is already connected");
            }
            CoordinatorMessage::FlowStart => {
                debug!("FlowStart received");
                self.running = true;
                self.submitted = false;
                self.send(ClientMessage::Ack);
            }
            CoordinatorMessage::FlowEnd(metrics) => {
                debug!("FlowEnd received");
                self.running = false;
                self.pending_getline = false;
                self.tab_set.stdin_tab.waiting_for_input = false;
                if self.submission_settings.display_metrics {
                    self.show_modal = true;
                    self.modal_content = ("Flow Ended - Metrics".into(), format!("{metrics}"));
                }
                // NO response - so we can use next request sent to submit another flow
                if self.ui_settings.auto_exit {
                    self.info("Auto exiting on flow completion");
                    let _ = std::io::stdout().flush();
                    process::exit(0);
                }
            }
            CoordinatorMessage::CoordinatorExiting(_) => {
                self.coordinator_state = CoordinatorState::Disconnected("Exited".into());
                self.send(ClientMessage::Ack);
            }
            CoordinatorMessage::Stdout(string) => {
                if self.ui_settings.auto_exit {
                    println!("{string}");
                }
                self.tab_set.stdout_tab.content.push(string);
                if self.tab_set.active_tab != 0 {
                    self.tab_set.stdout_tab.unread_count += 1;
                }
                self.send(ClientMessage::Ack);
                if self.tab_set.stdout_tab.auto_scroll {
                    return operation::snap_to(
                        self.tab_set.stdout_tab.id.clone(),
                        RelativeOffset::END,
                    );
                }
            }
            CoordinatorMessage::Stderr(string) => {
                if self.ui_settings.auto_exit {
                    eprintln!("{string}");
                }
                self.tab_set.stderr_tab.content.push(string);
                if self.tab_set.active_tab != 1 {
                    self.tab_set.stderr_tab.unread_count += 1;
                }
                self.send(ClientMessage::Ack);
                if self.tab_set.stderr_tab.auto_scroll {
                    return operation::snap_to(
                        self.tab_set.stderr_tab.id.clone(),
                        RelativeOffset::END,
                    );
                }
            }
            CoordinatorMessage::GetStdin => {
                debug!(
                    "GetStdin received, buffer has {} lines, cursor at {}",
                    self.tab_set.stdin_tab.content.len(),
                    self.tab_set.stdin_tab.cursor
                );
                // In auto mode, read all remaining process stdin when buffer is empty
                if self.ui_settings.auto_exit
                    && self.tab_set.stdin_tab.cursor >= self.tab_set.stdin_tab.content.len()
                {
                    let stdin = std::io::stdin();
                    for line in stdin.lock().lines().map_while(Result::ok) {
                        self.tab_set.stdin_tab.new_line(line);
                    }
                }
                let msg = if let Some(buf) = self.tab_set.stdin_tab.get_all() {
                    debug!("GetStdin: returning buffered content ({} bytes)", buf.len());
                    ClientMessage::Stdin(buf)
                } else {
                    debug!("GetStdin: buffer empty, sending GetStdinEof");
                    ClientMessage::GetStdinEof
                };
                self.send(msg);
            }
            CoordinatorMessage::GetLine(_prompt) => {
                debug!(
                    "GetLine received, buffer has {} lines, cursor at {}",
                    self.tab_set.stdin_tab.content.len(),
                    self.tab_set.stdin_tab.cursor
                );
                // In auto mode, read a line from process stdin when buffer is empty
                if self.ui_settings.auto_exit
                    && self.tab_set.stdin_tab.cursor >= self.tab_set.stdin_tab.content.len()
                {
                    let mut input = String::new();
                    match std::io::stdin().lock().read_line(&mut input) {
                        Ok(n) if n > 0 => {
                            self.tab_set.stdin_tab.new_line(input.trim().to_string());
                        }
                        _ => {} // EOF or error — buffer stays empty, will send GetLineEof
                    }
                }
                if let Some(line) = self.tab_set.stdin_tab.get_line() {
                    trace!("GetLine: returning buffered line: '{line}'");
                    debug!("GetLine: returning buffered line ({} chars)", line.len());
                    self.send(ClientMessage::Line(line));
                } else if self.ui_settings.auto_exit || self.tab_set.stdin_tab.eof_signaled {
                    debug!("GetLine: EOF (auto mode or user signaled)");
                    self.send(ClientMessage::GetLineEof);
                    self.tab_set.stdin_tab.eof_signaled = false;
                } else {
                    debug!("GetLine: buffer empty, waiting for user input");
                    self.pending_getline = true;
                    self.tab_set.stdin_tab.waiting_for_input = true;
                    self.tab_set.active_tab = 2; // Switch to Stdin tab
                }
            }
            CoordinatorMessage::GetArgs => {
                let args = self.flow_arg_vec();
                let msg = ClientMessage::Args(args);
                self.send(msg);

                /* Override args for the debugger to use
                if let Ok(override_args) = self.override_args.lock() {
                    if override_args.is_empty() {
                        ClientMessage::Args(self.args.clone())
                    } else {
                        // we want to retain arg[0] which is the flow name and replace  all others
                        // with the override args supplied
                        let mut one_time_args = vec!(self.args[0].clone());
                        one_time_args.append(&mut override_args.to_vec());
                        ClientMessage::Args(one_time_args)
                    }
                } else {
                    ClientMessage::Args(self.args.clone())
                }
                */
            }
            CoordinatorMessage::Read(file_path) => {
                // TODO list file reads and write in the UI somewhere
                let msg = match File::open(&file_path) {
                    Ok(mut f) => {
                        let mut buffer = Vec::new();
                        match f.read_to_end(&mut buffer) {
                            Ok(_) => {
                                self.tab_set
                                    .fileio_tab
                                    .content
                                    .push(format!("READ <-- {file_path}"));
                                if self.tab_set.active_tab != 4 {
                                    self.tab_set.fileio_tab.unread_count += 1;
                                }
                                /*
                                                               if self.tab_set.stdout_tab.auto_scroll {
                                                                   return scrollable::snap_to(
                                                                       self.tab_set.stdout_tab.id.clone(), scrollable::RelativeOffset::END);
                                                               }
                                */
                                ClientMessage::FileContents(file_path, buffer)
                            }
                            Err(e) => {
                                let msg = format!("Could not read content from '{file_path}': {e}");
                                self.error(&msg);
                                ClientMessage::Error(msg)
                            }
                        }
                    }
                    Err(e) => {
                        let msg = format!("Could not open file '{file_path}': {e}");
                        self.error(&msg);
                        ClientMessage::Error(msg)
                    }
                };
                self.send(msg);
            }
            CoordinatorMessage::Write(filename, bytes) => {
                let msg = match File::create(&filename) {
                    Ok(mut file) => match file.write_all(bytes.as_slice()) {
                        Ok(()) => {
                            self.tab_set
                                .fileio_tab
                                .content
                                .push(format!("WRITE --> {filename}"));
                            if self.tab_set.active_tab != 4 {
                                self.tab_set.fileio_tab.unread_count += 1;
                            }
                            /*
                                                           if self.tab_set.stdout_tab.auto_scroll {
                                                               return scrollable::snap_to(
                                                                   self.tab_set.stdout_tab.id.clone(), scrollable::RelativeOffset::END);
                                                           }
                            */

                            ClientMessage::Ack
                        }
                        Err(e) => {
                            let msg = format!("Error writing to file: '{filename}': '{e}'");
                            self.error(&msg);
                            ClientMessage::Error(msg)
                        }
                    },
                    Err(e) => {
                        let msg = format!("Error creating file: '{filename}': '{e}'");
                        self.error(&msg);
                        ClientMessage::Error(msg)
                    }
                };
                self.send(msg);
            }
            CoordinatorMessage::PixelWrite(
                (x_coord, y_coord),
                (red, green, blue),
                (width, height),
                ref name,
            ) => {
                if self.tab_set.images_tab.images.is_empty() {
                    let data = RgbaImage::new(width, height);
                    self.tab_set.images_tab.images.insert(
                        name.clone(),
                        ImageReference {
                            width,
                            height,
                            data,
                        },
                    );
                    if self.tab_set.active_tab != 3 {
                        self.tab_set.images_tab.new_activity = true;
                    }
                }
                if let Some(ImageReference {
                    width: _,
                    height: _,
                    ref mut data,
                }) = &mut self.tab_set.images_tab.images.get_mut(name)
                {
                    data.put_pixel(x_coord, y_coord, Rgba([red, green, blue, 255]));
                }
                self.send(ClientMessage::Ack);
            }
            CoordinatorMessage::StdoutEof => {
                trace!("StdoutEof received");
                self.send(ClientMessage::Ack);
            }
            CoordinatorMessage::StderrEof => {
                trace!("StderrEof received");
                self.send(ClientMessage::Ack);
            }
            _ => {}
        }
        Task::none()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use super::*;

    #[test]
    fn flow_url_absolute_path() {
        let url = FlowrGui::flow_url("/tmp/test.toml").expect("Could not create url");
        assert_eq!(url.scheme(), "file");
        assert!(url.path().ends_with("/tmp/test.toml"));
    }

    #[test]
    fn flow_url_relative_path() {
        let url = FlowrGui::flow_url("test.toml").expect("Could not create url");
        assert_eq!(url.scheme(), "file");
        assert!(url.path().ends_with("/test.toml"));
    }

    fn test_gui() -> FlowrGui {
        FlowrGui {
            submission_settings: SubmissionSettings {
                flow_manifest_url: String::new(),
                flow_args: String::new(),
                max_jobs_text: String::new(),
                debug_this_flow: false,
                display_metrics: false,
                parallel_jobs_limit: None,
            },
            coordinator_settings: CoordinatorSettings::ClientOnly(0),
            ui_settings: UiSettings {
                auto_start: false,
                auto_exit: false,
            },
            coordinator_state: CoordinatorState::Disconnected("test".into()),
            tab_set: TabSet::new(),
            submitted: false,
            running: false,
            show_modal: false,
            modal_content: (String::new(), String::new()),
            pending_getline: false,
        }
    }

    #[test]
    fn max_jobs_valid_number() {
        let mut gui = test_gui();
        drop(gui.update(Message::MaxJobsChanged("4".into())));
        assert_eq!(gui.submission_settings.parallel_jobs_limit, Some(4));
        assert_eq!(gui.submission_settings.max_jobs_text, "4");
    }

    #[test]
    fn max_jobs_empty_clears() {
        let mut gui = test_gui();
        drop(gui.update(Message::MaxJobsChanged("4".into())));
        drop(gui.update(Message::MaxJobsChanged(String::new())));
        assert_eq!(gui.submission_settings.parallel_jobs_limit, None);
        assert_eq!(gui.submission_settings.max_jobs_text, "");
    }

    #[test]
    fn max_jobs_invalid_sets_none() {
        let mut gui = test_gui();
        drop(gui.update(Message::MaxJobsChanged("abc".into())));
        assert_eq!(gui.submission_settings.parallel_jobs_limit, None);
        assert_eq!(gui.submission_settings.max_jobs_text, "abc");
    }

    #[test]
    fn debug_submit_without_coordinator_is_noop() {
        let mut gui = test_gui();
        assert!(!gui.submission_settings.debug_this_flow);
        drop(gui.update(Message::DebugSubmitFlow));
        assert!(!gui.submitted);
    }

    #[test]
    fn submit_error_shows_modal() {
        let mut gui = test_gui();
        assert!(!gui.show_modal);
        drop(gui.update(Message::SubmitError("test error".into())));
        assert!(gui.show_modal);
        assert_eq!(gui.modal_content.0, "Error");
        assert_eq!(gui.modal_content.1, "test error");
    }

    #[test]
    fn close_modal_hides_it() {
        let mut gui = test_gui();
        drop(gui.update(Message::SubmitError("test error".into())));
        assert!(gui.show_modal);
        drop(gui.update(Message::CloseModal));
        assert!(!gui.show_modal);
    }

    #[test]
    fn error_method_shows_modal() {
        let mut gui = test_gui();
        gui.error("something went wrong");
        assert!(gui.show_modal);
        assert_eq!(gui.modal_content.0, "Error");
        assert_eq!(gui.modal_content.1, "something went wrong");
    }

    #[test]
    fn error_modal_renders_with_ok_button() {
        use iced_test::simulator::simulator;

        let mut gui = test_gui();
        drop(gui.update(Message::SubmitError("bad flow path".into())));
        let view = gui.view();
        let mut ui = simulator(view);
        let found = ui.find("OK");
        assert!(found.is_ok(), "OK button should be present in error modal");
    }
}
