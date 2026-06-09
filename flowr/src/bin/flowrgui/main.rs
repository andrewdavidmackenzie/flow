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
use flowrlib::connections::CoordinatorConnection;
#[cfg(feature = "debugger")]
use flowrlib::debug_client::DebugClient;
use flowrlib::info as flowrlib_info;

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
mod icons;
mod tabs;

/// provides [Error][errors::Error] that other modules in this crate will `use crate::errors::*;`
/// to get access to everything `error_chain` creates.
mod errors;

/// custom widget styling
mod theme;

/// A clickable link in debug output
#[cfg(feature = "debugger")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum LinkType {
    Function,
    Flow,
    Job,
    Input,
    Output,
    Route,
    State,
    Other,
}

/// A clickable link in debug output
#[cfg(feature = "debugger")]
#[derive(Debug, Clone)]
pub struct DebugLink {
    /// Byte range in the text
    pub start: usize,
    /// End of byte range
    pub end: usize,
    /// Inspect spec to trigger on click
    pub spec: String,
    /// Entity type for color coding
    pub link_type: LinkType,
}

/// A line of debug output with optional color and clickable links
#[cfg(feature = "debugger")]
#[derive(Debug, Clone)]
pub struct DebugEventLine {
    /// The text content
    pub text: String,
    /// Optional color override (None = default theme text color)
    pub color: Option<iced::Color>,
    /// Whether this line is a separator (rendered as Rule + label + Rule)
    pub separator: bool,
    /// Clickable links in this line
    pub links: Vec<DebugLink>,
    /// Section ID for collapsible grouping (set by `DebugTab` on push)
    pub section_id: usize,
}

#[cfg(feature = "debugger")]
impl DebugEventLine {
    fn new(text: String, color: Option<iced::Color>) -> Self {
        let links = Self::extract_links(&text);
        Self {
            text,
            color,
            separator: false,
            links,
            section_id: 0,
        }
    }

    /// Create a builder for constructing lines with chip links
    #[must_use]
    pub fn build() -> DebugLineBuilder {
        DebugLineBuilder::new()
    }

    /// Create a line with pre-built links (skipping text extraction)
    #[must_use]
    pub fn with_links(text: String, color: Option<iced::Color>, links: Vec<DebugLink>) -> Self {
        Self {
            text,
            color,
            separator: false,
            links,
            section_id: 0,
        }
    }

    fn separator(label: String, color: iced::Color) -> Self {
        Self {
            text: label,
            color: Some(color),
            separator: true,
            links: Vec::new(),
            section_id: 0,
        }
    }

    #[allow(clippy::too_many_lines)]
    fn extract_links(text: &str) -> Vec<DebugLink> {
        let mut links = Vec::new();
        let mut search_from = 0;

        while let Some(pos) = text[search_from..].find("Function #") {
            let abs_pos = search_from + pos;
            let after_hash = abs_pos + "Function #".len();
            let digit_end = text[after_hash..]
                .find(|c: char| !c.is_ascii_digit())
                .map_or(text.len(), |i| after_hash + i);
            if digit_end > after_hash {
                let id_str = &text[after_hash..digit_end];
                links.push(DebugLink {
                    start: abs_pos,
                    end: digit_end,
                    spec: id_str.to_string(),
                    link_type: LinkType::Function,
                });
            }
            search_from = digit_end;
        }

        // Match standalone #N patterns (process tree lines like "#1 'add' @ ...")
        if text.trim_start().starts_with('#') && !text.contains("Function #") {
            let trimmed = text.trim_start();
            let hash_pos = text.len() - trimmed.len();
            let after_hash = hash_pos + 1;
            if after_hash < text.len() {
                let digit_end = text[after_hash..]
                    .find(|c: char| !c.is_ascii_digit())
                    .map_or(text.len(), |i| after_hash + i);
                if digit_end > after_hash {
                    let id_str = &text[after_hash..digit_end];
                    links.push(DebugLink {
                        start: hash_pos,
                        end: digit_end,
                        spec: id_str.to_string(),
                        link_type: LinkType::Function,
                    });
                }
            }
        }

        // Match Flow #N patterns
        search_from = 0;
        while let Some(pos) = text[search_from..].find("Flow #") {
            let abs_pos = search_from + pos;
            let after_hash = abs_pos + "Flow #".len();
            let digit_end = text[after_hash..]
                .find(|c: char| !c.is_ascii_digit())
                .map_or(text.len(), |i| after_hash + i);
            if digit_end > after_hash {
                let id_str = &text[after_hash..digit_end];
                links.push(DebugLink {
                    start: abs_pos,
                    end: digit_end,
                    spec: id_str.to_string(),
                    link_type: LinkType::Flow,
                });
            }
            search_from = digit_end;
        }

        let context_func_id = None::<usize>.or_else(|| {
            text.find("Function #").and_then(|pos| {
                let after = pos + "Function #".len();
                let end = text[after..]
                    .find(|c: char| !c.is_ascii_digit())
                    .map_or(text.len(), |i| after + i);
                text[after..end].parse::<usize>().ok()
            })
        });

        // Match Job #N patterns — link to the function that ran the job
        search_from = 0;
        while let Some(pos) = text[search_from..].find("Job #") {
            let abs_pos = search_from + pos;
            let after_hash = abs_pos + "Job #".len();
            let digit_end = text[after_hash..]
                .find(|c: char| !c.is_ascii_digit())
                .map_or(text.len(), |i| after_hash + i);
            if digit_end > after_hash {
                if let Some(func_id) = context_func_id {
                    links.push(DebugLink {
                        start: abs_pos,
                        end: digit_end,
                        spec: func_id.to_string(),
                        link_type: LinkType::Job,
                    });
                }
            }
            search_from = digit_end;
        }

        // Match Input:N patterns
        search_from = 0;
        while let Some(pos) = text[search_from..].find("Input:") {
            let abs_pos = search_from + pos;
            let after = abs_pos + "Input:".len();
            let digit_end = text[after..]
                .find(|c: char| !c.is_ascii_digit())
                .map_or(text.len(), |i| after + i);
            if digit_end > after {
                if let Some(func_id) = context_func_id {
                    let input_num = &text[after..digit_end];
                    links.push(DebugLink {
                        start: abs_pos,
                        end: digit_end,
                        spec: format!("{func_id}:{input_num}"),
                        link_type: LinkType::Input,
                    });
                }
            }
            search_from = digit_end;
        }

        // Match Output routes like "Output '...'" or "Output:"
        search_from = 0;
        while let Some(pos) = text[search_from..].find("Output ") {
            let abs_pos = search_from + pos;
            let output_text_end = text[abs_pos..]
                .find("->")
                .map_or(abs_pos + "Output ".len(), |i| abs_pos + i);
            if let Some(func_id) = context_func_id {
                links.push(DebugLink {
                    start: abs_pos,
                    end: output_text_end.min(abs_pos + 20),
                    spec: format!("{func_id}/"),
                    link_type: LinkType::Output,
                });
            }
            search_from = output_text_end;
        }

        // Match ALL occurrences of state keywords in square brackets
        for keyword in &[
            "[Ready]",
            "[Waiting]",
            "[Running]",
            "[Completed]",
            "[Blocked]",
        ] {
            let mut kw_from = 0;
            while let Some(pos) = text[kw_from..].find(keyword) {
                let abs_pos = kw_from + pos;
                let state_name = &keyword[1..keyword.len() - 1];
                links.push(DebugLink {
                    start: abs_pos,
                    end: abs_pos + keyword.len(),
                    spec: state_name.to_lowercase(),
                    link_type: LinkType::State,
                });
                kw_from = abs_pos + keyword.len();
            }
        }

        // Match route paths like '/my-first-flow/add'
        search_from = 0;
        while let Some(pos) = text[search_from..].find("'/") {
            let abs_pos = search_from + pos + 1;
            if let Some(end_quote) = text[abs_pos..].find('\'') {
                let route = &text[abs_pos..abs_pos + end_quote];
                links.push(DebugLink {
                    start: abs_pos,
                    end: abs_pos + end_quote,
                    spec: route.to_string(),
                    link_type: LinkType::Route,
                });
                search_from = abs_pos + end_quote;
            } else {
                break;
            }
        }

        // Match RunState field labels as links to state inspections
        for (label, spec) in &[
            ("Jobs Running:", "running"),
            ("Functions Ready:", "ready"),
            ("Functions Completed:", "completed"),
        ] {
            if let Some(pos) = text.find(label) {
                links.push(DebugLink {
                    start: pos,
                    end: pos + label.len(),
                    spec: (*spec).to_string(),
                    link_type: LinkType::Other,
                });
            }
        }

        // Match Busy Functions entries like "1: 1, 0: 1" — the keys are function IDs
        if let Some(after_colon) = text.strip_prefix("Busy Functions:") {
            for part in after_colon.split(',') {
                let part = part.trim().trim_start_matches(['{', ' ']);
                if let Some(colon_pos) = part.find(':') {
                    let key = part[..colon_pos].trim();
                    if let Ok(_id) = key.parse::<usize>() {
                        if let Some(abs_pos) = text.find(&format!("{key}:")) {
                            links.push(DebugLink {
                                start: abs_pos,
                                end: abs_pos + key.len(),
                                spec: key.to_string(),
                                link_type: LinkType::Function,
                            });
                        }
                    }
                }
            }
        }

        links.sort_by_key(|l| l.start);
        // Remove overlapping links
        links.dedup_by(|b, a| b.start < a.end);
        links
    }
}

/// Builder for constructing debug lines with embedded entity-typed chip links
#[cfg(feature = "debugger")]
#[derive(Default)]
pub struct DebugLineBuilder {
    text: String,
    links: Vec<DebugLink>,
    color: Option<iced::Color>,
}

#[cfg(feature = "debugger")]
impl DebugLineBuilder {
    /// Create a new empty builder
    #[must_use]
    pub fn new() -> Self {
        Self {
            text: String::new(),
            links: Vec::new(),
            color: None,
        }
    }

    /// Append plain text
    #[must_use]
    pub fn text(mut self, s: &str) -> Self {
        self.text.push_str(s);
        self
    }

    /// Append a clickable chip with entity type coloring
    #[must_use]
    pub fn chip(mut self, label: &str, spec: &str, link_type: LinkType) -> Self {
        let start = self.text.len();
        self.text.push_str(label);
        let end = self.text.len();
        self.links.push(DebugLink {
            start,
            end,
            spec: spec.to_string(),
            link_type,
        });
        self
    }

