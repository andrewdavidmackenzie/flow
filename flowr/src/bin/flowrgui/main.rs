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
use iced::alignment::Horizontal;
use iced::executor;
use iced::widget::scrollable::Id;
use iced::widget::{scrollable, text_input, Button, Column, Row, Text};
use iced::{Alignment, Application, Command, Element, Length, Settings, Subscription, Theme};
use iced_aw::{modal, Card};
use image::{ImageBuffer, Rgba, RgbaImage};
use log::{info, LevelFilter};
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
    /// ddd
    Submitted,
    /// The Url of the flow to run has been edited by the UI
    UrlChanged(String),
    /// The arguments to send to the flow when executed have been edited by the UI
    FlowArgsChanged(String),
    /// A different tab of stdio has been selected
    TabSelected(usize),
    /// Text has been entered into STDIN text box
    NewStdin(String),
    /// A new line entered for STDIN
    LineOfStdin(String),
    /// toggle to auto-scroll to bottom of STDIO has changed
    StdioAutoScrollTogglerChanged(Id, bool),
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
    FlowrGui::run(Settings {
        antialiasing: true,
        ..Settings::default()
    })
}

#[derive(Clone)]
struct SubmissionSettings {
    // TODO make native a UI setting
    // TODO num threads make a UI setting
    // TODO make lib search path a UI setting
    flow_manifest_url: String,
    flow_args: String,
    debug_this_flow: bool,
    display_metrics: bool,
    parallel_jobs_limit: Option<usize>, // TODO read from settings or UI
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
    auto: bool,
}

struct ImageReference {
    pub width: u32,
    pub height: u32,
    pub data: ImageBuffer<Rgba<u8>, Vec<u8>>,
}

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
}

// Implement the iced Application trait for FlowIde
impl Application for FlowrGui {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    /// Create the flowrgui app and populate fields with options passed on the command line
    fn new(_flags: ()) -> (Self, Command<Message>) {
        let settings = FlowrGui::initial_settings();

        let flowrgui = FlowrGui {
            submission_settings: settings.0,
            coordinator_settings: settings.1,
            ui_settings: settings.2,
            coordinator_state: CoordinatorState::Disconnected("Starting".into()),
            tab_set: TabSet::new(),
            submitted: false,
            running: false,
            show_modal: false,
            modal_content: (String::new(), String::new()),
        };

        (flowrgui, Command::none())
    }

