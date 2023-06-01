#![deny(missing_docs)]
// TODO re-instate #![warn(clippy::unwrap_used)]
//! `flowrgui` is a GUI flow runner for running `flow` programs.
//!
//! It reads a compiled [FlowManifest][flowcore::model::flow_manifest::FlowManifest] produced by a
//! flow compiler, such as `flowc`, that describes the graph of communicating functions that
//! constitute the flow program.
//!
//! Use `flowrgui --help` or `flowrgui -h` at the command line to see the command line options
//!
//! The [gui] module implements a set of `context functions`, adapted to a GUI runner
//! that allow the flow program to interact with the environment where it is being run.
//!
//! Depending on the command line options supplied `flowrgui` executes the
//! [Coordinator][flowrlib::coordinator::Coordinator] of flow execution in a background thread or
//! connects to an already running coordinator in another process.
//! Application and Coordinator (thread or process) communicate via network messages using the
//! [SubmissionHandler][flowrlib::submission_handler::SubmissionHandler] to submit flows for execution,
//! and interchanging [ClientMessages][crate::gui::client_message::ClientMessage]
//! and [CoordinatorMessages][crate::gui::coordinator_message::CoordinatorMessage] for execution of context
//! interaction in the client, as requested by functions running in the coordinator's
//! [Executors][flowrlib::executor::Executor]

use core::str::FromStr;
use std::{env, io, process, thread};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

use clap::{Arg, ArgMatches};
use clap::Command as ClapCommand;
use env_logger::Builder;
use flowcore::meta_provider::MetaProvider;
use flowcore::model::flow_manifest::FlowManifest;
use flowcore::model::submission::Submission;
use flowcore::provider::Provider;
use flowcore::url_helper::url_from_string;
use flowrlib::info as flowrlib_info;
use gui::coordinator_connection::CoordinatorConnection;
use gui::debug_message::DebugServerMessage;
use gui::debug_message::DebugServerMessage::*;
use iced::{Alignment, Application, Command, Element, Length, Settings, Subscription, Theme};
use iced::executor;
use iced::widget::{Button, Column, container, Row, scrollable, text, text_input, toggler};
use iced::widget::scrollable::{Id, Scrollable};
use image::{ImageBuffer, Rgb, RgbImage};
use log::{info, LevelFilter, warn};
use log::error;
use once_cell::sync::Lazy;
use simpath::Simpath;
use url::Url;

use crate::errors::*;
use crate::gui::client_message::ClientMessage;
use crate::gui::coordinator_message::CoordinatorMessage;

static STDOUT_SCROLLABLE_ID: Lazy<Id> = Lazy::new(Id::unique);

//use iced_aw::{TabLabel, Tabs};

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

/// [Message] enum captures all the types of messages that are sent to and processed by the
/// [FlowrGui] Iced Application
#[derive(Debug, Clone)]
pub enum Message {
    /// We lost contact with the coordinator
    CoordinatorDisconnected,
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
    /// The toggle to auto-scroll to bottom of STDIO has changed
    StdioAutoScrollTogglerChanged(bool),
}

enum CoordinatorState {
    Disconnected,
    Connected(tokio::sync::mpsc::Sender<ClientMessage>),
}

/// Main for flowrgui binary - call `run()` and print any error that results or exit silently if OK
fn main() -> iced::Result {
    FlowrGui::run(Settings {
        antialiasing: true,
        ..Settings::default()
    })
}

struct SubmissionSettings {
    flow_manifest_url: String,
    flow_args: String,
    debug_this_flow: bool,
    display_metrics: bool,
    parallel_jobs_limit: Option<usize>, // TODO read from settings or UI
}

/// [CoordinatorSettings] captures the parameters to be used when creating a new Coordinator
#[derive(Clone)]
pub struct CoordinatorSettings {
    /// Should the coordinator use the natively linked flowstdlib library, or the wasm version
    native_flowstdlib: bool,
    /// How many executor threads should be used
    num_threads: usize,
    /// The path to search for libs when a lib reference is found
    lib_search_path: Simpath,
}

struct UiSettings {
    auto: bool,
}