    /// Set the line color
    #[must_use]
    pub fn color(mut self, c: iced::Color) -> Self {
        self.color = Some(c);
        self
    }

    /// Build the final `DebugEventLine`
    #[must_use]
    pub fn finish(self) -> DebugEventLine {
        DebugEventLine::with_links(self.text, self.color, self.links)
    }
}

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
    /// Save the text content of a tab to a file
    SaveTabContent(String),
    /// Save an image to a file by its name key
    SaveImage(String),
    /// An error occurred while saving content to a file
    SaveError(String),
    /// closing of the Modal was requested
    CloseModal,
    /// Formatted debug event lines from the debug server
    #[cfg(feature = "debugger")]
    DebugEvent(Vec<DebugEventLine>),
    /// The debugger is waiting for a command (enables debug buttons)
    #[cfg(feature = "debugger")]
    DebugWaiting,
    /// Debug client connected to the debug server
    #[cfg(feature = "debugger")]
    DebugConnected,
    /// Debug client disconnected from the debug server
    #[cfg(feature = "debugger")]
    DebugDisconnected(String),
    /// User clicked Continue in the debug controls
    #[cfg(feature = "debugger")]
    DebugContinue,
    /// User clicked Step in the debug controls
    #[cfg(feature = "debugger")]
    DebugStep,
    /// User clicked Run/Reset in the debug controls
    #[cfg(feature = "debugger")]
    DebugReset,
    /// User clicked Run Process — runs a specific process using the spec field
    #[cfg(feature = "debugger")]
    DebugRunProcess,
    /// User clicked Exit Debugger in the debug controls
    #[cfg(feature = "debugger")]
    DebugExit,
    /// User clicked Pause to break into the debugger mid-execution
    #[cfg(feature = "debugger")]
    DebugPause,
    /// User changed a value in the run input panel
    #[cfg(feature = "debugger")]
    RunInputChanged(usize, String),
    /// User clicked Execute in the run input panel
    #[cfg(feature = "debugger")]
    RunInputExecute,
    /// User cancelled the run input panel
    #[cfg(feature = "debugger")]
    RunInputCancel,
    /// The step count text input changed
    #[cfg(feature = "debugger")]
    DebugStepCountChanged(String),
    /// The breakpoint/inspect spec text input changed
    #[cfg(feature = "debugger")]
    DebugSpecChanged(String),
    /// User clicked Set Breakpoint
    #[cfg(feature = "debugger")]
    DebugSetBreakpoint,
    /// User clicked Delete All Breakpoints
    #[cfg(feature = "debugger")]
    DebugDeleteBreakpoints,
    /// User clicked List Breakpoints
    #[cfg(feature = "debugger")]
    DebugListBreakpoints,
    /// User clicked Inspect State
    #[cfg(feature = "debugger")]
    DebugInspect,
    /// User clicked Functions list
    #[cfg(feature = "debugger")]
    DebugFunctions(bool),
    /// User clicked Flows list
    #[cfg(feature = "debugger")]
    DebugFlows,
    /// User clicked Processes tree
    #[cfg(feature = "debugger")]
    DebugProcesses,
    /// User clicked State button — show `RunState` stats only
    #[cfg(feature = "debugger")]
    DebugState,
    /// User clicked Validate
    #[cfg(feature = "debugger")]
    DebugValidate,
    /// Show the breakpoint spec popup
    #[cfg(feature = "debugger")]
    ShowBpPopup,
    /// Close the breakpoint spec popup
    #[cfg(feature = "debugger")]
    CloseBpPopup,
    /// Breakpoint type changed in popup
    #[cfg(feature = "debugger")]
    BpTabChanged(BpTab),
    /// Breakpoint target changed in popup
    #[cfg(feature = "debugger")]
    BpTargetChanged(String),
    /// Confirm and set breakpoint from popup
    #[cfg(feature = "debugger")]
    BpPopupConfirm,
    /// Cycle before/after breakpoint on a function in Route tab
    #[cfg(feature = "debugger")]
    BpCycleFunction(usize),
    /// A clickable link in the debug output was clicked (spec to inspect)
    #[cfg(feature = "debugger")]
    DebugInspectLink(String),
    /// Toggle collapse of a debug output section
    #[cfg(feature = "debugger")]
    DebugToggleSection(usize),
    /// Function list received from debug server
    #[cfg(feature = "debugger")]
    DebugFunctionListReceived(Vec<CachedFunction>),
    /// Breakpoint list received from debug server
    #[cfg(feature = "debugger")]
    DebugBreakpointListReceived(Vec<String>),
    /// Discover coordinators on the network
    DiscoverCoordinators,
    /// Services discovered (label, address)
    ServicesDiscovered(Vec<(String, String)>),
    /// User selected a service from the picker
    ServiceSelected(String, String),
    /// Close the coordinator picker
    CloseCoordinatorPicker,
    /// Show the inspect helper popup
    #[cfg(feature = "debugger")]
    ShowInspectPopup,
    /// Close the inspect helper popup
    #[cfg(feature = "debugger")]
    CloseInspectPopup,
    /// Inspect tab changed in popup
    #[cfg(feature = "debugger")]
    InspectTabChanged(InspectTab),
    /// Item selected in inspect popup
    #[cfg(feature = "debugger")]
    InspectPopupSelect(String),
}

#[allow(clippy::ignored_unit_patterns)]
enum CoordinatorState {
    Disconnected(String),
    Connected(tokio::sync::mpsc::Sender<ClientMessage>),
}

/// Detect if the system prefers dark mode.
fn dark_mode_enabled() -> bool {
    #[cfg(target_os = "macos")]
    {
        // Check macOS dark mode via `defaults read`
        std::process::Command::new("defaults")
            .args(["read", "-g", "AppleInterfaceStyle"])
            .output()
            .map_or(true, |o| {
                String::from_utf8_lossy(&o.stdout).contains("Dark")
            })
    }
    #[cfg(not(target_os = "macos"))]
    {
        // Default to dark on Linux/other
        true
    }
}

/// Main for flowrgui binary - call `run()` and print any error that results or exit silently if OK
fn main() -> iced::Result {
    iced::application(FlowrGui::new, FlowrGui::update, FlowrGui::view)
        .subscription(FlowrGui::subscription)
        .title(FlowrGui::title)
        .theme(FlowrGui::theme)
        .font(icons::FONT)
        .antialiasing(true)
        .window_size((1100.0, 700.0))
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
    #[cfg(feature = "debugger")]
    debug_mode: DebugMode,
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
    ClientOnly,
}

#[cfg(feature = "debugger")]
#[derive(Clone, PartialEq, Eq)]
enum DebugMode {
    Off,
    GuiLocal,
    External,
}

/// Cached function info for the breakpoint popup
#[cfg(feature = "debugger")]
#[derive(Debug, Clone)]
pub struct CachedFunction {
    /// Function ID
    pub id: usize,
    /// Function name
    pub name: String,
    /// Function route
    pub route: String,
    /// Input info (index, name, `is_generic`)
    pub inputs: Vec<(usize, String, bool)>,
    /// Output routes
    pub outputs: Vec<String>,
    /// Whether this is a flow (not a leaf function)
    pub is_flow: bool,
}

/// Tabs in the breakpoint popup
#[cfg(feature = "debugger")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpTab {
    /// Break before a function executes
    Before,
    /// Break after a function completes
    After,
    /// Break when data arrives at an input
    Input,
    /// Break when data is sent from an output
    Output,
    /// Full hierarchical route view
    Route,
}

#[cfg(feature = "debugger")]
impl std::fmt::Display for BpTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Before => write!(f, "Before"),
            Self::After => write!(f, "After"),
            Self::Input => write!(f, "Input"),
            Self::Output => write!(f, "Output"),
            Self::Route => write!(f, "Route"),
        }
    }
}

#[cfg(feature = "debugger")]
const BP_TABS: [BpTab; 5] = [
    BpTab::Before,
    BpTab::After,
    BpTab::Input,
    BpTab::Output,
    BpTab::Route,
];

/// Tabs in the inspect popup
#[cfg(feature = "debugger")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectTab {
    /// Inspect by state (ready, waiting, etc.)
    State,
    /// Inspect a function
    Function,
    /// Inspect a flow
    Flow,
    /// Inspect an input
    Input,
    /// Inspect an output
    Output,
    /// Inspect by route
    Route,
}

#[cfg(feature = "debugger")]
impl std::fmt::Display for InspectTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::State => write!(f, "State"),
            Self::Function => write!(f, "Function"),
            Self::Flow => write!(f, "Flow"),
            Self::Input => write!(f, "Input"),
            Self::Output => write!(f, "Output"),
            Self::Route => write!(f, "Route"),
        }
    }
}