    fn title(&self) -> String {
        String::from("flowrgui")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::CoordinatorSent(CoordinatorMessage::Connected(sender)) => {
                self.coordinator_state = CoordinatorState::Connected(sender);
                if self.ui_settings.auto {
                    return Command::perform(Self::auto_submit(), |()| Message::SubmitFlow);
                }
            }
            Message::SubmitFlow => {
                if let CoordinatorState::Connected(sender) = &self.coordinator_state {
                    return Command::perform(
                        Self::submit(sender.clone(), self.submission_settings.clone()),
                        |()| Message::Submitted,
                    );
                }
            }
            Message::Submitted => {
                self.tab_set.clear();
                self.submitted = true;
            }
            Message::FlowArgsChanged(value) => self.submission_settings.flow_args = value,
            Message::UrlChanged(value) => self.submission_settings.flow_manifest_url = value,
            Message::TabSelected(_) | Message::StdioAutoScrollTogglerChanged(_, _) => {
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
            Message::LineOfStdin(line) => self.tab_set.stdin_tab.new_line(line),
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let main = Column::new()
            .spacing(10)
            .push(self.command_row())
            .push(self.tab_set.view())
            .push(self.status_row())
            .padding(10);

        let overlay = if self.show_modal {
            Some(
                Card::new(
                    Text::new(self.modal_content.clone().0),
                    Text::new(self.modal_content.clone().1),
                )
                    .foot(
                        Row::new().spacing(10).padding(5).width(Length::Fill).push(
                            Button::new(Text::new("OK").horizontal_alignment(Horizontal::Center))
                                .width(Length::Fill)
                                .on_press(Message::CloseModal),
                        ),
                    )
                    .max_width(300.0),
            )
        } else {
            None
        };

        modal(main, overlay)
            .backdrop(Message::CloseModal)
            .on_esc(Message::CloseModal)
            .into()
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
    ) {
        match Self::flow_url(&settings.flow_manifest_url) {
            Ok(url) => {
                let provider =
                    &MetaProvider::new(Simpath::new(""), PathBuf::default()) as &dyn Provider;

                match FlowManifest::load(provider, &url) {
                    Ok((flow_manifest, _)) => {
                        let submission = Submission::new(
                            flow_manifest,
                            settings.parallel_jobs_limit,
                            None, // No timeout waiting for job results
                            settings.debug_this_flow,
                        );

                        info!("Sending submission to Coordinator");
                        #[allow(clippy::single_match_else)]
                        #[allow(clippy::match_same_arms)]
                        match sender
                            .send(ClientMessage::ClientSubmission(submission))
                            .await
                        {
                            Ok(()) => {
                                // TODO report info that submitted
                            }
                            Err(_) => {
                                // TODO report submit error
                            }
                        }
                    }
                    Err(_e) => {
                        // TODO report manifest loading error
                    }
                }
            }
            Err(_e) => {
                // TODO report Invalid Url error
            }
        }
    }

    // report a new error
    #[allow(clippy::unused_self)]
    // TODO implement some display of this info on the UI
    fn error(&mut self, _msg: &str) {}

    // report a new info message
    // TODO implement some display of this info on the UI
    #[allow(clippy::unused_self)]
    fn info(&mut self, _msg: &str) {}

    fn command_row(&self) -> Row<'_, Message> {
        let url = text_input(
            "Flow location (relative, or absolute)",
            &self.submission_settings.flow_manifest_url,
        )
            .on_input(Message::UrlChanged);

        let args = text_input(
            "Space separated flow arguments",
            &self.submission_settings.flow_args,
        )
            .on_submit(Message::SubmitFlow)
            .on_input(Message::FlowArgsChanged)
            .on_paste(Message::FlowArgsChanged);

        let mut play = Button::new("Play");
        if matches!(self.coordinator_state, CoordinatorState::Connected(_))
            && !self.running
            && !self.submitted
        {
            play = play.on_press(Message::SubmitFlow);
        }

        Row::new()
            .spacing(10)
            .align_items(Alignment::End)
            .push(url)
            .push(args)
            .push(play)
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
            .to_string();
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

        (
            SubmissionSettings {
                flow_manifest_url,
                flow_args,
                debug_this_flow,
                display_metrics: matches.get_flag("metrics"),
                parallel_jobs_limit,
            },
            coordinator_settings,
            UiSettings {
                auto: matches.get_flag("auto"),
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
            None => thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
        }
    }

    fn send(&mut self, msg: ClientMessage) {
        if let CoordinatorState::Connected(ref sender) = self.coordinator_state {
            let _ = sender.try_send(msg);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn process_coordinator_message(&mut self, message: CoordinatorMessage) -> Command<Message> {
        match message {
            CoordinatorMessage::Connected(_) => {
                self.error("Coordinator is already connected");
            }
            CoordinatorMessage::FlowStart => {
                self.running = true;
                self.submitted = false;
                self.send(ClientMessage::Ack);
            }
            CoordinatorMessage::FlowEnd(metrics) => {
                self.running = false;
                if self.submission_settings.display_metrics {
                    self.show_modal = true;
                    self.modal_content = ("Flow Ended - Metrics".into(), format!("{metrics}"));
                }
                // NO response - so we can use next request sent to submit another flow
                if self.ui_settings.auto {
                    self.info("Auto exiting on flow completion");
                    process::exit(0);
                }
            }
            CoordinatorMessage::CoordinatorExiting(_) => {
                self.coordinator_state = CoordinatorState::Disconnected("Exited".into());
                self.send(ClientMessage::Ack);
            }
            CoordinatorMessage::Stdout(string) => {
                self.tab_set.stdout_tab.content.push(string);
                self.send(ClientMessage::Ack);
                if self.tab_set.stdout_tab.auto_scroll {
                    return scrollable::snap_to(
                        self.tab_set.stdout_tab.id.clone(),
                        scrollable::RelativeOffset::END,
                    );
                }
            }
            CoordinatorMessage::Stderr(string) => {
                self.tab_set.stderr_tab.content.push(string);
                self.send(ClientMessage::Ack);
                if self.tab_set.stderr_tab.auto_scroll {
                    return scrollable::snap_to(
                        self.tab_set.stderr_tab.id.clone(),
                        scrollable::RelativeOffset::END,
                    );
                }
            }
            CoordinatorMessage::GetStdin => {
                let msg = match self.tab_set.stdin_tab.get_all() {
                    Some(buf) => ClientMessage::Stdin(buf),
                    None => ClientMessage::GetLineEof,
                };
                self.send(msg);
            }
            CoordinatorMessage::GetLine(prompt) => {
                let msg = match self.tab_set.stdin_tab.get_line(&prompt) {
                    Some(line) => ClientMessage::Line(line),
                    None => ClientMessage::GetLineEof,
                };
                self.send(msg);
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
                                /*
                                                               if self.tab_set.stdout_tab.auto_scroll {
                                                                   return scrollable::snap_to(
                                                                       self.tab_set.stdout_tab.id.clone(), scrollable::RelativeOffset::END);
                                                               }
                                */
                                ClientMessage::FileContents(file_path, buffer)
                            }
                            Err(_) => ClientMessage::Error(format!(
                                "Could not read content from '{file_path:?}'"
                            )),
                        }
                    }
                    Err(_) => ClientMessage::Error(format!("Could not open file '{file_path:?}'")),
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
                            self.error("{msg}");
                            ClientMessage::Error(msg)
                        }
                    },
                    Err(e) => {
                        let msg = format!("Error creating file: '{filename}': '{e}'");
                        self.error("{msg}");
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
                    // TODO switch to the images tab when image first written to
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
            _ => {}
        }
        Command::none()
    }
}