struct FlowrGui {
    flow_settings: SubmissionSettings,
    coordinator_settings: CoordinatorSettings,
    ui_settings: UiSettings,
    gui_coordinator: CoordinatorState,
    active_tab: usize,
    stdout: Vec<String>,
    stderr: Vec<String>,
    auto_scroll_stdout: bool,
    running: bool,
    submitted: bool,
    image_buffers: HashMap<String, ImageBuffer<Rgb<u8>, Vec<u8>>>,
}

// Implement the iced Application trait for FlowIde
impl Application for FlowrGui {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    /// Create the FlowIde app and populate fields with options passed on the command line
    fn new(_flags: ()) -> (Self, Command<Message>) {
        let settings = FlowrGui::initial_settings();

        let flowrgui = FlowrGui {
            flow_settings: settings.0,
            coordinator_settings: settings.1,
            ui_settings: settings.2,
            gui_coordinator: CoordinatorState::Disconnected,
            active_tab: 0,
            stdout: Vec::new(),
            stderr: Vec::new(),
            auto_scroll_stdout: true,
            submitted: false,
            running: false,
            image_buffers: HashMap::<String, ImageBuffer<Rgb<u8>, Vec<u8>>>::new(),
        };

        (flowrgui, Command::none())
    }

    fn title(&self) -> String {
        String::from("FlowIde")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match &self.gui_coordinator {
            CoordinatorState::Disconnected => {
                match message {
                    Message::CoordinatorSent(CoordinatorMessage::Connected(sender)) => {
                        self.gui_coordinator = CoordinatorState::Connected(sender);
                        if self.ui_settings.auto {
                            return Command::perform(Self::auto_submit(), |_| Message::SubmitFlow);
                        }
                    },
                    _ => error!("Unexpected message: {:?} when Coordinator Disconnected", message),
                }
            },
            CoordinatorState::Connected(sender) => {
                match message {
                    Message::SubmitFlow => {
                        let url = self.flow_url().unwrap();
                        let parallel_jobs_limit = self.flow_settings.parallel_jobs_limit;
                        let debug_this_flow = self.flow_settings.debug_this_flow;
                        return Command::perform(Self::submit(sender.clone(),
                                                            url, parallel_jobs_limit,
                                                             debug_this_flow
                        ), |_| Message::Submitted);
                    },
                    Message::Submitted => self.submitted = true,
                    Message::FlowArgsChanged(value) => self.flow_settings.flow_args = value,
                    Message::UrlChanged(value) => self.flow_settings.flow_manifest_url = value,
                    Message::CoordinatorSent(coord_msg) =>
                        return self.process_coordinator_message(coord_msg),
                    Message::TabSelected(tab_index) => self.active_tab = tab_index,
                    Message::StdioAutoScrollTogglerChanged(value) => self.auto_scroll_stdout = value,
                    Message::CoordinatorDisconnected => self.gui_coordinator = CoordinatorState::Disconnected,
                }
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let main = Column::new().spacing(10)
            .push(self.command_row())
            .push(self.stdio());
        container(main)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(10)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        coordinator::subscribe(self.coordinator_settings.clone())
            .map(Message::CoordinatorSent)
    }
}

impl FlowrGui {
    async fn auto_submit() {
        info!("Auto submitting flow");
    }

    // Submit the flow to the coordinator for execution
    async fn submit(sender: tokio::sync::mpsc::Sender<ClientMessage>,
                    url: Url,
                    parallel_jobs_limit: Option<usize>,
                    debug_this_flow: bool) {
        let provider = &MetaProvider::new(Simpath::new(""),
                                          PathBuf::default()) as &dyn Provider;

        let (flow_manifest, _) = FlowManifest::load(provider, &url).unwrap(); // TODO
        let submission = Submission::new(
            flow_manifest,
            parallel_jobs_limit,
            None, // No timeout waiting for job results
            debug_this_flow,
        );

        info!("Sending submission to Coordinator");
        let _ = sender.send(ClientMessage::ClientSubmission(submission)).await;
    }

    fn command_row<'a>(&self) -> Element<'a, Message> {
        // .on_submit(), .on_paste(), .width()
        let url = text_input("Flow location (relative, or absolute)",
                             &self.flow_settings.flow_manifest_url)
            .on_input(Message::UrlChanged);
        let args = text_input("Space separated flow arguments",
                              &self.flow_settings.flow_args)
            .on_input(Message::FlowArgsChanged);
        // TODO disable until loaded flow
        let mut play = Button::new("Play");
        if  matches!(self.gui_coordinator, CoordinatorState::Connected(_)) && !self.running && !self.submitted {
            play = play.on_press(Message::SubmitFlow);
        }
        Row::new()
            .spacing(10)
            .align_items(Alignment::End)
            .push(url)
            .push(args)
            .push(play).into()
    }

    fn stdio_area<'a>(content: &[String], id: Id) -> Element<'a, Message> {
        let text_column = Column::with_children(
            content
                .iter()
                .cloned()
                .map(text)
                .map(Element::from)
                .collect(),
            )
            .width(Length::Fill)
            .padding(1);

        Scrollable::new(text_column) //.snap_to_bottom()
            .id(id)
            .into()
    }

    fn stdio<'a>(&self) -> Element<'a, Message> {
        let toggler = toggler(
                "Auto-scroll Stdio".to_owned(),
                self.auto_scroll_stdout,
                Message::StdioAutoScrollTogglerChanged);

        let stdout = Self::stdio_area(&self.stdout, STDOUT_SCROLLABLE_ID.clone());

        Column::new()
            .push(toggler)
            .push(stdout).into()
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

        let lib_search_path = FlowrGui::lib_search_path(&lib_dirs)
            .unwrap(); // TODO

        let flow_manifest_url = matches.get_one::<String>("flow-manifest")
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
        let num_threads = FlowrGui::num_threads(&matches);

        (SubmissionSettings {
            flow_manifest_url,
            flow_args,
            debug_this_flow,
            display_metrics: true,
            parallel_jobs_limit,
        },
         CoordinatorSettings {
            num_threads,
            native_flowstdlib,
            lib_search_path,
        },
            UiSettings {
                auto: matches.get_flag("auto")
            }
        )
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
    fn flow_url(&self) -> flowcore::errors::Result<Url> {
        let cwd_url = Url::from_directory_path(env::current_dir()?)
            .map_err(|_| "Could not form a Url for the current working directory")?;
        url_from_string(&cwd_url, Some(&self.flow_settings.flow_manifest_url))
    }

    // Create array of strings that are the args to the flow
    fn flow_arg_vec(&self) -> Vec<String> {
        // arg #0 is the flow url
        let mut flow_args: Vec<String> = vec![self.flow_settings.flow_manifest_url.clone()];
        let additional_args : Vec<String> = self.flow_settings.flow_args.split(' ')
            .map(|s| s.to_string()).collect();
        flow_args.extend(additional_args);
        flow_args
    }

    // For the lib provider, libraries maybe installed in multiple places in the file system.
    // In order to find the content, a FLOW_LIB_PATH environment variable can be configured with a
    // list of directories in which to look for the library in question.
    fn lib_search_path(search_path_additions: &[String]) -> Result<Simpath> {
        let mut lib_search_path = Simpath::new_with_separator("FLOW_LIB_PATH",
                                                              ',');

        for additions in search_path_additions {
            lib_search_path.add(additions);
            info!("'{}' added to the Library Search Path", additions);
        }

        if lib_search_path.is_empty() {
            warn!("'$FLOW_LIB_PATH' not set and no LIB_DIRS supplied. Libraries may not be found.");
        }

        Ok(lib_search_path)
    }

    // Determine the number of threads to use to execute flows
    // - default (if value is not provided on the command line) of the number of cores
    fn num_threads(matches: &ArgMatches) -> usize {
        match matches.get_one::<usize>("threads") {
            Some(num_threads) => *num_threads,
            None => thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
        }
    }

    fn send(&mut self, msg: ClientMessage) {
        if let CoordinatorState::Connected(ref sender) = self.gui_coordinator {
            let _ = sender.try_send(msg);
        }
    }

    fn process_coordinator_message(&mut self, message: CoordinatorMessage) -> Command<Message> {
        match message {
            CoordinatorMessage::Connected(_) => {
                error!("Coordinator is already connected");
            },
            CoordinatorMessage::FlowStart => {
                self.running = true;
                self.submitted = false;
                self.send(ClientMessage::Ack);
            },
            CoordinatorMessage::FlowEnd(metrics) => {
                self.running = false;
                if self.flow_settings.display_metrics {
                    // TODO put on UI
                    println!("{}", metrics);
                }
                self.send(ClientMessage::Ack);
                if self.ui_settings.auto {
                    info!("Auto exiting on flow completion");
                    process::exit(0);
                }
            }
            CoordinatorMessage::CoordinatorExiting(_) => {
                self.gui_coordinator = CoordinatorState::Disconnected;
                self.send(ClientMessage::Ack);
            },
            CoordinatorMessage::Stdout(string) => {
                self.stdout.push(string);
                self.send(ClientMessage::Ack);
                if self.auto_scroll_stdout {
                    return scrollable::snap_to(
                        STDOUT_SCROLLABLE_ID.clone(), scrollable::RelativeOffset::END);
                }
            },
            CoordinatorMessage::Stderr(string) => {
                self.stderr.push(string);
                self.send(ClientMessage::Ack);
            },
            CoordinatorMessage::GetStdin => {
                // TODO read the buffer entirely and reset the cursor to after that text
                // grey out the text read?
                let mut buffer = String::new();
                let msg = if let Ok(size) = io::stdin().read_to_string(&mut buffer) {
                    if size > 0 {
                        ClientMessage::Stdin(buffer.trim().to_string())
                    } else {
                        ClientMessage::GetStdinEof
                    }
                } else {
                    ClientMessage::Error("Could not read Stdin".into())
                };
                self.send(msg);
            }
            CoordinatorMessage::GetLine(prompt) => {
                // TODO print the prompt, read one line of input, move cursor and grey out text
                // If there is no text to pickup beyond the cursor then prompt the user for more
                let mut input = String::new();
                if !prompt.is_empty() {
                    print!("{}", prompt);
                    let _ = io::stdout().flush();
                }
                let line = io::stdin().lock().read_line(&mut input);
                let msg = match line {
                    Ok(n) if n > 0 => ClientMessage::Line(input.trim().to_string()),
                    Ok(n) if n == 0 => ClientMessage::GetLineEof,
                    _ => ClientMessage::Error("Could not read Readline".into()),
                };
                self.send(msg);
            }
            CoordinatorMessage::GetArgs => {
                let args = self.flow_arg_vec();
                let msg = ClientMessage::Args(args);
                self.send(msg);

                /*
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
                            Ok(_) => ClientMessage::FileContents(file_path, buffer),
                            Err(_) => ClientMessage::Error(format!(
                                "Could not read content from '{file_path:?}'"
                            )),
                        }
                    }
                    Err(_) => ClientMessage::Error(format!("Could not open file '{file_path:?}'")),
                };
                self.send(msg);
            },
            CoordinatorMessage::Write(filename, bytes) => {
                // TODO list file reads and write in the UI somewhere
                let msg = match File::create(&filename) {
                    Ok(mut file) => match file.write_all(bytes.as_slice()) {
                        Ok(_) => ClientMessage::Ack,
                        Err(e) => {
                            let msg = format!("Error writing to file: '{filename}': '{e}'");
                            error!("{msg}");
                            ClientMessage::Error(msg)
                        }
                    },
                    Err(e) => {
                        let msg = format!("Error creating file: '{filename}': '{e}'");
                        error!("{msg}");
                        ClientMessage::Error(msg)
                    }
                };
                self.send(msg);
            },
            CoordinatorMessage::PixelWrite((x, y), (r, g, b), (width, height), name) => {
                let image = self
                    .image_buffers
                    .entry(name)
                    .or_insert_with(|| RgbImage::new(width, height));
                image.put_pixel(x, y, Rgb([r, g, b]));
                self.send(ClientMessage::Ack);
            }
            _ => {},
        }
        Command::none()
    }
}