#[cfg(feature = "debugger")]
const INSPECT_TABS: [InspectTab; 6] = [
    InspectTab::State,
    InspectTab::Function,
    InspectTab::Flow,
    InspectTab::Input,
    InspectTab::Output,
    InspectTab::Route,
];

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
    #[cfg(feature = "debugger")]
    debug_waiting: bool,
    #[cfg(feature = "debugger")]
    debug_spec_text: String,
    #[cfg(feature = "debugger")]
    debug_step_count: String,
    #[cfg(feature = "debugger")]
    debug_client_active: bool,
    #[cfg(feature = "debugger")]
    show_bp_popup: bool,
    #[cfg(feature = "debugger")]
    bp_tab: BpTab,
    #[cfg(feature = "debugger")]
    bp_target: String,
    #[cfg(feature = "debugger")]
    cached_functions: Vec<CachedFunction>,
    #[cfg(feature = "debugger")]
    active_breakpoints: std::collections::HashSet<String>,
    #[cfg(feature = "debugger")]
    show_inspect_popup: bool,
    #[cfg(feature = "debugger")]
    suppress_next_output: bool,
    #[cfg(feature = "debugger")]
    inspect_tab: InspectTab,
    #[cfg(feature = "debugger")]
    show_run_inputs: bool,
    #[cfg(feature = "debugger")]
    run_target_id: Option<usize>,
    #[cfg(feature = "debugger")]
    run_input_values: Vec<String>,
    #[cfg(feature = "debugger")]
    run_input_names: Vec<String>,
    #[cfg(feature = "debugger")]
    run_input_types: Vec<String>,
    #[cfg(feature = "debugger")]
    pending_action: Option<Message>,
    show_coordinator_picker: bool,
    discovered_services: Vec<(String, String)>,
    discovering: bool,
    selected_debug_address: Option<String>,
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
            #[cfg(feature = "debugger")]
            debug_waiting: false,
            #[cfg(feature = "debugger")]
            debug_spec_text: String::new(),
            #[cfg(feature = "debugger")]
            debug_step_count: String::new(),
            #[cfg(feature = "debugger")]
            debug_client_active: false,
            #[cfg(feature = "debugger")]
            show_bp_popup: false,
            #[cfg(feature = "debugger")]
            bp_tab: BpTab::Before,
            #[cfg(feature = "debugger")]
            bp_target: String::new(),
            #[cfg(feature = "debugger")]
            cached_functions: Vec::new(),
            #[cfg(feature = "debugger")]
            active_breakpoints: std::collections::HashSet::new(),
            #[cfg(feature = "debugger")]
            show_inspect_popup: false,
            #[cfg(feature = "debugger")]
            suppress_next_output: false,
            #[cfg(feature = "debugger")]
            inspect_tab: InspectTab::State,
            #[cfg(feature = "debugger")]
            show_run_inputs: false,
            #[cfg(feature = "debugger")]
            run_target_id: None,
            #[cfg(feature = "debugger")]
            run_input_values: Vec::new(),
            #[cfg(feature = "debugger")]
            run_input_names: Vec::new(),
            #[cfg(feature = "debugger")]
            run_input_types: Vec::new(),
            #[cfg(feature = "debugger")]
            pending_action: None,
            show_coordinator_picker: false,
            discovered_services: Vec::new(),
            discovering: false,
            selected_debug_address: None,
        };

        (flowrgui, Task::none())
    }

    #[allow(clippy::unused_self)]
    fn title(&self) -> String {
        String::from("flowrgui")
    }

    #[allow(clippy::unused_self)]
    fn theme(&self) -> iced::Theme {
        if dark_mode_enabled() {
            iced::Theme::CatppuccinMocha
        } else {
            iced::Theme::CatppuccinLatte
        }
    }

    #[allow(clippy::too_many_lines)]
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::CoordinatorSent(CoordinatorMessage::Connected(sender)) => {
                self.coordinator_state = CoordinatorState::Connected(sender);
                if self.ui_settings.auto_start {
                    #[cfg(feature = "debugger")]
                    if self.submission_settings.debug_this_flow {
                        return Task::perform(Self::auto_submit(), |()| Message::DebugSubmitFlow);
                    }
                    return Task::perform(Self::auto_submit(), |()| Message::SubmitFlow);
                }
            }
            Message::SubmitFlow => {
                if matches!(self.coordinator_state, CoordinatorState::Disconnected(_))
                    && matches!(self.coordinator_settings, CoordinatorSettings::ClientOnly)
                {
                    return Task::done(Message::DiscoverCoordinators);
                }
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
                self.tab_set.flow_name =
                    std::path::Path::new(&self.submission_settings.flow_manifest_url)
                        .parent()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();
                self.submitted = true;
            }
            Message::SubmitError(msg) | Message::SaveError(msg) => {
                self.show_modal = true;
                self.modal_content = ("Error".into(), msg);
            }
            Message::StopFlow => {
                connection_manager::request_stop();
                #[cfg(feature = "debugger")]
                if self.debug_client_active {
                    self.debug_waiting = false;
                    connection_manager::send_debug_command(
                        flowcore::model::debug_command::DebugCommand::ExitDebugger,
                    );
                }
            }
            Message::FlowArgsChanged(value) => self.submission_settings.flow_args = value,
            Message::MaxJobsChanged(value) => {
                self.submission_settings.parallel_jobs_limit = value.trim().parse::<usize>().ok();
                self.submission_settings.max_jobs_text = value;
            }
            Message::DebugSubmitFlow => {
                if matches!(self.coordinator_state, CoordinatorState::Disconnected(_))
                    && matches!(self.coordinator_settings, CoordinatorSettings::ClientOnly)
                {
                    return Task::done(Message::DiscoverCoordinators);
                }
                if let CoordinatorState::Connected(sender) = &self.coordinator_state {
                    let mut settings = self.submission_settings.clone();
                    settings.debug_this_flow = true;
                    self.submission_settings.debug_this_flow = true;
                    #[cfg(feature = "debugger")]
                    {
                        self.debug_client_active = true;
                        self.debug_waiting = false;
                    }
                    return Task::perform(Self::submit(sender.clone(), settings), |result| {
                        match result {
                            Ok(()) => Message::Submitted,
                            Err(msg) => Message::SubmitError(msg),
                        }
                    });
                }
            }
            Message::UrlChanged(value) => self.submission_settings.flow_manifest_url = value,
            Message::DiscoverCoordinators => {
                self.show_coordinator_picker = true;
                self.discovering = true;
                self.discovered_services.clear();
                let discover_debug = self.submission_settings.debug_this_flow;
                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let mut results = Vec::new();
                            let timeout = std::time::Duration::from_secs(5);
                            match flowcore::discovery::discover_services(
                                flowrlib::services::COORDINATOR_SERVICE_NAME,
                                timeout,
                            ) {
                                Ok(coords) => {
                                    for (addr, _port) in coords {
                                        results.push(("Coordinator".to_string(), addr));
                                    }
                                }
                                Err(e) => {
                                    log::error!("Coordinator discovery failed: {e}");
                                }
                            }
                            #[cfg(feature = "debugger")]
                            if discover_debug {
                                if let Ok(debugs) = flowcore::discovery::discover_services(
                                    flowrlib::services::DEBUG_SERVICE_NAME,
                                    timeout,
                                ) {
                                    for (addr, _port) in debugs {
                                        results.push(("Debug Server".to_string(), addr));
                                    }
                                }
                            }
                            results
                        })
                        .await
                        .unwrap_or_default()
                    },
                    Message::ServicesDiscovered,
                );
            }
            Message::ServicesDiscovered(services) => {
                self.discovered_services = services;
                self.discovering = false;
            }
            Message::ServiceSelected(service_type, address) => {
                info!("User selected {service_type} at {address}");
                if service_type == "Coordinator" {
                    connection_manager::set_discovered_address(address);
                    if !self.submission_settings.debug_this_flow
                        || self
                            .discovered_services
                            .iter()
                            .all(|(t, _)| t != "Debug Server")
                    {
                        self.show_coordinator_picker = false;
                        self.ui_settings.auto_start = true;
                    }
                } else if service_type == "Debug Server" {
                    #[cfg(feature = "debugger")]
                    {
                        self.selected_debug_address = Some(address);
                        self.debug_client_active = true;
                        self.submission_settings.debug_this_flow = true;
                    }
                    self.show_coordinator_picker = false;
                    self.ui_settings.auto_start = true;
                }
            }
            Message::CloseCoordinatorPicker => {
                self.show_coordinator_picker = false;
            }
            Message::TabSelected(_)
            | Message::StdioAutoScrollTogglerChanged(_, _)
            | Message::ClearTab(_)
            | Message::SaveTabContent(_)
            | Message::SaveImage(_) => {
                return self.tab_set.update(message);
            }
            Message::CoordinatorSent(coord_msg) => {
                return self.process_coordinator_message(coord_msg);
            }
            Message::CloseModal => self.show_modal = false,
            #[cfg(feature = "debugger")]
            msg @ (Message::DebugEvent(_)
            | Message::DebugWaiting
            | Message::DebugConnected
            | Message::DebugDisconnected(_)
            | Message::DebugContinue
            | Message::DebugStep
            | Message::DebugReset
            | Message::DebugRunProcess
            | Message::DebugExit
            | Message::DebugPause
            | Message::RunInputChanged(_, _)
            | Message::RunInputExecute
            | Message::RunInputCancel
            | Message::DebugStepCountChanged(_)
            | Message::DebugSpecChanged(_)
            | Message::DebugSetBreakpoint
            | Message::DebugDeleteBreakpoints
            | Message::DebugListBreakpoints
            | Message::DebugInspect
            | Message::DebugFunctions(_)
            | Message::DebugFlows
            | Message::DebugProcesses
            | Message::DebugState
            | Message::DebugValidate
            | Message::ShowBpPopup
            | Message::CloseBpPopup
            | Message::BpTabChanged(_)
            | Message::BpTargetChanged(_)
            | Message::BpPopupConfirm
            | Message::BpCycleFunction(_)
            | Message::DebugFunctionListReceived(_)
            | Message::DebugBreakpointListReceived(_)
            | Message::DebugInspectLink(_)
            | Message::DebugToggleSection(_)
            | Message::ShowInspectPopup
            | Message::CloseInspectPopup
            | Message::InspectTabChanged(_)
            | Message::InspectPopupSelect(_)) => {
                return self.process_debug_message(msg);
            }
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
        let mut main_content = Column::new().spacing(4).push(self.command_row());

        #[cfg(feature = "debugger")]
        if self.submission_settings.debug_this_flow && self.debug_client_active {
            main_content = main_content.push(self.debug_row());
            if self.show_run_inputs {
                main_content = main_content.push(self.run_input_row());
            }
        }

        let main_content = main_content
            .push(self.tab_set.view())
            .push(self.status_bar())
            .padding([theme::SPACE_XS, theme::SPACE_SM]);

        #[cfg(feature = "debugger")]
        if self.show_bp_popup {
            let bp_popup = self.bp_popup_card();
            return stack![
                main_content,
                opaque(mouse_area(center(opaque(bp_popup))).on_press(Message::CloseBpPopup))
            ]
            .into();
        }

        #[cfg(feature = "debugger")]
        if self.show_inspect_popup {
            let popup = self.inspect_popup_card();
            return stack![
                main_content,
                opaque(mouse_area(center(opaque(popup))).on_press(Message::CloseInspectPopup))
            ]
            .into();
        }

        if self.show_coordinator_picker {
            let picker = self.coordinator_picker_card();
            return stack![
                main_content,
                opaque(
                    mouse_area(center(opaque(picker))).on_press(Message::CloseCoordinatorPicker)
                )
            ]
            .into();
        }

        if self.show_modal {
            let modal_card = Card::new(
                Text::new(self.modal_content.clone().0),
                Text::new(self.modal_content.clone().1),
            )
            .foot(
                Row::new().spacing(10).padding(5).width(Fill).push(
                    Button::new(Text::new("OK").align_x(Center))
                        .width(Fill)
                        .style(theme::pill_button)
                        .on_press(Message::CloseModal),
                ),
            )
            .style(theme::popup_card)
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
        let coordinator_sub = connection_manager::subscribe(self.coordinator_settings.clone())
            .map(Message::CoordinatorSent);

        #[cfg(feature = "debugger")]
        if self.debug_client_active {
            let address = if let Some(ref addr) = self.selected_debug_address {
                Some(addr.clone())
            } else {
                match &self.submission_settings.debug_mode {
                    DebugMode::GuiLocal => {
                        let port = connection_manager::get_debug_port();
                        if port > 0 {
                            Some(format!("localhost:{port}"))
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            };
            if let Some(addr) = address {
                return Subscription::batch([
                    coordinator_sub,
                    connection_manager::debug_client_subscribe(addr),
                ]);
            }
        }

        coordinator_sub
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
            #[cfg(feature = "debugger")]
            settings.debug_this_flow,
        );

        info!("Sending submission to Coordinator");
        sender
            .send(ClientMessage::ClientSubmission(Box::new(submission)))
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
        .on_submit(Message::SubmitFlow)
        .style(theme::pill_input)
        .width(iced::Length::FillPortion(7));

        let args = text_input(
            "Space separated flow arguments",
            &self.submission_settings.flow_args,
        )
        .on_submit(Message::SubmitFlow)
        .on_input(Message::FlowArgsChanged)
        .on_paste(Message::FlowArgsChanged)
        .style(theme::pill_input)
        .width(iced::Length::FillPortion(3));

        let max_jobs = text_input("Max jobs", &self.submission_settings.max_jobs_text)
            .on_input(Message::MaxJobsChanged)
            .style(theme::pill_input)
            .width(80);

        let is_client_mode = matches!(self.coordinator_settings, CoordinatorSettings::ClientOnly);
        let can_run = (matches!(self.coordinator_state, CoordinatorState::Connected(_))
            || is_client_mode)
            && !self.running
            && !self.submitted;

        let cmd_icon = |icon_text: &str, label: &str| -> Row<'_, Message> {
            Row::new()
                .spacing(theme::SPACE_SM)
                .align_y(iced::alignment::Vertical::Center)
                .push(
                    Text::new(icon_text.to_string())
                        .font(iced::Font::with_name("icons"))
                        .size(theme::FONT_MD),
                )
                .push(Text::new(label.to_string()))
        };

        let play = if self.running {
            Button::new(cmd_icon("\u{25AA}", "Stop"))
                .on_press(Message::StopFlow)
                .style(theme::styled_button)
                .padding([6, 16])
        } else {
            let mut btn = Button::new(cmd_icon("\u{25B6}", "Play"))
                .style(theme::styled_button)
                .padding([6, 16]);
            if can_run {
                btn = btn.on_press(Message::SubmitFlow);
            }
            btn
        };

        let mut debug_play = Button::new(cmd_icon("\u{F188}", "Debug"))
            .style(theme::styled_button)
            .padding([6, 16]);
        if can_run {
            debug_play = debug_play.on_press(Message::DebugSubmitFlow);
        }

        Row::new()
            .spacing(10)
            .padding(5)
            .align_y(iced::alignment::Vertical::Center)
            .push(url)
            .push(args)
            .push(max_jobs)
            .push(play)
            .push(debug_play)
    }

    fn coordinator_picker_card(&self) -> Card<'_, Message> {
        use iced::widget::scrollable::Scrollable;
        use iced::Length;

        let mut items = Column::new().spacing(4);

        if self.discovering {
            items = items.push(
                Text::new("Discovering services...")
                    .size(14)
                    .color(crate::theme::TEXT_SECONDARY),
            );
        } else if self.discovered_services.is_empty() {
            items = items.push(
                Text::new("No services found on the network")
                    .size(14)
                    .color(crate::theme::TEXT_ERROR),
            );
        } else {
            if self.submission_settings.debug_this_flow {
                items = items.push(
                    Text::new("Select a coordinator, then a debug server")
                        .size(12)
                        .color(crate::theme::TEXT_SECONDARY),
                );
            }
            for (service_type, address) in &self.discovered_services {
                let is_coord_selected =
                    service_type == "Coordinator" && connection_manager::has_discovered_address();
                let label = if is_coord_selected {
                    format!("\u{2714} {service_type}: {address}")
                } else {
                    format!("{service_type}: {address}")
                };
                let btn = Button::new(Text::new(label).size(14))
                    .width(Length::Fill)
                    .padding([6, 10])
                    .style(theme::list_button)
                    .on_press(Message::ServiceSelected(
                        service_type.clone(),
                        address.clone(),
                    ));
                items = items.push(btn);
            }
        }

        let list = Scrollable::new(items)
            .height(Length::Fixed(200.0))
            .width(Length::Fill);

        let body = Column::new().spacing(8).push(list);

        Card::new(Text::new("Discover Coordinators"), body)
            .foot(
                Button::new(Text::new("Close").align_x(Center))
                    .width(Fill)
                    .on_press(Message::CloseCoordinatorPicker)
                    .style(theme::pill_button)
                    .padding([4.0, 8.0]),
            )
            .style(theme::popup_card)
            .max_width(450.0)
    }

    #[cfg(feature = "debugger")]
    fn tip<'a>(content: impl Into<Element<'a, Message>>, hint: &str) -> Element<'a, Message> {
        let tip_content =
            iced::widget::Container::new(Text::new(hint.to_string()).size(theme::FONT_SM))
                .padding([theme::SPACE_SM, theme::SPACE_MD])
                .style(|theme: &iced::Theme| {
                    let palette = theme.palette();
                    iced::widget::container::Style {
                        background: Some(iced::Background::Color(theme::SURFACE_TOOLTIP)),
                        border: iced::Border {
                            color: iced::Color {
                                a: 0.3,
                                ..palette.text
                            },
                            width: 1.0,
                            radius: theme::RADIUS_SM.into(),
                        },
                        text_color: Some(palette.text),
                        ..Default::default()
                    }
                });
        iced::widget::tooltip(
            content,
            tip_content,
            iced::widget::tooltip::Position::Bottom,
        )
        .gap(4)
        .into()
    }

    #[cfg(feature = "debugger")]
    #[allow(clippy::too_many_lines)]
    fn debug_row(&self) -> Row<'_, Message> {
        let can_cmd = self.debug_waiting;

        let jobs_started = connection_manager::get_job_count() > 0;
        let bp = theme::BUTTON_PAD;
        let sp = theme::BUTTON_PAD_SM;

        let icon_btn = |icon_text: &str, label: &str| -> Row<'_, Message> {
            Row::new()
                .spacing(theme::SPACE_SM)
                .align_y(iced::alignment::Vertical::Center)
                .push(
                    Text::new(icon_text.to_string())
                        .font(iced::Font::with_name("icons"))
                        .size(theme::FONT_SM),
                )
                .push(Text::new(label.to_string()))
        };

        let mut continue_btn = Button::new(icon_btn("\u{27A6}", "Continue"))
            .style(theme::styled_button)
            .padding(bp);
        if can_cmd && jobs_started {
            continue_btn = continue_btn.on_press(Message::DebugContinue);
        }

        let mut step_btn = Button::new(icon_btn("\u{F178}", "Step"))
            .style(theme::styled_button)
            .padding(bp);
        if can_cmd {
            step_btn = step_btn.on_press(Message::DebugStep);
        }

        let step_count = text_input("n", &self.debug_step_count)
            .on_input(Message::DebugStepCountChanged)
            .style(theme::pill_input)
            .padding(theme::BUTTON_PAD)
            .width(35);

        let has_run_target = !self.debug_spec_text.trim().is_empty();
        let is_run = has_run_target || !jobs_started;
        let mut reset_btn = Button::new(if is_run {
            icon_btn("\u{25B6}", "Run")
        } else {
            icon_btn("\u{27F3}", "Reset")
        })
        .style(theme::styled_button)
        .padding(bp);
        if self.debug_client_active {
            reset_btn = reset_btn.on_press(if has_run_target {
                Message::DebugRunProcess
            } else {
                Message::DebugReset
            });
        }

        let mut pause_btn = Button::new(icon_btn("\u{2389}", "Pause"))
            .style(theme::styled_button)
            .padding(bp);
        if self.debug_client_active && !can_cmd && jobs_started {
            pause_btn = pause_btn.on_press(Message::DebugPause);
        }

        let mut exit_btn = Button::new(icon_btn("\u{E741}", "Exit"))
            .style(theme::styled_button)
            .padding(bp);
        if can_cmd {
            exit_btn = exit_btn.on_press(Message::DebugExit);
        }

        let spec_input = text_input("spec", &self.debug_spec_text)
            .on_input(Message::DebugSpecChanged)
            .on_submit(Message::DebugSetBreakpoint)
            .style(theme::pill_input)
            .width(100);

        let mut bp_btn = Button::new(Text::new("Set BP"))
            .style(theme::styled_button)
            .padding(sp);
        if can_cmd {
            bp_btn = bp_btn.on_press(Message::ShowBpPopup);
        }

        let mut del_btn = Button::new(Text::new("Del All"))
            .style(theme::styled_button)
            .padding(sp);
        if can_cmd {
            del_btn = del_btn.on_press(Message::DebugDeleteBreakpoints);
        }

        let mut list_btn = Button::new(Text::new("List BPs"))
            .style(theme::styled_button)
            .padding(sp);
        if can_cmd {
            list_btn = list_btn.on_press(Message::DebugListBreakpoints);
        }

        let mut inspect_btn = Button::new(Text::new("Inspect"))
            .style(theme::styled_button)
            .padding(sp);
        if can_cmd {
            inspect_btn = inspect_btn.on_press(if self.debug_spec_text.is_empty() {
                Message::ShowInspectPopup
            } else {
                Message::DebugInspect
            });
        }

        let mut funcs_btn = Button::new(Text::new("Functions"))
            .style(theme::styled_button)
            .padding(sp);
        if can_cmd {
            funcs_btn = funcs_btn.on_press(Message::DebugFunctions(true));
        }

        let mut flows_btn = Button::new(Text::new("Flows"))
            .style(theme::styled_button)
            .padding(sp);
        if can_cmd {
            flows_btn = flows_btn.on_press(Message::DebugFlows);
        }

        let mut procs_btn = Button::new(Text::new("Processes"))
            .style(theme::styled_button)
            .padding(sp);
        if can_cmd {
            procs_btn = procs_btn.on_press(Message::DebugProcesses);
        }

        let mut state_btn = Button::new(Text::new("State"))
            .style(theme::styled_button)
            .padding(sp);
        if can_cmd {
            state_btn = state_btn.on_press(Message::DebugState);
        }

        let mut validate_btn = Button::new(Text::new("Validate"))
            .style(theme::styled_button)
            .padding(sp);
        if can_cmd {
            validate_btn = validate_btn.on_press(Message::DebugValidate);
        }

        Row::new()
            .spacing(4)
            .padding(4)
            .align_y(iced::alignment::Vertical::Center)
            .push(Self::tip(
                continue_btn,
                "Continue execution until next breakpoint",
            ))
            .push(Self::tip(step_btn, "Execute the next job(s) then pause"))
            .push(Self::tip(step_count, "Number of jobs to step"))
            .push(Self::tip(pause_btn, "Pause execution and enter debugger"))
            .push(Self::tip(
                reset_btn,
                if has_run_target {
                    "Run a specific process (spec field has target)"
                } else {
                    "Reset flow state and re-run from start"
                },
            ))
            .push(Self::tip(
                spec_input,
                "Enter spec: breakpoint (3, 3+, 1:0) or run target (ID, /route, name [args])",
            ))
            .push(Self::tip(bp_btn, "Open breakpoint picker"))
            .push(Self::tip(del_btn, "Delete all breakpoints"))
            .push(Self::tip(list_btn, "List active breakpoints"))
            .push(Self::tip(
                inspect_btn,
                "Inspect state (use spec field for specific target)",
            ))
            .push(Self::tip(funcs_btn, "List all functions"))
            .push(Self::tip(flows_btn, "List all flows"))
            .push(Self::tip(procs_btn, "Show flow/function hierarchy"))
            .push(Self::tip(
                state_btn,
                "Show runtime state (jobs, completed, busy)",
            ))
            .push(Self::tip(validate_btn, "Validate flow state for deadlocks"))
            .push(iced::widget::container(iced::widget::text("")).width(iced::Length::Fill))
            .push(Self::tip(exit_btn, "Stop execution and exit debugger"))
    }

    #[cfg(feature = "debugger")]
    fn run_input_row(&self) -> Row<'_, Message> {
        let mut row = Row::new()
            .spacing(6)
            .padding([4, 8])
            .align_y(iced::alignment::Vertical::Center);

        for (i, name) in self.run_input_names.iter().enumerate() {
            let value = self.run_input_values.get(i).cloned().unwrap_or_default();
            let type_hint = self.run_input_types.get(i).map_or("Value", String::as_str);
            let tooltip = type_hint.to_string();
            let idx = i;
            let input = Self::tip(
                text_input(name, &value)
                    .on_input(move |v| Message::RunInputChanged(idx, v))
                    .on_submit(Message::RunInputExecute)
                    .style(theme::pill_input)
                    .width(100),
                &tooltip,
            );
            row = row.push(input);
        }

        let exec_btn = Button::new(Text::new("Execute"))
            .style(theme::styled_button)
            .padding([3, 8])
            .on_press(Message::RunInputExecute);
        let cancel_btn = Button::new(Text::new("Cancel"))
            .style(theme::styled_button)
            .padding([3, 8])
            .on_press(Message::RunInputCancel);

        row = row.push(exec_btn).push(cancel_btn);
        row
    }

    #[cfg(feature = "debugger")]
    #[allow(clippy::too_many_lines)]
    fn bp_popup_card(&self) -> Card<'_, Message> {
        use iced::widget::scrollable::Scrollable;
        use iced::Length;

        let type_row = Row::new()
            .spacing(4)
            .align_y(iced::alignment::Vertical::Center);
        let type_row = BP_TABS.iter().fold(type_row, |row, &bt| {
            let is_active = bt == self.bp_tab;
            let mut btn = Button::new(Text::new(bt.to_string()).size(theme::FONT_MD))
                .padding(theme::BUTTON_PAD)
                .style(if is_active {
                    theme::pill_button_active
                        as fn(
                            &iced::Theme,
                            iced::widget::button::Status,
                        ) -> iced::widget::button::Style
                } else {
                    theme::pill_button
                });
            if !is_active {
                btn = btn.on_press(Message::BpTabChanged(bt));
            }
            row.push(btn)
        });

        let mut items = Column::new().spacing(2);

        if self.cached_functions.is_empty() {
            items = items.push(
                Text::new("Loading function list...")
                    .size(13)
                    .color(crate::theme::TEXT_SECONDARY),
            );
        } else {
            let bp_marker = |spec: &str| -> &str {
                if self.active_breakpoints.contains(spec) {
                    "\u{1F534} "
                } else {
                    "  "
                }
            };

            match self.bp_tab {
                BpTab::Before | BpTab::After => {
                    for f in &self.cached_functions {
                        let spec = match self.bp_tab {
                            BpTab::Before => format!("{}", f.id),
                            BpTab::After => format!("{}+", f.id),
                            _ => unreachable!(),
                        };
                        let marker = bp_marker(&spec);
                        let label = format!("{marker}#{} '{}' @ '{}'", f.id, f.name, f.route);
                        let mut btn = Button::new(Text::new(label).size(13))
                            .width(Length::Fill)
                            .padding([3, 8]);
                        if self.active_breakpoints.contains(&spec) {
                            btn = btn.style(theme::styled_button);
                        } else {
                            btn = btn.style(theme::list_button);
                        }
                        btn = btn.on_press(Message::BpTargetChanged(spec.clone()));
                        items = items.push(btn);
                    }
                }
                BpTab::Route => {
                    for f in &self.cached_functions {
                        let before_spec = format!("{}", f.id);
                        let after_spec = format!("{}+", f.id);
                        let has_before = self.active_breakpoints.contains(&before_spec);
                        let has_after = self.active_breakpoints.contains(&after_spec);
                        let before_dot = if has_before { "\u{1F534}" } else { "  " };
                        let after_dot = if has_after { " \u{1F534}" } else { "" };
                        let label = format!(
                            "{before_dot} {} (#{} '{}'){after_dot}",
                            f.route, f.id, f.name
                        );
                        let mut btn = Button::new(
                            Text::new(label)
                                .size(13)
                                .shaping(iced::widget::text::Shaping::Advanced),
                        )
                        .width(Length::Fill)
                        .padding([3, 8]);
                        if has_before || has_after {
                            btn = btn.style(theme::styled_button);
                        } else {
                            btn = btn.style(theme::list_button);
                        }
                        btn = btn.on_press(Message::BpCycleFunction(f.id));
                        items = items.push(btn);

                        for (idx, input_name, _generic) in &f.inputs {
                            let input_spec = format!("{}:{idx}", f.id);
                            let name_part = if input_name.is_empty() {
                                String::new()
                            } else {
                                format!(" '{input_name}'")
                            };
                            let im = bp_marker(&input_spec);
                            let input_label =
                                format!("  {im}input:{idx}{name_part} on '{}'", f.route);
                            let mut ibtn = Button::new(Text::new(input_label).size(12))
                                .width(Length::Fill)
                                .padding([2, 16]);
                            if self.active_breakpoints.contains(&input_spec) {
                                ibtn = ibtn.style(theme::styled_button);
                            } else {
                                ibtn = ibtn.style(theme::list_button);
                            }
                            ibtn = ibtn.on_press(Message::BpTargetChanged(input_spec));
                            items = items.push(ibtn);
                        }
                        for output_route in &f.outputs {
                            let out_spec = format!("{}{output_route}", f.id);
                            let om = bp_marker(&out_spec);
                            let out_label =
                                format!("  {om}output:'{output_route}' on '{}'", f.route);
                            let mut obtn = Button::new(Text::new(out_label).size(12))
                                .width(Length::Fill)
                                .padding([2, 16]);
                            if self.active_breakpoints.contains(&out_spec) {
                                obtn = obtn.style(theme::styled_button);
                            } else {
                                obtn = obtn.style(theme::list_button);
                            }
                            obtn = obtn.on_press(Message::BpTargetChanged(out_spec));
                            items = items.push(obtn);
                        }
                    }
                }
                BpTab::Input => {
                    for f in &self.cached_functions {
                        for (idx, input_name, _generic) in &f.inputs {
                            let spec = format!("{}:{idx}", f.id);
                            let marker = bp_marker(&spec);
                            let name_part = if input_name.is_empty() {
                                String::new()
                            } else {
                                format!(" '{input_name}'")
                            };
                            let label =
                                format!("{marker}#{} '{}' input:{idx}{name_part}", f.id, f.name);
                            let mut btn = Button::new(Text::new(label).size(13))
                                .width(Length::Fill)
                                .padding([3, 8]);
                            if self.active_breakpoints.contains(&spec) {
                                btn = btn.style(theme::styled_button);
                            } else {
                                btn = btn.style(theme::list_button);
                            }
                            btn = btn.on_press(Message::BpTargetChanged(spec));
                            items = items.push(btn);
                        }
                    }
                }
                BpTab::Output => {
                    for f in &self.cached_functions {
                        for output_route in &f.outputs {
                            let spec = format!("{}{output_route}", f.id);
                            let marker = bp_marker(&spec);
                            let label =
                                format!("{marker}#{} '{}' output:'{output_route}'", f.id, f.name);
                            let mut btn = Button::new(Text::new(label).size(13))
                                .width(Length::Fill)
                                .padding([3, 8]);
                            if self.active_breakpoints.contains(&spec) {
                                btn = btn.style(theme::styled_button);
                            } else {
                                btn = btn.style(theme::list_button);
                            }
                            btn = btn.on_press(Message::BpTargetChanged(spec));
                            items = items.push(btn);
                        }
                    }
                }
            }
        }

        let list = Scrollable::new(items)
            .height(Length::Fixed(200.0))
            .width(Length::Fill);

        let body = Column::new().spacing(8).push(type_row).push(list);

        let bp_header = Row::new()
            .push(Text::new("Breakpoints"))
            .push(iced::widget::container(Text::new("")).width(iced::Length::Fill))
            .push(
                Button::new(
                    Text::new("\u{2715}")
                        .font(iced::Font::with_name("icons"))
                        .size(theme::FONT_MD),
                )
                .on_press(Message::CloseBpPopup)
                .style(theme::list_button)
                .padding(theme::BUTTON_PAD_SM),
            )
            .align_y(iced::alignment::Vertical::Center);

        Card::new(bp_header, body)
            .style(theme::popup_card)
            .max_width(500.0)
    }

    #[cfg(feature = "debugger")]
    #[allow(clippy::too_many_lines)]
    fn inspect_popup_card(&self) -> Card<'_, Message> {
        use iced::widget::scrollable::Scrollable;
        use iced::Length;

        let tab_row = Row::new()
            .spacing(4)
            .align_y(iced::alignment::Vertical::Center);
        let tab_row = INSPECT_TABS.iter().fold(tab_row, |row, &tab| {
            let is_active = tab == self.inspect_tab;
            let mut btn = Button::new(Text::new(tab.to_string()).size(theme::FONT_MD))
                .padding(theme::BUTTON_PAD)
                .style(if is_active {
                    theme::pill_button_active
                        as fn(
                            &iced::Theme,
                            iced::widget::button::Status,
                        ) -> iced::widget::button::Style
                } else {
                    theme::pill_button
                });
            if !is_active {
                btn = btn.on_press(Message::InspectTabChanged(tab));
            }
            row.push(btn)
        });

        let mut items = Column::new().spacing(2);

        match self.inspect_tab {
            InspectTab::State => {
                for state_name in &["ready", "waiting", "running", "completed", "blocked"] {
                    let btn = Button::new(Text::new(format!("  {state_name}")).size(13))
                        .width(Length::Fill)
                        .padding([3, 8])
                        .style(theme::list_button)
                        .on_press(Message::InspectPopupSelect((*state_name).to_string()));
                    items = items.push(btn);
                }
            }
            InspectTab::Function => {
                for f in &self.cached_functions {
                    let btn = Button::new(
                        Text::new(format!("#{} '{}' @ '{}'", f.id, f.name, f.route)).size(13),
                    )
                    .width(Length::Fill)
                    .padding([3, 8])
                    .style(theme::list_button)
                    .on_press(Message::InspectPopupSelect(format!("{}", f.id)));
                    items = items.push(btn);
                }
            }
            InspectTab::Flow => {
                for f in &self.cached_functions {
                    let btn = Button::new(
                        Text::new(format!("#{} '{}' @ {}", f.id, f.name, f.route)).size(13),
                    )
                    .width(Length::Fill)
                    .padding([3, 8])
                    .style(theme::list_button)
                    .on_press(Message::InspectPopupSelect(format!("{}", f.id)));
                    items = items.push(btn);
                }
            }
            InspectTab::Input => {
                for f in &self.cached_functions {
                    for (idx, input_name, _generic) in &f.inputs {
                        let name_part = if input_name.is_empty() {
                            String::new()
                        } else {
                            format!(" '{input_name}'")
                        };
                        let btn = Button::new(
                            Text::new(format!("#{} '{}' input:{idx}{name_part}", f.id, f.name))
                                .size(13),
                        )
                        .width(Length::Fill)
                        .padding([3, 8])
                        .style(theme::list_button)
                        .on_press(Message::InspectPopupSelect(format!("{}:{idx}", f.id)));
                        items = items.push(btn);
                    }
                }
            }
            InspectTab::Output => {
                for f in &self.cached_functions {
                    for output_route in &f.outputs {
                        let btn = Button::new(
                            Text::new(format!("#{} '{}' output:'{output_route}'", f.id, f.name))
                                .size(13),
                        )
                        .width(Length::Fill)
                        .padding([3, 8])
                        .style(theme::list_button)
                        .on_press(Message::InspectPopupSelect(format!(
                            "{}{output_route}",
                            f.id
                        )));
                        items = items.push(btn);
                    }
                }
            }
            InspectTab::Route => {
                for f in &self.cached_functions {
                    let btn = Button::new(
                        Text::new(format!("{} (#{} '{}')", f.route, f.id, f.name)).size(13),
                    )
                    .width(Length::Fill)
                    .padding([3, 8])
                    .style(theme::list_button)
                    .on_press(Message::InspectPopupSelect(f.route.clone()));
                    items = items.push(btn);
                }
            }
        }

        let list = Scrollable::new(items)
            .height(Length::Fixed(200.0))
            .width(Length::Fill);

        let body = Column::new().spacing(8).push(tab_row).push(list);

        let inspect_header = Row::new()
            .push(Text::new("Inspect"))
            .push(iced::widget::container(Text::new("")).width(iced::Length::Fill))
            .push(
                Button::new(
                    Text::new("\u{2715}")
                        .font(iced::Font::with_name("icons"))
                        .size(theme::FONT_MD),
                )
                .on_press(Message::CloseInspectPopup)
                .style(theme::list_button)
                .padding(theme::BUTTON_PAD_SM),
            )
            .align_y(iced::alignment::Vertical::Center);

        Card::new(inspect_header, body)
            .style(theme::popup_card)
            .max_width(500.0)
    }

    fn status_bar(&self) -> Column<'_, Message> {
        let (indicator, status) = match &self.coordinator_state {
            CoordinatorState::Disconnected(reason) => {
                ("\u{1F534}", format!("Disconnected({reason})"))
            }
            CoordinatorState::Connected(_) => match (self.submitted, self.running) {
                (false, false) => ("\u{1F7E2}", "Ready".to_string()),
                (_, true) => {
                    #[cfg(feature = "debugger")]
                    if self.debug_waiting {
                        ("\u{1F7E3}", "Debugging".to_string())
                    } else {
                        ("\u{1F535}", "Running".to_string())
                    }
                    #[cfg(not(feature = "debugger"))]
                    {
                        ("\u{1F535}", "Running".to_string())
                    }
                }
                (true, false) => {
                    #[cfg(feature = "debugger")]
                    if self.debug_client_active {
                        ("\u{1F7E3}", "Debugging".to_string())
                    } else if self.submission_settings.debug_this_flow {
                        (
                            "\u{1F7E0}",
                            "Waiting for debugger to connect...".to_string(),
                        )
                    } else {
                        ("\u{1F7E1}", "Submitted".to_string())
                    }
                    #[cfg(not(feature = "debugger"))]
                    {
                        ("\u{1F7E1}", "Submitted".to_string())
                    }
                }
            },
        };

        let mut row = Row::new()
            .spacing(8)
            .align_y(iced::alignment::Vertical::Center)
            .push(Text::new(indicator))
            .push(Text::new(status).size(13));
        if self.running {
            row = row
                .push(Text::new(format!("Jobs: {}", connection_manager::get_job_count())).size(13));
        }

        #[cfg(feature = "debugger")]
        if self.submission_settings.debug_mode == DebugMode::External && !self.debug_client_active {
            let debug_port = connection_manager::get_debug_port();
            if debug_port > 0 {
                row = row.push(
                    Text::new(format!("flowrdb --address localhost:{debug_port}"))
                        .size(13)
                        .color(crate::theme::TEXT_LINK),
                );
            }
        }

        Column::new().push(
            iced::widget::Container::new(row)
                .padding([theme::SPACE_XS, theme::SPACE_MD])
                .width(iced::Length::Fill),
        )
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

        #[cfg(feature = "debugger")]
        let debug_mode = if matches.get_flag("external-debugger") {
            DebugMode::External
        } else if matches.get_flag("debugger") {
            DebugMode::GuiLocal
        } else {
            DebugMode::Off
        };
        #[cfg(feature = "debugger")]
        let debug_this_flow = debug_mode != DebugMode::Off;
        #[cfg(not(feature = "debugger"))]
        let debug_this_flow = false;

        let coordinator_settings = if matches.get_flag("client") {
            CoordinatorSettings::ClientOnly
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
        let mut auto_start = auto || matches.get_flag("auto-start");
        #[cfg(feature = "debugger")]
        if debug_mode != DebugMode::Off
            && !matches!(coordinator_settings, CoordinatorSettings::ClientOnly)
        {
            auto_start = true;
        }

        (
            SubmissionSettings {
                flow_manifest_url,
                flow_args,
                max_jobs_text: parallel_jobs_limit.map_or(String::new(), |n| n.to_string()),
                debug_this_flow,
                display_metrics: matches.get_flag("metrics"),
                parallel_jobs_limit,
                #[cfg(feature = "debugger")]
                debug_mode,
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

        let app = app
            .arg(
                Arg::new("debugger")
                    .short('d')
                    .long("debugger")
                    .action(clap::ArgAction::SetTrue)
                    .help("Enable debugging (use with -c for remote, or alone for local)"),
            )
            .arg(
                Arg::new("external-debugger")
                    .long("external-debugger")
                    .action(clap::ArgAction::SetTrue)
                    .help("Start debug server for external flowrdb to connect"),
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
                .action(clap::ArgAction::SetTrue)
                .help("Client only — discover and connect to a remote coordinator"),
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

    #[cfg(feature = "debugger")]
    fn debug_separator(&mut self, label: &str) {
        self.tab_set.debug_tab.push(DebugEventLine::separator(
            label.to_string(),
            crate::theme::debug_colors::SEPARATOR,
        ));
    }

    #[cfg(feature = "debugger")]
    #[cfg(feature = "debugger")]
    fn ensure_functions_cached(&mut self, action: Message) -> bool {
        if self.cached_functions.is_empty() {
            self.pending_action = Some(action);
            self.suppress_next_output = true;
            connection_manager::send_debug_command(
                flowcore::model::debug_command::DebugCommand::FunctionList,
            );
            false
        } else {
            true
        }
    }

    #[cfg(feature = "debugger")]
    #[allow(clippy::too_many_lines)]
    fn process_debug_message(&mut self, message: Message) -> Task<Message> {
        use flowcore::model::debug_command::{BreakpointSpec, DebugCommand};

        match message {
            Message::DebugEvent(lines) => {
                if self.suppress_next_output {
                    self.suppress_next_output = false;
                } else {
                    for line in lines {
                        self.tab_set.debug_tab.push(line);
                    }
                    if self.tab_set.active_tab != 5 {
                        self.tab_set.debug_tab.unread_count += 1;
                    }
                    if self.tab_set.debug_tab.auto_scroll {
                        return operation::snap_to(
                            self.tab_set.debug_tab.id.clone(),
                            RelativeOffset::END,
                        );
                    }
                }
            }
            Message::DebugWaiting => {
                self.debug_waiting = true;
            }
            Message::DebugConnected => {
                self.tab_set
                    .debug_tab
                    .push_text("Connected to debug server".into());
                self.tab_set.active_tab = 5;
            }
            Message::DebugDisconnected(reason) => {
                self.debug_waiting = false;
                self.debug_client_active = false;
                self.tab_set.debug_tab.push(DebugEventLine::new(
                    format!("Disconnected: {reason}"),
                    Some(crate::theme::debug_colors::ERROR),
                ));
            }
            Message::DebugContinue => {
                self.debug_waiting = false;
                self.debug_separator("Continue");
                connection_manager::send_debug_command(DebugCommand::Continue);
            }
            Message::DebugStep => {
                self.debug_waiting = false;
                let params = if self.debug_step_count.is_empty() {
                    None
                } else {
                    Some(vec![self.debug_step_count.clone()])
                };
                let count = DebugClient::parse_optional_int(params);
                if let Some(n) = count {
                    self.debug_separator(&format!("Step ({n})"));
                } else {
                    self.debug_separator("Step");
                }
                connection_manager::send_debug_command(DebugCommand::Step(count));
            }
            Message::DebugReset => {
                self.debug_waiting = false;
                connection_manager::set_job_count(0);
                self.debug_separator("Run / Reset");
                connection_manager::send_debug_command(DebugCommand::RunReset(None, vec![]));
            }
            Message::DebugRunProcess => {
                if self.debug_spec_text.trim().is_empty() {
                    return iced::Task::none();
                }
                if !self.ensure_functions_cached(Message::DebugRunProcess) {
                    return iced::Task::none();
                }
                let parts: Vec<String> = self
                    .debug_spec_text
                    .split_whitespace()
                    .map(String::from)
                    .collect();
                if let Some(target_str) = parts.first() {
                    let target_id = if let Ok(id) = target_str.parse::<usize>() {
                        Some(id)
                    } else if target_str.starts_with('/') {
                        self.cached_functions
                            .iter()
                            .find(|f| f.route == *target_str)
                            .map(|f| f.id)
                    } else {
                        self.cached_functions
                            .iter()
                            .find(|f| f.name == *target_str)
                            .map(|f| f.id)
                    };
                    if let Some(id) = target_id {
                        if let Some(func) = self.cached_functions.iter().find(|f| f.id == id) {
                            let inline_args: Vec<String> =
                                parts.get(1..).unwrap_or_default().to_vec();
                            self.run_input_names = func
                                .inputs
                                .iter()
                                .enumerate()
                                .map(|(i, (_, name, _))| {
                                    if name.is_empty() {
                                        format!("input_{i}")
                                    } else {
                                        name.clone()
                                    }
                                })
                                .collect();
                            self.run_input_types = func
                                .inputs
                                .iter()
                                .map(|(_, _, generic)| {
                                    if *generic {
                                        "Generic".to_string()
                                    } else {
                                        "Value".to_string()
                                    }
                                })
                                .collect();
                            self.run_input_values = (0..func.inputs.len())
                                .map(|i| inline_args.get(i).cloned().unwrap_or_default())
                                .collect();
                            self.run_target_id = Some(id);
                            self.show_run_inputs = true;
                        } else {
                            let args: Vec<String> = parts.get(1..).unwrap_or_default().to_vec();
                            self.debug_waiting = false;
                            self.debug_spec_text.clear();
                            self.debug_separator(&format!("Run process #{id}"));
                            connection_manager::send_debug_command(DebugCommand::RunReset(
                                Some(flowcore::model::debug_command::ProcessTarget::Id(id)),
                                args,
                            ));
                        }
                    } else {
                        self.debug_waiting = false;
                        let target = if target_str.starts_with('/') {
                            flowcore::model::debug_command::ProcessTarget::Route(target_str.clone())
                        } else {
                            flowcore::model::debug_command::ProcessTarget::Name(target_str.clone())
                        };
                        self.debug_separator(&format!("Run process: {target_str}"));
                        connection_manager::send_debug_command(DebugCommand::RunReset(
                            Some(target),
                            vec![],
                        ));
                    }
                }
            }
            Message::RunInputChanged(index, value) => {
                if let Some(v) = self.run_input_values.get_mut(index) {
                    *v = value;
                }
            }
            Message::RunInputExecute => {
                self.show_run_inputs = false;
                if let Some(id) = self.run_target_id.take() {
                    let args = self.run_input_values.clone();
                    self.debug_waiting = false;
                    self.debug_spec_text.clear();
                    self.debug_separator(&format!("Run process #{id}"));
                    connection_manager::send_debug_command(DebugCommand::RunReset(
                        Some(flowcore::model::debug_command::ProcessTarget::Id(id)),
                        args,
                    ));
                }
            }
            Message::RunInputCancel => {
                self.show_run_inputs = false;
                self.run_target_id = None;
                self.run_input_values.clear();
                self.run_input_names.clear();
            }
            Message::DebugPause => {
                self.debug_separator("Pause");
                self.send(ClientMessage::EnterDebugger);
            }
            Message::DebugExit => {
                self.debug_waiting = false;
                self.debug_separator("Exit Debugger");
                connection_manager::send_debug_command(DebugCommand::ExitDebugger);
            }
            Message::DebugStepCountChanged(value) => self.debug_step_count = value,
            Message::DebugSpecChanged(value) => self.debug_spec_text = value,
            Message::DebugSetBreakpoint => {
                self.debug_waiting = false;
                let params = if self.debug_spec_text.is_empty() {
                    None
                } else {
                    Some(vec![self.debug_spec_text.clone()])
                };
                let spec_label = if self.debug_spec_text.is_empty() {
                    "no spec".to_string()
                } else {
                    self.debug_spec_text.clone()
                };
                self.debug_separator(&format!("Set Breakpoint ({spec_label})"));
                if !self.debug_spec_text.is_empty() {
                    self.active_breakpoints.insert(self.debug_spec_text.clone());
                }
                let spec = DebugClient::parse_breakpoint_spec(params);
                connection_manager::send_debug_command(DebugCommand::Breakpoint(spec));
            }
            Message::DebugDeleteBreakpoints => {
                self.debug_waiting = false;
                self.active_breakpoints.clear();
                self.debug_separator("Delete All Breakpoints");
                connection_manager::send_debug_command(DebugCommand::Delete(Some(
                    BreakpointSpec::All,
                )));
            }
            Message::DebugListBreakpoints => {
                self.debug_waiting = false;
                self.debug_separator("List Breakpoints");
                connection_manager::send_debug_command(DebugCommand::List);
            }
            Message::DebugInspect => {
                self.debug_waiting = false;
                let params = if self.debug_spec_text.is_empty() {
                    None
                } else {
                    Some(vec![self.debug_spec_text.clone()])
                };
                if let Some(cmd) = DebugClient::parse_inspect_spec(params) {
                    let label = if self.debug_spec_text.is_empty() {
                        "Inspect (overall flow state)".to_string()
                    } else {
                        format!("Inspect ({})", self.debug_spec_text)
                    };
                    self.debug_separator(&label);
                    connection_manager::send_debug_command(cmd);
                } else {
                    self.debug_waiting = true;
                }
            }
            Message::DebugFunctions(display) => {
                if display {
                    self.debug_waiting = false;
                    self.debug_separator("Functions List");
                }
                self.suppress_next_output = !display;
                connection_manager::send_debug_command(DebugCommand::FunctionList);
            }
            Message::DebugFlows => {
                self.debug_waiting = false;
                connection_manager::set_flows_only(true);
                self.debug_separator("Flows");
                connection_manager::send_debug_command(DebugCommand::ProcessList);
            }
            Message::DebugState => {
                self.debug_waiting = false;
                self.debug_separator("Runtime State");
                connection_manager::send_debug_command(DebugCommand::Inspect);
            }
            Message::DebugProcesses => {
                self.debug_waiting = false;
                self.debug_separator("Process Tree");
                connection_manager::send_debug_command(DebugCommand::ProcessList);
            }
            Message::DebugValidate => {
                self.debug_waiting = false;
                self.debug_separator("Validate");
                connection_manager::send_debug_command(DebugCommand::Validate);
            }
            Message::DebugFunctionListReceived(functions) => {
                self.cached_functions = functions;
                if let Some(action) = self.pending_action.take() {
                    return self.process_debug_message(action);
                }
            }
            Message::DebugBreakpointListReceived(specs) => {
                self.active_breakpoints = specs.into_iter().collect();
            }
            Message::DebugInspectLink(ref spec) if self.debug_waiting => {
                self.debug_waiting = false;
                let label = if spec.parse::<usize>().is_ok() {
                    format!("Inspect #{spec}")
                } else if spec.contains(':') {
                    format!("Inspect Input ({spec})")
                } else if spec.starts_with('/') {
                    format!("Inspect Route ({spec})")
                } else if spec.contains('/') {
                    format!("Inspect Output ({spec})")
                } else {
                    format!("Inspect {spec}")
                };
                let params = Some(vec![spec.clone()]);
                if let Some(cmd) = DebugClient::parse_inspect_spec(params) {
                    self.debug_separator(&label);
                    connection_manager::send_debug_command(cmd);
                } else {
                    self.debug_waiting = true;
                }
            }
            Message::DebugToggleSection(section_id) => {
                self.tab_set.debug_tab.toggle_section(section_id);
            }
            Message::ShowInspectPopup => {
                if !self.ensure_functions_cached(Message::ShowInspectPopup) {
                    return iced::Task::none();
                }
                self.show_inspect_popup = true;
            }
            Message::CloseInspectPopup => self.show_inspect_popup = false,
            Message::InspectTabChanged(tab) => self.inspect_tab = tab,
            Message::InspectPopupSelect(spec) => {
                self.show_inspect_popup = false;
                if self.debug_waiting {
                    self.debug_waiting = false;
                    if spec.is_empty() {
                        self.debug_separator("Inspect (overall flow state)");
                        connection_manager::send_debug_command(
                            flowcore::model::debug_command::DebugCommand::Inspect,
                        );
                    } else {
                        let label = if spec.parse::<usize>().is_ok() {
                            format!("Inspect #{spec}")
                        } else if spec.starts_with('/') {
                            format!("Inspect Route ({spec})")
                        } else if spec.contains(':') {
                            format!("Inspect Input ({spec})")
                        } else if spec.contains('/') {
                            format!("Inspect Output ({spec})")
                        } else {
                            format!("Inspect {spec}")
                        };
                        let params = Some(vec![spec]);
                        if let Some(cmd) = DebugClient::parse_inspect_spec(params) {
                            self.debug_separator(&label);
                            connection_manager::send_debug_command(cmd);
                        }
                    }
                }
            }
            Message::ShowBpPopup => {
                if !self.ensure_functions_cached(Message::ShowBpPopup) {
                    return iced::Task::none();
                }
                self.show_bp_popup = true;
                self.bp_target.clear();
                if self.debug_waiting {
                    self.debug_waiting = false;
                    self.suppress_next_output = true;
                    connection_manager::send_debug_command(DebugCommand::List);
                }
            }
            Message::CloseBpPopup => self.show_bp_popup = false,
            Message::BpTabChanged(tab) => {
                self.bp_tab = tab;
                self.bp_target.clear();
            }
            Message::BpTargetChanged(value) => {
                if self.debug_waiting {
                    let spec_str = if self.bp_tab == BpTab::After {
                        format!("{value}+")
                    } else {
                        value.clone()
                    };
                    if self.active_breakpoints.contains(&spec_str) {
                        self.active_breakpoints.remove(&spec_str);
                        self.debug_waiting = false;
                        let spec = DebugClient::parse_breakpoint_spec(Some(vec![spec_str.clone()]));
                        self.debug_separator(&format!("Delete Breakpoint ({spec_str})"));
                        connection_manager::send_debug_command(DebugCommand::Delete(spec));
                    } else {
                        self.active_breakpoints.insert(spec_str.clone());
                        self.debug_waiting = false;
                        let spec = DebugClient::parse_breakpoint_spec(Some(vec![spec_str.clone()]));
                        self.debug_separator(&format!("Set Breakpoint ({spec_str})"));
                        connection_manager::send_debug_command(DebugCommand::Breakpoint(spec));
                    }
                }
                self.bp_target = value;
            }
            Message::BpPopupConfirm => {
                self.show_bp_popup = false;
            }
            Message::BpCycleFunction(func_id) if self.debug_waiting => {
                let before_spec = format!("{func_id}");
                let after_spec = format!("{func_id}+");
                let has_before = self.active_breakpoints.contains(&before_spec);
                let has_after = self.active_breakpoints.contains(&after_spec);

                self.debug_waiting = false;
                match (has_before, has_after) {
                    (true, true) => {
                        self.active_breakpoints.remove(&before_spec);
                        let spec = DebugClient::parse_breakpoint_spec(Some(vec![before_spec]));
                        self.debug_separator("Remove before breakpoint");
                        connection_manager::send_debug_command(DebugCommand::Delete(spec));
                    }
                    (false, true) => {
                        self.active_breakpoints.remove(&after_spec);
                        let spec = DebugClient::parse_breakpoint_spec(Some(vec![after_spec]));
                        self.debug_separator("Remove after breakpoint");
                        connection_manager::send_debug_command(DebugCommand::Delete(spec));
                    }
                    (false, false) => {
                        self.active_breakpoints.insert(before_spec.clone());
                        let spec = DebugClient::parse_breakpoint_spec(Some(vec![before_spec]));
                        self.debug_separator("Set before breakpoint");
                        connection_manager::send_debug_command(DebugCommand::Breakpoint(spec));
                    }
                    (true, false) => {
                        self.active_breakpoints.insert(after_spec.clone());
                        let spec = DebugClient::parse_breakpoint_spec(Some(vec![after_spec]));
                        self.debug_separator("Set after breakpoint");
                        connection_manager::send_debug_command(DebugCommand::Breakpoint(spec));
                    }
                }
            }
            _ => {}
        }
        Task::none()
    }

    #[allow(clippy::too_many_lines)]
    fn process_coordinator_message(&mut self, message: CoordinatorMessage) -> Task<Message> {
        match message {
            CoordinatorMessage::Connected(_) => {
                self.error("Coordinator is already connected");
            }
            CoordinatorMessage::FlowStart => {
                self.running = true;
                self.submitted = false;
                connection_manager::set_job_count(0);
                self.send(ClientMessage::Ack);
            }
            CoordinatorMessage::FlowEnd(metrics) => {
                self.running = false;
                self.submitted = false;
                self.pending_getline = false;
                connection_manager::set_job_count(0);
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
            CoordinatorMessage::ImageWrite(grid, ref name) => {
                let height = u32::try_from(grid.len()).unwrap_or(0);
                let width = grid
                    .first()
                    .map_or(0, |row| u32::try_from(row.len()).unwrap_or(0));
                let data = self
                    .tab_set
                    .images_tab
                    .images
                    .entry(name.clone())
                    .or_insert_with(|| ImageReference {
                        width,
                        height,
                        data: RgbaImage::new(width, height),
                    });
                for (y, row) in grid.iter().enumerate() {
                    for (x, &val) in row.iter().enumerate() {
                        let gray = val;
                        data.data.put_pixel(
                            u32::try_from(x).unwrap_or(0),
                            u32::try_from(y).unwrap_or(0),
                            Rgba([gray, gray, gray, 255]),
                        );
                    }
                }
                if self.tab_set.active_tab != 3 {
                    self.tab_set.images_tab.new_activity = true;
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
            CoordinatorMessage::Disconnected(reason) => {
                self.coordinator_state = crate::CoordinatorState::Disconnected(reason.clone());
                self.running = false;
                self.error(&reason);
            }
            CoordinatorMessage::Invalid => {}
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
                #[cfg(feature = "debugger")]
                debug_mode: DebugMode::Off,
            },
            coordinator_settings: CoordinatorSettings::ClientOnly,
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
            #[cfg(feature = "debugger")]
            debug_waiting: false,
            #[cfg(feature = "debugger")]
            debug_spec_text: String::new(),
            #[cfg(feature = "debugger")]
            debug_step_count: String::new(),
            #[cfg(feature = "debugger")]
            debug_client_active: false,
            #[cfg(feature = "debugger")]
            show_bp_popup: false,
            #[cfg(feature = "debugger")]
            bp_tab: BpTab::Before,
            #[cfg(feature = "debugger")]
            bp_target: String::new(),
            #[cfg(feature = "debugger")]
            cached_functions: Vec::new(),
            #[cfg(feature = "debugger")]
            active_breakpoints: std::collections::HashSet::new(),
            #[cfg(feature = "debugger")]
            show_inspect_popup: false,
            #[cfg(feature = "debugger")]
            suppress_next_output: false,
            #[cfg(feature = "debugger")]
            inspect_tab: InspectTab::State,
            #[cfg(feature = "debugger")]
            show_run_inputs: false,
            #[cfg(feature = "debugger")]
            run_target_id: None,
            #[cfg(feature = "debugger")]
            run_input_values: Vec::new(),
            #[cfg(feature = "debugger")]
            run_input_names: Vec::new(),
            #[cfg(feature = "debugger")]
            run_input_types: Vec::new(),
            #[cfg(feature = "debugger")]
            pending_action: None,
            show_coordinator_picker: false,
            discovered_services: Vec::new(),
            discovering: false,
            selected_debug_address: None,
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

    #[test]
    fn save_error_shows_modal() {
        let mut gui = test_gui();
        assert!(!gui.show_modal);
        drop(gui.update(Message::SaveError("write failed".into())));
        assert!(gui.show_modal);
        assert_eq!(gui.modal_content.0, "Error");
        assert_eq!(gui.modal_content.1, "write failed");
    }

    #[test]
    fn submitted_sets_flow_name_from_url() {
        let mut gui = test_gui();
        gui.submission_settings.flow_manifest_url =
            "flowr/examples/mandlebrot/manifest.json".into();
        drop(gui.update(Message::Submitted));
        assert_eq!(gui.tab_set.flow_name, "mandlebrot");
        assert!(gui.submitted);
    }

    #[test]
    fn submitted_sets_empty_flow_name_when_no_parent() {
        let mut gui = test_gui();
        gui.submission_settings.flow_manifest_url = "manifest.json".into();
        drop(gui.update(Message::Submitted));
        assert!(gui.tab_set.flow_name.is_empty());
    }

    // ---- iced_test view rendering tests ----

    #[test]
    fn main_view_renders_without_panic() {
        use iced_test::simulator::simulator;
        let gui = test_gui();
        let view = gui.view();
        let _ui = simulator(view);
    }

    #[cfg(feature = "debugger")]
    #[test]
    fn debug_view_renders_when_active() {
        use iced_test::simulator::simulator;
        let mut gui = test_gui();
        gui.debug_client_active = true;
        gui.debug_waiting = true;
        let view = gui.view();
        let _ui = simulator(view);
    }

    #[cfg(feature = "debugger")]
    #[test]
    fn debug_view_renders_when_not_waiting() {
        use iced_test::simulator::simulator;
        let mut gui = test_gui();
        gui.debug_client_active = true;
        gui.debug_waiting = false;
        let view = gui.view();
        let _ui = simulator(view);
    }

    #[cfg(feature = "debugger")]
    #[test]
    fn debug_reset_clears_waiting() {
        let mut gui = test_gui();
        gui.debug_client_active = true;
        gui.debug_waiting = true;
        drop(gui.update(Message::DebugReset));
        assert!(!gui.debug_waiting);
    }

    #[test]
    fn view_renders_after_submitted() {
        use iced_test::simulator::simulator;
        let mut gui = test_gui();
        gui.submission_settings.flow_manifest_url = "flowr/examples/fibonacci/manifest.json".into();
        drop(gui.update(Message::Submitted));
        let view = gui.view();
        let _ui = simulator(view);
    }

    #[test]
    fn view_renders_with_error_modal() {
        use iced_test::simulator::simulator;
        let mut gui = test_gui();
        drop(gui.update(Message::SubmitError("test error".into())));
        assert!(gui.show_modal);
        let view = gui.view();
        let _ui = simulator(view);
    }

    #[test]
    fn view_renders_after_tab_switch() {
        use iced_test::simulator::simulator;
        let mut gui = test_gui();
        drop(gui.update(Message::TabSelected(1)));
        let view = gui.view();
        let _ui = simulator(view);
    }

    #[cfg(feature = "debugger")]
    #[test]
    fn view_renders_with_breakpoint_popup() {
        use iced_test::simulator::simulator;
        let mut gui = test_gui();
        gui.debug_client_active = true;
        gui.debug_waiting = true;
        gui.show_bp_popup = true;
        let view = gui.view();
        let _ui = simulator(view);
    }

    #[cfg(feature = "debugger")]
    #[test]
    fn view_renders_with_inspect_popup() {
        use iced_test::simulator::simulator;
        let mut gui = test_gui();
        gui.debug_client_active = true;
        gui.debug_waiting = true;
        gui.show_inspect_popup = true;
        let view = gui.view();
        let _ui = simulator(view);
    }

    #[test]
    fn view_renders_with_coordinator_picker() {
        use iced_test::simulator::simulator;
        let mut gui = test_gui();
        gui.show_coordinator_picker = true;
        let view = gui.view();
        let _ui = simulator(view);
    }
}
