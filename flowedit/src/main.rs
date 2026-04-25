//! `flowedit` is a visual editor for flow definition files.
//!
//! Phase 1 provides a read-only viewer that renders the process nodes and connections
//! from a flow definition file onto an iced [`Canvas`][iced::widget::canvas::Canvas].
//!
//! Usage:
//! ```text
//! flowedit [flow-definition-file]
//! ```
//!
//! The flow file (TOML, YAML, or JSON) is parsed using flowcore's deserializer
//! and each [`ProcessReference`][flowcore::model::process_reference::ProcessReference]
//! is displayed as a colored, rounded rectangle on the canvas, with connections
//! drawn as bezier curves between nodes.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use clap::{Arg, ArgAction, Command as ClapCommand};
use iced::keyboard;
use iced::widget::{button, container, text_input, Column, Row, Text};
use iced::window;
use iced::{Color, Element, Fill, Subscription, Task, Theme};
use log::{info, warn};
use simpath::Simpath;
use url::Url;

use flowcore::deserializers::deserializer::get;
use flowcore::meta_provider::MetaProvider;
use flowcore::model::datatype::DataType;
use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::function_definition::FunctionDefinition;
use flowcore::model::io::IO;
use flowcore::model::lib_manifest::LibraryManifest;
use flowcore::model::name::HasName;
use flowcore::model::process::Process;
use flowcore::model::process_reference::ProcessReference;
use flowcore::model::route::Route;
use flowcore::provider::Provider;

use flow_canvas::{CanvasAction, CanvasMessage};
use hierarchy_panel::{FlowHierarchy, HierarchyMessage};
use history::EditHistory;
use library_panel::{LibraryAction, LibraryMessage, LibraryTree};
use window_state::FlowCanvasState;

mod file_ops;
mod flow_canvas;
mod hierarchy_panel;
mod history;
mod initializer;
mod library_mgmt;
mod library_panel;
mod node_layout;
mod utils;
mod window_state;

#[cfg(test)]
pub(crate) use window_state::InitializerEditor;
pub(crate) use window_state::{FunctionViewer, WindowKind, WindowState};

#[cfg(test)]
mod ui_test;

/// Messages for flow metadata and I/O editing, tagged by window
#[derive(Debug, Clone)]
pub(crate) enum FlowEditMessage {
    /// Flow name changed
    NameChanged(String),
    /// Flow version changed
    VersionChanged(String),
    /// Flow description changed
    DescriptionChanged(String),
    /// Flow authors changed
    AuthorsChanged(String),
    /// Toggle metadata editor visibility
    ToggleMetadata,
    /// Add a flow-level input port
    AddInput,
    /// Add a flow-level output port
    AddOutput,
    /// Delete a flow-level input port
    DeleteInput(usize),
    /// Delete a flow-level output port
    DeleteOutput(usize),
    /// Flow input port name changed
    InputNameChanged(usize, String),
    /// Flow output port name changed
    OutputNameChanged(usize, String),
}

/// Messages for function definition viewing/editing, tagged by window
#[derive(Debug, Clone)]
enum FunctionEditMessage {
    /// Switch tab in a function viewer window
    TabSelected(usize),
    /// Function name edited
    NameChanged(String),
    /// Function description edited
    DescriptionChanged(String),
    /// Browse for source file
    BrowseSource,
    /// Add a new input port to a function
    AddInput,
    /// Add a new output port to a function
    AddOutput,
    /// Delete an input port from a function
    DeleteInput(usize),
    /// Delete an output port from a function
    DeleteOutput(usize),
    /// Input port name changed
    InputNameChanged(usize, String),
    /// Input port type changed
    InputTypeChanged(usize, String),
    /// Output port name changed
    OutputNameChanged(usize, String),
    /// Output port type changed
    OutputTypeChanged(usize, String),
    /// Save function definition to disk
    Save,
}

/// View control messages for zoom and auto-fit
#[derive(Debug, Clone)]
enum ViewMessage {
    ZoomIn,
    ZoomOut,
    ToggleAutoFit,
}

/// Messages handled by the flowedit application
#[derive(Debug, Clone)]
enum Message {
    /// A message from the interactive canvas, tagged with its window ID
    WindowCanvas(window::Id, CanvasMessage),
    /// A message from the library side panel, tagged with the originating window ID
    Library(window::Id, LibraryMessage),
    /// A message from the flow hierarchy panel, tagged with window ID
    Hierarchy(window::Id, HierarchyMessage),
    /// A view control message (zoom, auto-fit), tagged with window ID
    View(window::Id, ViewMessage),
    /// Undo the last edit action
    Undo,
    /// Redo the last undone action
    Redo,
    /// Save the flow to the current file (or prompt if none)
    Save,
    /// Save the flow to a new file (always prompts)
    SaveAs,
    /// Open a flow file
    Open,
    /// Create a new empty flow
    New,
    /// Compile the current flow
    Compile,
    /// Initializer type changed in the editor dialog, tagged with the originating window ID
    InitializerTypeChanged(window::Id, String),
    /// Initializer value changed in the editor dialog, tagged with the originating window ID
    InitializerValueChanged(window::Id, String),
    /// Apply the initializer edit, tagged with the originating window ID
    InitializerApply(window::Id),
    /// Cancel the initializer edit, tagged with the originating window ID
    InitializerCancel(window::Id),
    /// Flow metadata and I/O editing messages (window ID, target route, message)
    FlowEdit(window::Id, Route, FlowEditMessage),
    /// Function definition viewing/editing messages
    FunctionEdit(window::Id, FunctionEditMessage),
    /// Create a new sub-flow and add it to the current flow (window that initiated the action)
    NewSubFlow(window::Id),
    /// Create a new provided implementation and add it to the current flow (window that initiated)
    NewFunction(window::Id),
    /// A window close was requested
    CloseRequested(window::Id),
    /// A window was actually closed (cleanup stale state)
    WindowClosed(window::Id),
    /// Close the currently focused window (Cmd+W)
    CloseActiveWindow,
    /// Quit the entire application (Cmd+Q)
    QuitAll,
    /// A window received focus
    WindowFocused(window::Id),
    /// Window was resized — track the new size
    WindowResized(window::Id, iced::Size),
    /// Window was moved — track the new position
    WindowMoved(window::Id, iced::Point),
    /// Add a library search path via file dialog
    AddLibraryPath,
    /// Remove a library search path by index
    RemoveLibraryPath(usize),
    /// Toggle the library paths editor
    ToggleLibPaths,
}

/// Top-level application state
struct FlowEdit {
    /// Per-window states, keyed by window ID
    windows: HashMap<window::Id, WindowState>,
    /// The single source of truth for the flow hierarchy
    root_flow: FlowDefinition,
    /// The ID of the root (main) window, if known
    root_window: Option<window::Id>,
    /// The ID of the currently focused window (updated on focus events)
    focused_window: Option<window::Id>,
    /// Library panel tree for process discovery
    library_tree: LibraryTree,
    show_lib_paths: bool,
    lib_paths: Vec<String>,
    /// Cached library manifests, keyed by library root URL (e.g., `lib://flowstdlib`)
    library_cache: HashMap<Url, LibraryManifest>,
    /// Cached parsed definitions for all library and context functions/flows
    all_definitions: HashMap<Url, Process>,
}

impl Default for FlowEdit {
    fn default() -> Self {
        Self {
            windows: HashMap::new(),
            root_flow: FlowDefinition::default(),
            root_window: None,
            focused_window: None,
            library_tree: LibraryTree { libraries: vec![] },
            show_lib_paths: false,
            lib_paths: Vec::new(),
            library_cache: HashMap::new(),
            all_definitions: HashMap::new(),
        }
    }
}

/// Main entry point for the flowedit binary.
///
/// Parses CLI arguments, loads the flow definition, and launches the iced GUI.
fn main() -> iced::Result {
    env_logger::init();
    iced::daemon(FlowEdit::new, FlowEdit::update, FlowEdit::view)
        .title(FlowEdit::title)
        .subscription(FlowEdit::subscription)
        .antialiasing(true)
        .run()
}

fn parse_cli_args() -> (Vec<String>, Option<String>) {
    let matches = ClapCommand::new("flowedit")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Visual editor for flow definition files")
        .arg(
            Arg::new("flow-file")
                .required(false)
                .help("Path to the flow definition file (.toml, .yaml, or .json)"),
        )
        .arg(
            Arg::new("lib_dir")
                .short('L')
                .long("libdir")
                .num_args(1)
                .action(ArgAction::Append)
                .value_name("LIB_DIR")
                .help("Add a directory to the Library Search path"),
        )
        .get_matches();

    let lib_dirs: Vec<String> = if matches.contains_id("lib_dir") {
        matches
            .get_many::<String>("lib_dir")
            .map(|dirs| dirs.map(std::string::ToString::to_string).collect())
            .unwrap_or_default()
    } else {
        vec![]
    };

    let flow_file = matches.get_one::<String>("flow-file").cloned();
    (lib_dirs, flow_file)
}

fn setup_lib_search_path(lib_dirs: &[String]) {
    let mut lib_search_path = Simpath::new_with_separator("FLOW_LIB_PATH", ',');
    for addition in lib_dirs {
        lib_search_path.add(addition);
        info!("'{addition}' added to the Library Search Path");
    }
    if lib_search_path.is_empty() {
        if let Ok(home) = std::env::var("HOME") {
            let default_lib = format!("{home}/.flow/lib");
            lib_search_path.add(&default_lib);
            std::env::set_var("FLOW_LIB_PATH", &default_lib);
        }
    } else if !lib_dirs.is_empty() {
        let current = std::env::var("FLOW_LIB_PATH").unwrap_or_default();
        let additions = lib_dirs.join(",");
        if current.is_empty() {
            std::env::set_var("FLOW_LIB_PATH", additions);
        } else {
            std::env::set_var("FLOW_LIB_PATH", format!("{current},{additions}"));
        }
    }
}

fn load_initial_flow(flow_file: Option<&str>) -> (String, FlowDefinition, BTreeSet<Url>) {
    if let Some(flow_path_str) = flow_file {
        let flow_path = PathBuf::from(flow_path_str);
        match file_ops::load_flow(&flow_path) {
            Ok(mut fd) => {
                let nc = fd.process_refs.len();
                let ec = fd.connections.len();
                let lib_refs = fd.lib_references.clone();
                if let Ok(url) = Url::from_file_path(&flow_path) {
                    fd.source_url = url;
                }
                (
                    format!("Ready - {nc} nodes, {ec} connections"),
                    fd,
                    lib_refs,
                )
            }
            Err(e) => {
                let fd = FlowDefinition {
                    name: String::from("(error)"),
                    ..FlowDefinition::default()
                };
                (format!("Error loading flow: {e}"), fd, BTreeSet::new())
            }
        }
    } else {
        let fd = FlowDefinition {
            name: String::from("(new flow)"),
            ..FlowDefinition::default()
        };
        (String::from("Ready"), fd, BTreeSet::new())
    }
}

/// Resolve a mutable reference to the [`FlowDefinition`] targeted by `route`.
///
/// When the route is empty or matches the root flow, returns `root_flow` directly.
/// Otherwise it walks the process hierarchy to find the matching sub-flow.
/// This is a free function (not a method) so that Rust can see that only
/// `root_flow` is borrowed mutably, leaving `windows` available for separate
/// mutable access.
fn resolve_flow_def_mut<'a>(
    root_flow: &'a mut FlowDefinition,
    route: &Route,
) -> &'a mut FlowDefinition {
    if route.is_empty() || *route == root_flow.route {
        return root_flow;
    }
    // Check immutably first to decide whether to descend, then borrow mutably.
    // This avoids moving `root_flow` after a mutable borrow in the `None` arm.
    let has_sub_flow = matches!(
        root_flow.process_from_route(route),
        Some(Process::FlowProcess(_))
    );
    if has_sub_flow {
        match root_flow.process_from_route_mut(route) {
            Some(Process::FlowProcess(f)) => f,
            _ => unreachable!(),
        }
    } else {
        root_flow
    }
}

impl FlowEdit {
    /// Create the application, parsing CLI args and optionally loading a flow file.
    fn new() -> (Self, Task<Message>) {
        let (lib_dirs, flow_file) = parse_cli_args();
        setup_lib_search_path(&lib_dirs);

        let (status, flow_definition, lib_refs) = load_initial_flow(flow_file.as_deref());

        let has_nodes = !flow_definition.process_refs.is_empty();

        let (library_cache, all_definitions) = library_mgmt::load_library_catalogs(&lib_refs);
        let library_tree = LibraryTree::from_cache(&library_cache, &all_definitions);

        let file_path = flow_definition.source_url.to_file_path().ok();
        let saved_prefs = file_path
            .as_ref()
            .and_then(|p| file_ops::load_editor_prefs(p));
        let saved_size = saved_prefs.as_ref().map_or_else(
            || iced::Size::new(1024.0, 768.0),
            |p| iced::Size::new(p.width, p.height),
        );
        let saved_position = saved_prefs
            .as_ref()
            .and_then(|p| match (p.x, p.y) {
                (Some(x), Some(y)) => Some(window::Position::Specific(iced::Point::new(x, y))),
                _ => None,
            })
            .unwrap_or(window::Position::Default);
        let (root_id, open_task) = window::open(window::Settings {
            size: saved_size,
            position: saved_position,
            ..Default::default()
        });

        let flow_hierarchy = FlowHierarchy::from_flow_definition(&flow_definition);
        let root_flow = flow_definition.clone();

        let win_state = WindowState {
            route: flow_definition.route.clone(),
            kind: WindowKind::FlowEditor,
            canvas_state: FlowCanvasState::default(),
            status,
            selected_node: None,
            selected_connection: None,
            auto_fit_pending: has_nodes,
            auto_fit_enabled: true,
            history: EditHistory::default(),
            tooltip: None,
            initializer_editor: None,
            is_root: true,
            context_menu: None,
            show_metadata: false,
            flow_hierarchy,
            last_size: None,
            last_position: None,
        };

        let mut windows = HashMap::new();
        windows.insert(root_id, win_state);

        let lib_paths = file_ops::resolve_lib_paths();
        let app = FlowEdit {
            windows,
            root_flow,
            root_window: Some(root_id),
            focused_window: Some(root_id),
            library_tree,
            show_lib_paths: false,
            lib_paths,
            library_cache,
            all_definitions,
        };

        (app, open_task.discard())
    }

    /// Return the window title, showing the flow name, file name, and unsaved indicator.
    fn title(&self, window_id: window::Id) -> String {
        if let Some(win) = self.windows.get(&window_id) {
            let modified = if win.history.is_empty() { "" } else { " *" };
            let file = WindowState::file_path_of(&self.root_flow)
                .as_ref()
                .and_then(|p| p.file_name().map(ToOwned::to_owned))
                .and_then(|n| n.to_str().map(String::from))
                .unwrap_or_else(|| String::from("untitled"));
            format!("flowedit - {} ({}){modified}", self.root_flow.name, file)
        } else {
            String::from("flowedit")
        }
    }

    /// Check whether the window identified by `win` is editing the file at `path`.
    ///
    /// Uses the window's route to look up the corresponding process in the flow
    /// hierarchy and compares its source URL against the given path.
    fn window_has_path(&self, win: &WindowState, path: &Path) -> bool {
        match &win.kind {
            WindowKind::FunctionViewer(viewer) => {
                viewer
                    .func_def
                    .get_source_url()
                    .to_file_path()
                    .ok()
                    .as_deref()
                    == Some(path)
            }
            WindowKind::FlowEditor => {
                self.root_flow
                    .process_from_route(&win.route)
                    .is_some_and(|p| match p {
                        Process::FlowProcess(f) => {
                            f.source_url.to_file_path().ok().as_deref() == Some(path)
                        }
                        Process::FunctionProcess(f) => {
                            f.get_source_url().to_file_path().ok().as_deref() == Some(path)
                        }
                    })
            }
        }
    }

    fn handle_hierarchy_message(
        &mut self,
        hier_win_id: window::Id,
        hier_msg: &HierarchyMessage,
    ) -> Task<Message> {
        let open_result = self
            .windows
            .get_mut(&hier_win_id)
            .and_then(|win| win.flow_hierarchy.update(hier_msg));
        let Some(route) = open_result else {
            return Task::none();
        };
        // Check if already open
        for (&win_id, win) in &self.windows {
            if win.route == route {
                return window::gain_focus(win_id);
            }
        }
        // Assign default positions to sub-flow nodes that don't have saved x/y
        if let Some(Process::FlowProcess(sub_flow)) = self.root_flow.process_from_route_mut(&route)
        {
            file_ops::assign_default_positions(sub_flow);
        }

        let process_info = match self.root_flow.process_from_route(&route) {
            Some(Process::FlowProcess(f)) => Some((
                true,
                !f.process_refs.is_empty(),
                f.process_refs.len(),
                f.connections.len(),
                None,
            )),
            Some(Process::FunctionProcess(f)) => Some((
                false,
                false,
                0,
                0,
                Some((
                    f.clone(),
                    f.get_source_url().to_file_path().unwrap_or_default(),
                )),
            )),
            None => None,
        };
        let Some((is_flow, has_nodes, nc, ec, func_info)) = process_info else {
            return Task::none();
        };
        if is_flow {
            let (new_id, open_task) = window::open(self.child_window_settings(1024.0, 768.0));
            let child = WindowState {
                route,
                kind: WindowKind::FlowEditor,
                canvas_state: FlowCanvasState::default(),
                status: format!("Ready - {nc} nodes, {ec} connections"),
                selected_node: None,
                selected_connection: None,
                history: EditHistory::default(),
                auto_fit_pending: has_nodes,
                auto_fit_enabled: true,
                flow_hierarchy: FlowHierarchy::from_flow_definition(&self.root_flow),
                tooltip: None,
                initializer_editor: None,
                is_root: false,
                context_menu: None,
                show_metadata: false,
                last_size: None,
                last_position: None,
            };
            self.windows.insert(new_id, child);
            open_task.discard()
        } else if let Some((func_def, source_path)) = func_info {
            self.open_function_viewer(
                hier_win_id,
                &source_path,
                &func_def,
                route.as_ref(),
                route.clone(),
            )
        } else {
            Task::none()
        }
    }

    fn handle_close_requested(&mut self, target: Option<window::Id>) -> Task<Message> {
        let Some(id) = target else {
            return Task::none();
        };
        if let Some(win) = self.windows.get(&id) {
            if !win.history.is_empty() {
                let dialog = rfd::MessageDialog::new()
                    .set_title("Unsaved Changes")
                    .set_description("This window has unsaved changes. Close without saving?")
                    .set_buttons(rfd::MessageButtons::YesNo)
                    .set_level(rfd::MessageLevel::Warning);
                if dialog.show() != rfd::MessageDialogResult::Yes {
                    return Task::none();
                }
            }
        }
        self.windows.remove(&id);
        if self.root_window == Some(id) || self.windows.is_empty() {
            return iced::exit();
        }
        window::close(id)
    }

    fn handle_quit_all(&self) -> Task<Message> {
        let has_unsaved = self.windows.values().any(|w| !w.history.is_empty());
        if has_unsaved {
            let dialog = rfd::MessageDialog::new()
                .set_title("Unsaved Changes")
                .set_description("There are unsaved changes. Quit without saving?")
                .set_buttons(rfd::MessageButtons::YesNo)
                .set_level(rfd::MessageLevel::Warning);
            if dialog.show() != rfd::MessageDialogResult::Yes {
                return Task::none();
            }
        }
        iced::exit()
    }

    fn handle_add_library(&mut self, win_id: window::Id) {
        let dialog = rfd::FileDialog::new();
        let Some(dir) = dialog.pick_folder() else {
            return;
        };
        let lib_name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let Ok(lib_url) = Url::parse(&format!("lib://{lib_name}")) else {
            return;
        };
        let Some(parent) = dir.parent() else {
            return;
        };
        let Some(parent_str) = parent.to_str() else {
            return;
        };

        let mut lib_search_path = Simpath::new_with_separator("FLOW_LIB_PATH", ',');
        lib_search_path.add_directory(parent_str);

        if let Ok(home) = std::env::var("HOME") {
            let default_lib = PathBuf::from(&home).join(".flow").join("lib");
            if default_lib.exists() {
                if let Some(path_str) = default_lib.to_str() {
                    lib_search_path.add_directory(path_str);
                }
            }
        }

        let context_root = std::env::var("HOME").map_or_else(
            |_| PathBuf::from("/"),
            |h| {
                PathBuf::from(h)
                    .join(".flow")
                    .join("runner")
                    .join("flowrcli")
            },
        );
        let provider = MetaProvider::new(lib_search_path.clone(), context_root.clone());
        let arc_provider: Arc<dyn Provider> = Arc::new(provider);

        match LibraryManifest::load(&arc_provider, &lib_url) {
            Ok((manifest, _manifest_url)) => {
                info!(
                    "Loaded library manifest for '{}' with {} locators",
                    lib_url,
                    manifest.locators.len()
                );

                for locator_url in manifest.locators.keys() {
                    let meta_provider =
                        MetaProvider::new(lib_search_path.clone(), context_root.clone());
                    match flowrclib::compiler::parser::parse(locator_url, &meta_provider) {
                        Ok(process) => {
                            self.all_definitions.insert(locator_url.clone(), process);
                        }
                        Err(e) => {
                            warn!("Could not parse library definition '{locator_url}': {e}");
                        }
                    }
                }

                self.library_cache.insert(lib_url.clone(), manifest);

                if !self.lib_paths.contains(&parent_str.to_string()) {
                    self.lib_paths.push(parent_str.to_string());
                    self.update_lib_paths();
                }

                self.library_tree =
                    LibraryTree::from_cache(&self.library_cache, &self.all_definitions);

                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.status = format!("Added library: {lib_name}");
                }
            }
            Err(e) => {
                warn!("Could not load library manifest for '{lib_url}': {e}");
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.status = format!("Failed to load library '{lib_name}': {e}");
                }
            }
        }
    }

    fn handle_library_message(
        &mut self,
        win_id: window::Id,
        lib_msg: &LibraryMessage,
    ) -> Task<Message> {
        match self.library_tree.update(lib_msg) {
            LibraryAction::Add(source, func_name) => {
                let route = self
                    .windows
                    .get(&win_id)
                    .map(|w| w.route.clone())
                    .unwrap_or_default();
                let flow_def = resolve_flow_def_mut(&mut self.root_flow, &route);
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.add_library_function(flow_def, &source, &func_name);
                }
            }
            LibraryAction::View(source, _name) => {
                return self.open_library_function(&source);
            }
            LibraryAction::AddLibrary => {
                self.handle_add_library(win_id);
            }
            LibraryAction::None => {}
        }
        Task::none()
    }

    fn handle_window_event(&mut self, message: &Message) -> Task<Message> {
        match *message {
            Message::WindowFocused(id) => {
                self.focused_window = Some(id);
            }
            Message::WindowResized(id, size) => {
                if let Some(win) = self.windows.get_mut(&id) {
                    win.last_size = Some(size);
                }
            }
            Message::WindowMoved(id, pos) => {
                if let Some(win) = self.windows.get_mut(&id) {
                    win.last_position = Some(pos);
                }
            }
            Message::WindowClosed(id) => {
                self.windows.remove(&id);
                if self.focused_window == Some(id) {
                    self.focused_window = self.root_window;
                }
                if self.root_window == Some(id) || self.windows.is_empty() {
                    return iced::exit();
                }
            }
            _ => {}
        }
        Task::none()
    }

    fn close_orphaned_windows(&mut self) -> Task<Message> {
        let orphans: Vec<window::Id> = self
            .windows
            .iter()
            .filter(|(_, win)| {
                if win.is_root || win.route.is_empty() {
                    return false;
                }
                if matches!(win.kind, WindowKind::FunctionViewer(_)) {
                    return false;
                }
                self.root_flow.process_from_route(&win.route).is_none()
            })
            .map(|(&id, _)| id)
            .collect();

        let mut tasks = Vec::new();
        for id in orphans {
            self.windows.remove(&id);
            tasks.push(window::close(id));
        }

        if tasks.is_empty() {
            Task::none()
        } else {
            Task::batch(tasks)
        }
    }

    fn handle_canvas_update(
        &mut self,
        win_id: window::Id,
        canvas_msg: CanvasMessage,
    ) -> Task<Message> {
        let route = self
            .windows
            .get(&win_id)
            .map(|w| w.route.clone())
            .unwrap_or_default();
        let flow_def = resolve_flow_def_mut(&mut self.root_flow, &route);
        let Some(win) = self.windows.get_mut(&win_id) else {
            return Task::none();
        };
        let action = win.handle_canvas_message(flow_def, canvas_msg);
        let close_task = self.close_orphaned_windows();
        let action_task = match action {
            CanvasAction::OpenNode(idx) => self.open_node(win_id, idx),
            CanvasAction::None => Task::none(),
        };
        Task::batch([close_task, action_task])
    }

    fn handle_initializer_message(&mut self, message: &Message) {
        match *message {
            Message::InitializerTypeChanged(win_id, ref new_type) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.handle_initializer_type_changed(new_type.clone());
                }
            }
            Message::InitializerValueChanged(win_id, ref new_value) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.handle_initializer_value_changed(new_value.clone());
                }
            }
            Message::InitializerApply(win_id) => {
                let route = self
                    .windows
                    .get(&win_id)
                    .map(|w| w.route.clone())
                    .unwrap_or_default();
                let flow_def = resolve_flow_def_mut(&mut self.root_flow, &route);
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.handle_initializer_apply(flow_def);
                }
            }
            Message::InitializerCancel(win_id) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.handle_initializer_cancel();
                }
            }
            _ => {}
        }
    }

    fn handle_undo_redo(&mut self, is_redo: bool) -> Task<Message> {
        let target = self.focused_window.or(self.root_window);
        if let Some(target_id) = target {
            let route = self
                .windows
                .get(&target_id)
                .map(|w| w.route.clone())
                .unwrap_or_default();
            let flow_def = resolve_flow_def_mut(&mut self.root_flow, &route);
            if let Some(win) = self.windows.get_mut(&target_id) {
                if is_redo {
                    win.handle_redo(flow_def);
                } else {
                    win.handle_undo(flow_def);
                }
            }
        }
        self.close_orphaned_windows()
    }

    fn handle_flow_edit(&mut self, win_id: window::Id, route: &Route, flow_msg: FlowEditMessage) {
        let affects_parent = matches!(
            flow_msg,
            FlowEditMessage::NameChanged(_)
                | FlowEditMessage::AddInput
                | FlowEditMessage::AddOutput
                | FlowEditMessage::DeleteInput(_)
                | FlowEditMessage::DeleteOutput(_)
                | FlowEditMessage::InputNameChanged(_, _)
                | FlowEditMessage::OutputNameChanged(_, _)
        );

        // Capture deleted I/O name before deletion for parent connection cleanup
        let io_delete = match &flow_msg {
            FlowEditMessage::DeleteInput(idx) => {
                let flow_def = resolve_flow_def_mut(&mut self.root_flow, route);
                flow_def
                    .inputs
                    .get(*idx)
                    .map(|io| (true, io.name().clone()))
            }
            FlowEditMessage::DeleteOutput(idx) => {
                let flow_def = resolve_flow_def_mut(&mut self.root_flow, route);
                flow_def
                    .outputs
                    .get(*idx)
                    .map(|io| (false, io.name().clone()))
            }
            _ => None,
        };

        // Capture old I/O name before rename for parent connection update
        let io_rename = match &flow_msg {
            FlowEditMessage::InputNameChanged(idx, new_name) => {
                let flow_def = resolve_flow_def_mut(&mut self.root_flow, route);
                flow_def
                    .inputs
                    .get(*idx)
                    .map(|io| (true, io.name().clone(), new_name.clone()))
            }
            FlowEditMessage::OutputNameChanged(idx, new_name) => {
                let flow_def = resolve_flow_def_mut(&mut self.root_flow, route);
                flow_def
                    .outputs
                    .get(*idx)
                    .map(|io| (false, io.name().clone(), new_name.clone()))
            }
            _ => None,
        };

        let flow_def = resolve_flow_def_mut(&mut self.root_flow, route);
        if let Some(win) = self.windows.get_mut(&win_id) {
            win.handle_flow_edit_message(flow_def, flow_msg);
        }

        if let Some((is_input, old_name, new_name)) = io_rename {
            self.update_parent_io_rename(route, is_input, &old_name, &new_name);
        }
        if let Some((is_input, deleted_name)) = io_delete {
            self.update_parent_io_delete(route, is_input, &deleted_name);
        }

        if affects_parent && !route.is_empty() && *route != self.root_flow.route {
            for win in self.windows.values_mut() {
                if win.route != *route && route.sub_route_of(&win.route).is_some() {
                    win.canvas_state.request_redraw();
                    win.trigger_auto_fit_if_enabled();
                }
            }
        }
    }

    fn update_parent_io_rename(
        &mut self,
        route: &Route,
        is_input: bool,
        old_name: &str,
        new_name: &str,
    ) {
        if route.is_empty() || *route == self.root_flow.route || old_name == new_name {
            return;
        }
        let sub_alias = route.as_ref().rsplit('/').next().unwrap_or("");
        if sub_alias.is_empty() {
            return;
        }
        let parent_route_str = &route.as_ref()[..route.as_ref().rfind('/').unwrap_or(0)];
        let parent_route = Route::from(parent_route_str);
        let parent_flow = resolve_flow_def_mut(&mut self.root_flow, &parent_route);
        let old_endpoint = format!("{sub_alias}/{old_name}");
        let new_endpoint = format!("{sub_alias}/{new_name}");
        for conn in &mut parent_flow.connections {
            if is_input {
                let new_to: Vec<Route> = conn
                    .to()
                    .iter()
                    .map(|r| {
                        if r.to_string() == old_endpoint {
                            Route::from(new_endpoint.as_str())
                        } else {
                            r.clone()
                        }
                    })
                    .collect();
                conn.set_to(new_to);
            } else if conn.from().to_string() == old_endpoint {
                conn.set_from(Route::from(new_endpoint.as_str()));
            }
        }
    }

    fn update_parent_io_delete(&mut self, route: &Route, is_input: bool, deleted_name: &str) {
        if route.is_empty() || *route == self.root_flow.route {
            return;
        }
        let sub_alias = route.as_ref().rsplit('/').next().unwrap_or("");
        if sub_alias.is_empty() {
            return;
        }
        let parent_route_str = &route.as_ref()[..route.as_ref().rfind('/').unwrap_or(0)];
        let parent_route = Route::from(parent_route_str);
        let parent_flow = resolve_flow_def_mut(&mut self.root_flow, &parent_route);
        let endpoint = format!("{sub_alias}/{deleted_name}");
        if is_input {
            for conn in &mut parent_flow.connections {
                let new_to: Vec<Route> = conn
                    .to()
                    .iter()
                    .filter(|r| r.to_string() != endpoint)
                    .cloned()
                    .collect();
                conn.set_to(new_to);
            }
            parent_flow.connections.retain(|c| !c.to().is_empty());
        } else {
            parent_flow
                .connections
                .retain(|c| c.from().to_string() != endpoint);
        }
    }

    /// Handle messages from canvas interactions.
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WindowCanvas(win_id, canvas_msg) => {
                return self.handle_canvas_update(win_id, canvas_msg);
            }
            Message::Hierarchy(hier_win_id, ref hier_msg) => {
                return self.handle_hierarchy_message(hier_win_id, hier_msg);
            }
            Message::Library(win_id, ref lib_msg) => {
                return self.handle_library_message(win_id, lib_msg);
            }
            Message::View(win_id, view_msg) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    win.handle_view_message(&view_msg);
                }
            }
            Message::Undo => return self.handle_undo_redo(false),
            Message::Redo => return self.handle_undo_redo(true),
            Message::Save | Message::SaveAs | Message::Open | Message::New | Message::Compile => {
                self.handle_file_message(message);
            }
            Message::FlowEdit(win_id, ref route, flow_msg) => {
                self.handle_flow_edit(win_id, route, flow_msg);
            }
            Message::NewSubFlow(target_win_id) => {
                for win in self.windows.values_mut() {
                    win.context_menu = None;
                }
                return self.create_new_subflow(target_win_id);
            }
            Message::NewFunction(target_win_id) => {
                for win in self.windows.values_mut() {
                    win.context_menu = None;
                }
                return self.create_new_function(target_win_id);
            }
            Message::InitializerTypeChanged(_, _)
            | Message::InitializerValueChanged(_, _)
            | Message::InitializerApply(_)
            | Message::InitializerCancel(_) => {
                self.handle_initializer_message(&message);
            }
            Message::AddLibraryPath => {
                let dialog = rfd::FileDialog::new();
                if let Some(dir) = dialog.pick_folder() {
                    let path_str = dir.to_string_lossy().to_string();
                    if !self.lib_paths.contains(&path_str) {
                        self.lib_paths.push(path_str);
                        self.update_lib_paths();
                    }
                }
            }
            Message::RemoveLibraryPath(idx) => {
                if idx < self.lib_paths.len() {
                    self.lib_paths.remove(idx);
                    self.update_lib_paths();
                }
            }
            Message::ToggleLibPaths => {
                self.show_lib_paths = !self.show_lib_paths;
            }
            Message::FunctionEdit(win_id, func_msg) => {
                self.handle_function_edit_message(win_id, func_msg);
            }
            Message::WindowFocused(_)
            | Message::WindowResized(_, _)
            | Message::WindowMoved(_, _)
            | Message::WindowClosed(_) => {
                return self.handle_window_event(&message);
            }
            Message::CloseRequested(_) | Message::CloseActiveWindow => {
                let target = match message {
                    Message::CloseRequested(win_id) => Some(win_id),
                    Message::CloseActiveWindow => self.focused_window.or(self.root_window),
                    _ => None,
                };
                return self.handle_close_requested(target);
            }
            Message::QuitAll => {
                return self.handle_quit_all();
            }
        }
        Task::none()
    }

    /// Build the view for a window, dispatching based on window kind.
    fn view(&self, window_id: window::Id) -> Element<'_, Message> {
        let Some(win) = self.windows.get(&window_id) else {
            return Text::new("Window not found").into();
        };

        if let WindowKind::FunctionViewer(ref viewer) = win.kind {
            return Self::view_function(window_id, viewer, &win.status, !win.history.is_empty());
        }

        // Resolve the FlowDefinition for this window: sub-flow windows use
        // the sub-flow found via their route, root windows use root_flow directly.
        let flow_def = if win.is_root || win.route.is_empty() || win.route == self.root_flow.route {
            &self.root_flow
        } else {
            match self.root_flow.process_from_route(&win.route) {
                Some(Process::FlowProcess(f)) => f,
                _ => &self.root_flow,
            }
        };

        let canvas_with_controls = win.view_canvas_area(flow_def, window_id);

        let hierarchy_panel = win
            .flow_hierarchy
            .view(flow_def)
            .map(move |msg| Message::Hierarchy(window_id, msg));

        let library_panel = self
            .library_tree
            .view(&self.all_definitions)
            .map(move |msg| Message::Library(window_id, msg));

        let left_panel = Column::new()
            .push(hierarchy_panel)
            .push(library_panel)
            .height(Fill);

        let mut right_col: Column<'_, Message> =
            Column::new().push(container(canvas_with_controls).width(Fill).height(Fill));

        // Metadata editor panel (toggled by Info button)
        if win.show_metadata && matches!(win.kind, WindowKind::FlowEditor) {
            right_col = right_col.push(Self::view_metadata_panel(flow_def, window_id));
        }

        // Library paths panel (toggled by Libs button)
        if self.show_lib_paths {
            right_col = right_col.push(self.view_lib_paths_panel());
        }

        right_col = right_col.push(self.view_toolbar(win, flow_def, window_id));

        let layout = Row::new().push(left_panel).push(right_col.width(Fill));
        layout.into()
    }

    /// Build the toolbar/status bar with action buttons and status text.
    fn view_toolbar<'a>(
        &'a self,
        win: &'a WindowState,
        flow_def: &'a FlowDefinition,
        window_id: window::Id,
    ) -> Element<'a, Message> {
        let edit_indicator = if win.history.is_empty() {
            String::from("  |  saved")
        } else {
            String::from("  |  unsaved edit(s)")
        };

        let btn_pad = [6, 14];
        let btn_size = 13;
        let toolbar_btn = |_theme: &Theme, status: button::Status| -> button::Style {
            let is_hovered = matches!(status, button::Status::Hovered);
            button::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.25, 0.25, 0.30))),
                text_color: Color::WHITE,
                border: iced::Border {
                    color: if is_hovered {
                        Color::WHITE
                    } else {
                        Color::from_rgb(0.4, 0.4, 0.45)
                    },
                    width: 2.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            }
        };
        let toolbar_btn_active = |_theme: &Theme, status: button::Status| -> button::Style {
            let is_hovered = matches!(status, button::Status::Hovered);
            button::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.3, 0.35, 0.5))),
                text_color: Color::WHITE,
                border: iced::Border {
                    color: if is_hovered {
                        Color::WHITE
                    } else {
                        Color::from_rgb(0.4, 0.5, 0.7)
                    },
                    width: 2.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            }
        };

        let status_bar: Row<'_, Message> = if win.is_root {
            let mut compile_btn = button(Text::new("\u{1F528} Build").size(btn_size).center())
                .padding(btn_pad)
                .style(toolbar_btn);
            if !flow_def.process_refs.is_empty() {
                compile_btn = compile_btn.on_press(Message::Compile);
            }

            let new_subflow_btn = button(Text::new("+ Sub-flow").size(btn_size).center())
                .on_press(Message::NewSubFlow(window_id))
                .style(toolbar_btn)
                .padding(btn_pad);

            let new_func_btn = button(Text::new("+ Function").size(btn_size).center())
                .on_press(Message::NewFunction(window_id))
                .style(toolbar_btn)
                .padding(btn_pad);

            let info_btn = button(Text::new("\u{2139} Info").size(btn_size).center())
                .on_press(Message::FlowEdit(
                    window_id,
                    flow_def.route.clone(),
                    FlowEditMessage::ToggleMetadata,
                ))
                .style(if win.show_metadata {
                    toolbar_btn_active
                } else {
                    toolbar_btn
                })
                .padding(btn_pad);

            Row::new()
                .spacing(8)
                .padding([4, 8])
                .push(
                    container(Text::new(format!("{}{}", win.status, edit_indicator)).size(14))
                        .width(Fill)
                        .clip(true),
                )
                .push(info_btn)
                .push(
                    button(Text::new("\u{1F4C1} Libs").size(btn_size).center())
                        .on_press(Message::ToggleLibPaths)
                        .style(if self.show_lib_paths {
                            toolbar_btn_active
                        } else {
                            toolbar_btn
                        })
                        .padding(btn_pad),
                )
                .push(new_subflow_btn)
                .push(new_func_btn)
                .push(compile_btn)
        } else {
            Row::new().spacing(8).padding([4, 8]).push(
                container(Text::new(format!("{}{}", win.status, edit_indicator)).size(14))
                    .width(Fill)
                    .clip(true),
            )
        };

        container(status_bar).width(Fill).padding(5).into()
    }

    /// Build the metadata editor panel.
    fn view_metadata_panel(
        flow_def: &FlowDefinition,
        window_id: window::Id,
    ) -> Element<'_, Message> {
        let authors_str = flow_def.metadata.authors.join(", ");
        let meta_panel = container(
            Column::new()
                .spacing(6)
                .padding(12)
                .push(
                    Row::new()
                        .spacing(8)
                        .align_y(iced::Alignment::Center)
                        .push(Text::new("Name:").size(12).width(70))
                        .push(
                            text_input("Flow name", &flow_def.name)
                                .on_input(move |s| {
                                    Message::FlowEdit(
                                        window_id,
                                        flow_def.route.clone(),
                                        FlowEditMessage::NameChanged(s),
                                    )
                                })
                                .size(13)
                                .padding(4)
                                .width(250),
                        ),
                )
                .push(
                    Row::new()
                        .spacing(8)
                        .align_y(iced::Alignment::Center)
                        .push(Text::new("Version:").size(12).width(70))
                        .push(
                            text_input("0.1.0", &flow_def.metadata.version)
                                .on_input(move |s| {
                                    Message::FlowEdit(
                                        window_id,
                                        flow_def.route.clone(),
                                        FlowEditMessage::VersionChanged(s),
                                    )
                                })
                                .size(13)
                                .padding(4)
                                .width(120),
                        ),
                )
                .push(
                    Row::new()
                        .spacing(8)
                        .align_y(iced::Alignment::Center)
                        .push(Text::new("Description:").size(12).width(70))
                        .push(
                            text_input("A short description", &flow_def.metadata.description)
                                .on_input(move |s| {
                                    Message::FlowEdit(
                                        window_id,
                                        flow_def.route.clone(),
                                        FlowEditMessage::DescriptionChanged(s),
                                    )
                                })
                                .size(13)
                                .padding(4)
                                .width(Fill),
                        ),
                )
                .push(
                    Row::new()
                        .spacing(8)
                        .align_y(iced::Alignment::Center)
                        .push(Text::new("Authors:").size(12).width(70))
                        .push(
                            text_input("Name <email>, ...", &authors_str)
                                .on_input(move |s| {
                                    Message::FlowEdit(
                                        window_id,
                                        flow_def.route.clone(),
                                        FlowEditMessage::AuthorsChanged(s),
                                    )
                                })
                                .size(13)
                                .padding(4)
                                .width(Fill),
                        ),
                ),
        )
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.14, 0.14, 0.18))),
            border: iced::Border {
                color: Color::from_rgb(0.3, 0.3, 0.3),
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .width(Fill);

        meta_panel.into()
    }

    /// Build the library paths panel.
    fn view_lib_paths_panel(&self) -> Element<'_, Message> {
        let mut paths_col = Column::new().spacing(4).padding(12);
        paths_col = paths_col.push(Text::new("Library Search Paths").size(14));

        for (i, p) in self.lib_paths.iter().enumerate() {
            let row = Row::new()
                .spacing(6)
                .align_y(iced::Alignment::Center)
                .push(Text::new(p).size(12))
                .push(iced::widget::Space::new().width(Fill))
                .push(
                    button(Text::new("\u{2715}").size(10).center())
                        .on_press(Message::RemoveLibraryPath(i))
                        .style(button::danger)
                        .padding([2, 5]),
                );
            paths_col = paths_col.push(row);
        }
        paths_col = paths_col.push(
            button(Text::new("+ Add Path...").size(12).center())
                .on_press(Message::AddLibraryPath)
                .style(button::secondary)
                .padding([4, 10]),
        );

        let lib_panel = container(paths_col)
            .style(|_theme: &Theme| container::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.14, 0.14, 0.18))),
                border: iced::Border {
                    color: Color::from_rgb(0.3, 0.3, 0.3),
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .width(Fill);

        lib_panel.into()
    }

    fn view_function_definition_tab(
        window_id: window::Id,
        viewer: &FunctionViewer,
    ) -> Element<'_, Message> {
        let input_color = Color::from_rgb(0.4, 0.8, 1.0);
        let output_color = Color::from_rgb(1.0, 0.6, 0.3);
        let editable = !viewer.read_only;

        let input_col = Self::view_function_input_ports(window_id, viewer, input_color, editable);
        let output_col =
            Self::view_function_output_ports(window_id, viewer, output_color, editable);

        let mut name_widget = text_input("Function name", &viewer.func_def.name)
            .size(16)
            .padding(6)
            .width(250);
        if editable {
            name_widget = name_widget.on_input(move |s| {
                Message::FunctionEdit(window_id, FunctionEditMessage::NameChanged(s))
            });
        }
        let name_input = container(name_widget).center_x(Fill);

        let mut desc_widget = text_input("Description", &viewer.func_def.description)
            .size(13)
            .padding(6)
            .width(480);
        if editable {
            let ext = std::path::Path::new(&viewer.func_def.source)
                .extension()
                .unwrap_or_default();
            let is_provided = ext.eq_ignore_ascii_case("rs") || ext.eq_ignore_ascii_case("wasm");
            if is_provided {
                desc_widget = desc_widget.on_input(move |s| {
                    Message::FunctionEdit(window_id, FunctionEditMessage::DescriptionChanged(s))
                });
            }
        }
        let desc_input = container(desc_widget).center_x(Fill);

        let mut source_row = Row::new().spacing(6).align_y(iced::Alignment::Center).push(
            button(
                Text::new(&viewer.func_def.source)
                    .size(13)
                    .color(Color::from_rgb(0.6, 0.8, 1.0)),
            )
            .on_press(Message::FunctionEdit(
                window_id,
                FunctionEditMessage::TabSelected(1),
            ))
            .style(button::text)
            .padding(0),
        );
        if editable {
            source_row = source_row.push(
                button(Text::new("...").size(12).center())
                    .on_press(Message::FunctionEdit(
                        window_id,
                        FunctionEditMessage::BrowseSource,
                    ))
                    .style(button::secondary)
                    .padding([3, 8]),
            );
        }
        if viewer.docs_content.is_some() {
            source_row = source_row.push(
                button(Text::new("Docs").size(12).center())
                    .on_press(Message::FunctionEdit(
                        window_id,
                        FunctionEditMessage::TabSelected(2),
                    ))
                    .style(button::secondary)
                    .padding([3, 8]),
            );
        }

        let func_box = container(
            Column::new()
                .spacing(20)
                .padding(iced::Padding {
                    top: 24.0,
                    bottom: 24.0,
                    left: 0.0,
                    right: 0.0,
                })
                .push(name_input)
                .push(desc_input)
                .push(
                    Row::new()
                        .push(input_col)
                        .push(iced::widget::Space::new().width(Fill))
                        .push(output_col),
                )
                .push(container(source_row).center_x(Fill)),
        )
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.18, 0.18, 0.22))),
            border: iced::Border {
                color: Color::from_rgb(0.5, 0.3, 0.7),
                width: 2.0,
                radius: 12.0.into(),
            },
            ..Default::default()
        })
        .width(550);

        container(func_box).center(Fill).padding(40).into()
    }

    fn view_function_input_ports(
        window_id: window::Id,
        viewer: &FunctionViewer,
        input_color: Color,
        editable: bool,
    ) -> Column<'_, Message> {
        let mut input_col = Column::new().spacing(6);
        for (i, io) in viewer.func_def.inputs.iter().enumerate() {
            let port_name = io.name().clone();
            let dtype = io
                .datatypes()
                .first()
                .map(ToString::to_string)
                .unwrap_or_default();
            let mut name_widget = text_input("name", &port_name).size(13).padding(3).width(90);
            let mut type_widget = text_input("type", &dtype).size(11).padding(3).width(75);
            if editable {
                name_widget = name_widget.on_input(move |s| {
                    Message::FunctionEdit(window_id, FunctionEditMessage::InputNameChanged(i, s))
                });
                type_widget = type_widget.on_input(move |s| {
                    Message::FunctionEdit(window_id, FunctionEditMessage::InputTypeChanged(i, s))
                });
            }
            let mut row = Row::new()
                .spacing(4)
                .align_y(iced::Alignment::Center)
                .push(Text::new("\u{25D7}").size(24).color(input_color))
                .push(name_widget)
                .push(type_widget);
            if editable {
                row = row.push(
                    button(Text::new("\u{2715}").size(10).center())
                        .on_press(Message::FunctionEdit(
                            window_id,
                            FunctionEditMessage::DeleteInput(i),
                        ))
                        .style(button::danger)
                        .padding([2, 5]),
                );
            }
            input_col = input_col.push(row);
        }
        if editable {
            input_col = input_col.push(
                button(Text::new("+ Input").size(11).center())
                    .on_press(Message::FunctionEdit(
                        window_id,
                        FunctionEditMessage::AddInput,
                    ))
                    .style(button::secondary)
                    .padding([2, 8]),
            );
        }
        input_col
    }

    fn view_function_output_ports(
        window_id: window::Id,
        viewer: &FunctionViewer,
        output_color: Color,
        editable: bool,
    ) -> Column<'_, Message> {
        let mut output_col = Column::new().spacing(6).align_x(iced::Alignment::End);
        for (i, io) in viewer.func_def.outputs.iter().enumerate() {
            let port_name = io.name().clone();
            let dtype = io
                .datatypes()
                .first()
                .map(ToString::to_string)
                .unwrap_or_default();
            let mut type_widget = text_input("type", &dtype).size(11).padding(3).width(75);
            let mut name_widget = text_input("name", &port_name).size(13).padding(3).width(90);
            if editable {
                type_widget = type_widget.on_input(move |s| {
                    Message::FunctionEdit(window_id, FunctionEditMessage::OutputTypeChanged(i, s))
                });
                name_widget = name_widget.on_input(move |s| {
                    Message::FunctionEdit(window_id, FunctionEditMessage::OutputNameChanged(i, s))
                });
            }
            let mut row = Row::new().spacing(4).align_y(iced::Alignment::Center);
            if editable {
                row = row.push(
                    button(Text::new("\u{2715}").size(10).center())
                        .on_press(Message::FunctionEdit(
                            window_id,
                            FunctionEditMessage::DeleteOutput(i),
                        ))
                        .style(button::danger)
                        .padding([2, 5]),
                );
            }
            row = row
                .push(type_widget)
                .push(name_widget)
                .push(Text::new("\u{25D6}").size(24).color(output_color));
            output_col = output_col.push(row);
        }
        if editable {
            output_col = output_col.push(
                button(Text::new("+ Output").size(11).center())
                    .on_press(Message::FunctionEdit(
                        window_id,
                        FunctionEditMessage::AddOutput,
                    ))
                    .style(button::secondary)
                    .padding([2, 8]),
            );
        }
        output_col
    }

    fn view_function_source_tab(window_id: window::Id, rs_content: &str) -> Element<'_, Message> {
        let back_btn = button(Text::new("\u{2190} Definition").size(13).center())
            .on_press(Message::FunctionEdit(
                window_id,
                FunctionEditMessage::TabSelected(0),
            ))
            .style(button::secondary)
            .padding([6, 14]);
        Column::new()
            .push(container(back_btn).padding([8, 12]))
            .push(
                container(
                    iced::widget::scrollable(
                        Text::new(rs_content).size(14).font(iced::Font::MONOSPACE),
                    )
                    .width(Fill)
                    .height(Fill),
                )
                .width(Fill)
                .height(Fill)
                .padding(12),
            )
            .into()
    }

    fn view_function_docs_tab(
        window_id: window::Id,
        docs_content: Option<&str>,
    ) -> Element<'_, Message> {
        let back_btn = button(Text::new("\u{2190} Definition").size(13).center())
            .on_press(Message::FunctionEdit(
                window_id,
                FunctionEditMessage::TabSelected(0),
            ))
            .style(button::secondary)
            .padding([6, 14]);
        let docs = docs_content.unwrap_or("");
        Column::new()
            .push(container(back_btn).padding([8, 12]))
            .push(
                container(
                    iced::widget::scrollable(Text::new(docs).size(14))
                        .width(Fill)
                        .height(Fill),
                )
                .width(Fill)
                .height(Fill)
                .padding(12),
            )
            .into()
    }

    fn view_function<'a>(
        window_id: window::Id,
        viewer: &'a FunctionViewer,
        status: &'a str,
        has_unsaved: bool,
    ) -> Element<'a, Message> {
        let content: Element<'_, Message> = match viewer.active_tab {
            0 => Self::view_function_definition_tab(window_id, viewer),
            1 => Self::view_function_source_tab(window_id, &viewer.rs_content),
            _ => Self::view_function_docs_tab(window_id, viewer.docs_content.as_deref()),
        };

        let mut save_btn = button(Text::new("\u{1F4BE} Save").size(14).center())
            .style(if has_unsaved && !viewer.read_only {
                button::primary
            } else {
                button::secondary
            })
            .padding([6, 14]);
        if has_unsaved && !viewer.read_only {
            save_btn =
                save_btn.on_press(Message::FunctionEdit(window_id, FunctionEditMessage::Save));
        }

        let status_bar = Row::new()
            .spacing(8)
            .push(Text::new(status).size(14))
            .push(iced::widget::Space::new().width(Fill))
            .push(save_btn);

        Column::new()
            .push(container(content).width(Fill).height(Fill))
            .push(container(status_bar).width(Fill).padding(5))
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let _ = self; // required by iced daemon API signature
        let keyboard_sub = keyboard::listen().filter_map(|event| match event {
            keyboard::Event::KeyPressed {
                key: keyboard::Key::Character(ref c),
                modifiers,
                ..
            } if modifiers.command() => match c.as_str() {
                "z" if modifiers.shift() => Some(Message::Redo),
                "z" => Some(Message::Undo),
                "s" if modifiers.shift() => Some(Message::SaveAs),
                "s" => Some(Message::Save),
                "o" => Some(Message::Open),
                "n" => Some(Message::New),
                "b" => Some(Message::Compile),
                "w" => Some(Message::CloseActiveWindow),
                "q" => Some(Message::QuitAll),
                _ => None,
            },
            _ => None,
        });

        let window_events = iced::event::listen_with(|event, _status, id| match event {
            iced::Event::Window(iced::window::Event::CloseRequested) => {
                Some(Message::CloseRequested(id))
            }
            iced::Event::Window(iced::window::Event::Closed) => Some(Message::WindowClosed(id)),
            iced::Event::Window(iced::window::Event::Focused) => Some(Message::WindowFocused(id)),
            iced::Event::Window(iced::window::Event::Resized(size)) => {
                Some(Message::WindowResized(id, size))
            }
            iced::Event::Window(iced::window::Event::Moved(pos)) => {
                Some(Message::WindowMoved(id, pos))
            }
            _ => None,
        });

        Subscription::batch(vec![keyboard_sub, window_events])
    }

    fn open_library_function(&mut self, source: &str) -> Task<Message> {
        use flowcore::provider::Provider;

        let provider = file_ops::build_meta_provider();
        let Ok(source_url) = Url::parse(source) else {
            return Task::none();
        };
        let Ok((resolved_url, _)) = provider.resolve_url(&source_url, "default", &["toml"]) else {
            return Task::none();
        };
        let Ok(path) = resolved_url.to_file_path() else {
            return Task::none();
        };

        // Check if already open
        for (&win_id, win) in &self.windows {
            if self.window_has_path(win, &path) {
                return window::gain_focus(win_id);
            }
        }

        // Read and parse
        let Ok(contents) = std::fs::read_to_string(&path) else {
            return Task::none();
        };
        let Ok(url) = Url::from_file_path(&path) else {
            return Task::none();
        };
        let Ok(deserializer) = get::<Process>(&url) else {
            return Task::none();
        };

        match deserializer.deserialize(&contents, Some(&url)) {
            Ok(Process::FunctionProcess(ref func)) => {
                let Some(parent) = self.root_window else {
                    return Task::none();
                };
                self.open_function_viewer(
                    parent,
                    &path,
                    func,
                    &path.to_string_lossy(),
                    Route::default(),
                )
            }
            Ok(Process::FlowProcess(_)) => match file_ops::load_flow(&path) {
                Ok(mut flow_def) => {
                    let has_nodes = !flow_def.process_refs.is_empty();
                    let nc = flow_def.process_refs.len();
                    let ec = flow_def.connections.len();
                    let (new_id, open_task) =
                        window::open(self.child_window_settings(1024.0, 768.0));
                    if let Ok(url) = Url::from_file_path(&path) {
                        flow_def.source_url = url;
                    }
                    let child = WindowState {
                        route: flow_def.route.clone(),
                        kind: WindowKind::FlowEditor,
                        canvas_state: FlowCanvasState::default(),
                        status: format!("Library flow - {nc} nodes, {ec} connections"),
                        selected_node: None,
                        selected_connection: None,
                        history: EditHistory::default(),
                        auto_fit_pending: has_nodes,
                        auto_fit_enabled: true,
                        flow_hierarchy: FlowHierarchy::from_flow_definition(&flow_def),
                        tooltip: None,
                        initializer_editor: None,
                        is_root: false,
                        context_menu: None,
                        show_metadata: false,
                        last_size: None,
                        last_position: None,
                    };
                    self.windows.insert(new_id, child);
                    open_task.discard()
                }
                Err(_) => Task::none(),
            },
            Err(_) => Task::none(),
        }
    }

    fn update_lib_paths(&mut self) {
        let path_str = self.lib_paths.join(",");
        std::env::set_var("FLOW_LIB_PATH", &path_str);

        // Reload library catalogs with the updated search paths.
        // Gather lib_references from the root flow.
        let lib_refs = self.root_flow.lib_references.clone();
        let (lc, ad) = library_mgmt::load_library_catalogs(&lib_refs);
        self.library_cache = lc;
        self.all_definitions = ad;
        self.library_tree = LibraryTree::from_cache(&self.library_cache, &self.all_definitions);
    }

    #[allow(clippy::cast_precision_loss)]
    fn child_window_settings(&self, width: f32, height: f32) -> window::Settings {
        let n = self.windows.len() as f32;
        window::Settings {
            size: iced::Size::new(width, height),
            position: window::Position::Specific(iced::Point::new(
                200.0 + n * 30.0,
                150.0 + n * 30.0,
            )),
            ..Default::default()
        }
    }

    /// Open a sub-flow in a new in-process window, or show a status message
    /// if the node resolves to a function rather than a flow.
    fn open_node(&mut self, parent_win_id: window::Id, idx: usize) -> Task<Message> {
        let Some(win) = self.windows.get(&parent_win_id) else {
            return Task::none();
        };
        let parent_route = win.route.clone();

        // Get the parent flow's process_refs (may be a sub-flow, not root)
        let parent_flow = if parent_route == self.root_flow.route {
            &self.root_flow
        } else {
            match self.root_flow.process_from_route(&parent_route) {
                Some(Process::FlowProcess(f)) => f,
                _ => return Task::none(),
            }
        };
        let Some(pref) = parent_flow.process_refs.get(idx) else {
            return Task::none();
        };
        let alias = if pref.alias.is_empty() {
            crate::utils::derive_short_name(&pref.source)
        } else {
            pref.alias.clone()
        };
        let mut child_route = parent_route;
        child_route.extend(&Route::from(alias.as_str()));

        // Check if already open
        for (&win_id, win) in &self.windows {
            if win.route == child_route && win_id != parent_win_id {
                return window::gain_focus(win_id);
            }
        }

        // Assign default positions to sub-flow nodes
        if let Some(Process::FlowProcess(sub_flow)) =
            self.root_flow.process_from_route_mut(&child_route)
        {
            file_ops::assign_default_positions(sub_flow);
        }

        let process_info = self
            .root_flow
            .process_from_route(&child_route)
            .map(|p| match p {
                Process::FlowProcess(f) => (
                    true,
                    !f.process_refs.is_empty(),
                    f.process_refs.len(),
                    f.connections.len(),
                    None,
                ),
                Process::FunctionProcess(f) => (
                    false,
                    false,
                    0,
                    0,
                    Some((
                        f.clone(),
                        f.get_source_url().to_file_path().unwrap_or_default(),
                    )),
                ),
            });
        let Some((is_flow, has_nodes, nc, ec, func_info)) = process_info else {
            if let Some(win) = self.windows.get_mut(&parent_win_id) {
                win.status = format!("Could not find process at {child_route}");
            }
            return Task::none();
        };

        if is_flow {
            let (new_id, open_task) = window::open(self.child_window_settings(1024.0, 768.0));
            let child = WindowState {
                route: child_route,
                kind: WindowKind::FlowEditor,
                canvas_state: FlowCanvasState::default(),
                status: format!("Ready - {nc} nodes, {ec} connections"),
                selected_node: None,
                selected_connection: None,
                history: EditHistory::default(),
                auto_fit_pending: has_nodes,
                auto_fit_enabled: true,
                flow_hierarchy: FlowHierarchy::from_flow_definition(&self.root_flow),
                tooltip: None,
                initializer_editor: None,
                is_root: false,
                context_menu: None,
                show_metadata: false,
                last_size: None,
                last_position: None,
            };
            self.windows.insert(new_id, child);
            if let Some(win) = self.windows.get_mut(&parent_win_id) {
                win.status = format!("Opened: {alias}");
            }
            open_task.discard()
        } else if let Some((func_def, source_path)) = func_info {
            self.open_function_viewer(parent_win_id, &source_path, &func_def, &alias, child_route)
        } else {
            Task::none()
        }
    }

    fn open_function_viewer(
        &mut self,
        parent_win_id: window::Id,
        toml_path: &Path,
        func: &FunctionDefinition,
        node_source: &str,
        route: Route,
    ) -> Task<Message> {
        let dir = toml_path.parent().unwrap_or(Path::new("."));
        let func_name = &func.name;

        let rs_path = dir.join(&func.source);
        let rs_content = std::fs::read_to_string(&rs_path)
            .unwrap_or_else(|_| String::from("// Source file not found"));
        let docs_content = std::fs::read_to_string(dir.join(format!("{func_name}.md"))).ok();

        let (new_id, open_task) = window::open(self.child_window_settings(700.0, 500.0));

        let read_only = node_source.starts_with("lib://") || node_source.starts_with("context://");
        let mut func_def = func.clone();
        if let Ok(url) = Url::from_file_path(toml_path) {
            func_def.set_source_url(&url);
        }
        let viewer = FunctionViewer {
            func_def: func_def.clone(),
            rs_content,
            docs_content,
            active_tab: 0,
            parent_window: Some(parent_win_id),
            node_source: node_source.to_string(),
            read_only,
        };

        let mut func_flow_def = FlowDefinition {
            name: func_name.clone(),
            ..FlowDefinition::default()
        };
        if let Ok(url) = Url::from_file_path(toml_path) {
            func_flow_def.source_url = url;
        }
        let child = WindowState {
            route,
            kind: WindowKind::FunctionViewer(Box::new(viewer)),
            canvas_state: FlowCanvasState::default(),
            status: format!("Function: {func_name}"),
            selected_node: None,
            selected_connection: None,
            history: EditHistory::default(),
            auto_fit_pending: false,
            auto_fit_enabled: false,
            flow_hierarchy: FlowHierarchy::from_flow_definition(&func_flow_def),
            tooltip: None,
            initializer_editor: None,
            is_root: false,
            context_menu: None,
            show_metadata: false,
            last_size: None,
            last_position: None,
        };

        self.windows.insert(new_id, child);
        if let Some(win) = self.windows.get_mut(&parent_win_id) {
            win.status = format!("Opened function: {func_name}");
        }
        open_task.discard()
    }

    fn create_new_subflow(&mut self, target_id: window::Id) -> Task<Message> {
        // Use the target window (where the action was triggered) for adding the node
        let target_id = if self.windows.contains_key(&target_id) {
            target_id
        } else if let Some(root_id) = self.root_window {
            root_id
        } else {
            return Task::none();
        };

        // Get the parent flow's directory for relative path resolution
        let base_dir = WindowState::file_path_of(&self.root_flow)
            .as_ref()
            .and_then(|p| p.parent())
            .map(Path::to_path_buf);

        let Some(base) = base_dir else {
            if let Some(win) = self.windows.get_mut(&target_id) {
                win.status = String::from("Save the flow first before creating a sub-flow");
            }
            return Task::none();
        };

        // Prompt user to choose where to save the new sub-flow
        let dialog = rfd::FileDialog::new()
            .add_filter("Flow", &["toml"])
            .set_directory(&base)
            .set_file_name("new_subflow.toml");
        let Some(path) = dialog.save_file() else {
            return Task::none();
        };

        // Derive flow name from filename
        let flow_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("subflow")
            .to_string();

        // Create the sub-flow definition with empty content
        let mut flow_def = FlowDefinition {
            name: flow_name.clone(),
            ..FlowDefinition::default()
        };
        if let Ok(url) = Url::from_file_path(&path) {
            flow_def.source_url = url;
        }

        // Write the initial TOML file
        let toml = format!("flow = \"{flow_name}\"\n");
        if let Err(e) = std::fs::write(&path, &toml) {
            if let Some(win) = self.windows.get_mut(&target_id) {
                win.status = format!("Could not create sub-flow: {e}");
            }
            return Task::none();
        }

        // Compute relative source path from parent flow to new sub-flow
        let source = path.strip_prefix(&base).map_or_else(
            |_| path.to_string_lossy().to_string(),
            |p| p.to_string_lossy().to_string(),
        );
        // Strip .toml extension for the source reference
        let source = source.strip_suffix(".toml").unwrap_or(&source).to_string();

        // Add a process reference in the root flow
        {
            let alias = file_ops::generate_unique_alias(&flow_name, &self.root_flow.process_refs);
            let (x, y) = file_ops::next_node_position(&self.root_flow.process_refs);

            self.root_flow.process_refs.push(ProcessReference {
                alias: alias.clone(),
                source,
                initializations: std::collections::BTreeMap::new(),
                x: Some(x),
                y: Some(y),
                width: Some(180.0),
                height: Some(120.0),
            });
            if let Some(win) = self.windows.get_mut(&target_id) {
                win.history.mark_modified();
                win.canvas_state.request_redraw();
                win.trigger_auto_fit_if_enabled();
                win.status = format!("Created sub-flow: {alias}");
            }
        }

        // Open the new sub-flow in a child window
        let (new_id, open_task) = window::open(self.child_window_settings(1024.0, 768.0));

        let child = WindowState {
            route: flow_def.route.clone(),
            kind: WindowKind::FlowEditor,
            canvas_state: FlowCanvasState::default(),
            status: format!("New sub-flow: {flow_name}"),
            selected_node: None,
            selected_connection: None,
            history: EditHistory::default(),
            auto_fit_pending: false,
            auto_fit_enabled: true,
            flow_hierarchy: FlowHierarchy::from_flow_definition(&flow_def),
            tooltip: None,
            initializer_editor: None,
            is_root: false,
            context_menu: None,
            show_metadata: false,
            last_size: None,
            last_position: None,
        };

        self.windows.insert(new_id, child);
        open_task.discard()
    }

    fn create_new_function(&mut self, target_id: window::Id) -> Task<Message> {
        // Use the target window (where the action was triggered) for adding the node
        let target_id = if self.windows.contains_key(&target_id) {
            target_id
        } else if let Some(root_id) = self.root_window {
            root_id
        } else {
            return Task::none();
        };

        let base_dir = WindowState::file_path_of(&self.root_flow)
            .as_ref()
            .and_then(|p| p.parent())
            .map(Path::to_path_buf);

        let Some(base) = base_dir else {
            if let Some(win) = self.windows.get_mut(&target_id) {
                win.status = String::from("Save the flow first before creating a function");
            }
            return Task::none();
        };

        // Prompt user to choose where to save the new function definition
        let dialog = rfd::FileDialog::new()
            .add_filter("Flow Function", &["toml"])
            .set_directory(&base)
            .set_file_name("new_function.toml");
        let Some(path) = dialog.save_file() else {
            return Task::none();
        };

        let func_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("function")
            .to_string();

        let rs_filename = format!("{func_name}.rs");

        // Compute relative source from parent flow
        let source = path.strip_prefix(&base).map_or_else(
            |_| path.to_string_lossy().to_string(),
            |p| p.to_string_lossy().to_string(),
        );
        let source = source.strip_suffix(".toml").unwrap_or(&source).to_string();

        // Add process reference in the root flow
        {
            let alias = file_ops::generate_unique_alias(&func_name, &self.root_flow.process_refs);
            let (x, y) = file_ops::next_node_position(&self.root_flow.process_refs);

            self.root_flow.process_refs.push(ProcessReference {
                alias: alias.clone(),
                source: source.clone(),
                initializations: std::collections::BTreeMap::new(),
                x: Some(x),
                y: Some(y),
                width: Some(180.0),
                height: Some(120.0),
            });
            if let Some(win) = self.windows.get_mut(&target_id) {
                win.history.mark_modified();
                win.canvas_state.request_redraw();
                win.trigger_auto_fit_if_enabled();
                win.status = format!("Created function: {alias}");
            }
        }

        let (new_id, open_task) = window::open(self.child_window_settings(700.0, 500.0));
        let viewer =
            Self::build_new_function_viewer(&func_name, &rs_filename, &source, &path, target_id);

        let mut func_flow_def = FlowDefinition {
            name: func_name,
            ..FlowDefinition::default()
        };
        if let Ok(url) = Url::from_file_path(&path) {
            func_flow_def.source_url = url;
        }
        let mut child = WindowState {
            route: func_flow_def.route.clone(),
            kind: WindowKind::FunctionViewer(Box::new(viewer)),
            canvas_state: FlowCanvasState::default(),
            status: String::from("New function — add ports and Save"),
            selected_node: None,
            selected_connection: None,
            history: EditHistory::default(),
            auto_fit_pending: false,
            auto_fit_enabled: false,
            flow_hierarchy: FlowHierarchy::from_flow_definition(&func_flow_def),
            tooltip: None,
            initializer_editor: None,
            is_root: false,
            context_menu: None,
            show_metadata: false,
            last_size: None,
            last_position: None,
        };
        child.history.mark_modified(); // New function starts dirty

        self.windows.insert(new_id, child);
        open_task.discard()
    }

    #[allow(clippy::needless_pass_by_value)]
    fn handle_file_message(&mut self, message: Message) {
        match message {
            Message::Save => {
                let target = self.focused_window.or(self.root_window);
                if let Some(win) = target.and_then(|id| self.windows.get_mut(&id)) {
                    match &win.kind {
                        WindowKind::FunctionViewer(_) => {
                            if let WindowKind::FunctionViewer(ref v) = win.kind {
                                match file_ops::save_function_definition(v) {
                                    Ok(()) => {
                                        win.history.clear();
                                        win.status = String::from("Function saved");
                                    }
                                    Err(e) => win.status = format!("Save failed: {e}"),
                                }
                            }
                        }
                        WindowKind::FlowEditor => win.handle_save(&mut self.root_flow),
                    }
                }
            }
            Message::SaveAs => {
                let target = self.focused_window.or(self.root_window);
                if let Some(win) = target.and_then(|id| self.windows.get_mut(&id)) {
                    match &win.kind {
                        WindowKind::FunctionViewer(_) => {
                            if let WindowKind::FunctionViewer(ref v) = win.kind {
                                match file_ops::save_function_definition(v) {
                                    Ok(()) => {
                                        win.history.clear();
                                        win.status = String::from("Function saved");
                                    }
                                    Err(e) => win.status = format!("Save failed: {e}"),
                                }
                            }
                        }
                        WindowKind::FlowEditor => win.handle_save_as(&mut self.root_flow),
                    }
                }
            }
            Message::Open => {
                if let Some(root_id) = self.root_window {
                    if let Some(win) = self.windows.get_mut(&root_id) {
                        if let Some((lib_refs, _ctx_refs)) = win.perform_open(&mut self.root_flow) {
                            win.flow_hierarchy =
                                FlowHierarchy::from_flow_definition(&self.root_flow);

                            let (lc, ad) = library_mgmt::load_library_catalogs(&lib_refs);
                            self.library_cache = lc;
                            self.all_definitions = ad;
                            self.library_tree =
                                LibraryTree::from_cache(&self.library_cache, &self.all_definitions);
                        }
                    }
                }
            }
            Message::New => {
                if let Some(win) = self.root_window.and_then(|id| self.windows.get_mut(&id)) {
                    win.perform_new(&mut self.root_flow);
                    win.flow_hierarchy = FlowHierarchy::empty();
                    self.library_cache.clear();
                    self.all_definitions.clear();
                    self.library_tree =
                        LibraryTree::from_cache(&self.library_cache, &self.all_definitions);
                }
            }
            Message::Compile => {
                let target = self.focused_window.or(self.root_window);
                if let Some(win) = target.and_then(|id| self.windows.get_mut(&id)) {
                    if !self.root_flow.process_refs.is_empty() {
                        match win.perform_compile(&mut self.root_flow) {
                            Ok(path) => {
                                win.history.set_compiled_manifest(path.clone());
                                win.status = format!("Compiled: {}", path.display());
                            }
                            Err(e) => {
                                win.status = e;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_func_description_changed(&mut self, win_id: window::Id, new_desc: &str) {
        let parent_id = self.windows.get(&win_id).and_then(|win| {
            if let WindowKind::FunctionViewer(ref viewer) = win.kind {
                viewer.parent_window
            } else {
                None
            }
        });
        if let Some(win) = self.windows.get_mut(&win_id) {
            if let WindowKind::FunctionViewer(ref mut viewer) = win.kind {
                viewer.func_def.description = new_desc.to_string();
            }
            win.history.mark_modified();
        }
        self.sync_func_def_to_root(win_id);
        if let Some(pid) = parent_id {
            if let Some(parent_win) = self.windows.get_mut(&pid) {
                parent_win.canvas_state.request_redraw();
            }
        }
    }

    fn handle_func_browse_source(&mut self, win_id: window::Id) {
        let dialog = rfd::FileDialog::new().add_filter("Rust", &["rs"]);
        if let Some(selected) = dialog.pick_file() {
            if let Some(win) = self.windows.get_mut(&win_id) {
                if let WindowKind::FunctionViewer(ref mut viewer) = win.kind {
                    let base = viewer
                        .toml_path()
                        .as_deref()
                        .and_then(Path::parent)
                        .unwrap_or(Path::new("."))
                        .to_path_buf();
                    let rel = selected.strip_prefix(&base).map_or_else(
                        |_| selected.to_string_lossy().to_string(),
                        |p| p.to_string_lossy().to_string(),
                    );
                    viewer.func_def.source = rel;
                    viewer.rs_content = std::fs::read_to_string(&selected)
                        .unwrap_or_else(|_| String::from("// Could not read file"));
                }
                win.history.mark_modified();
            }
            self.sync_func_def_to_root(win_id);
        }
    }

    fn handle_func_save(&mut self, win_id: window::Id) {
        if let Some(win) = self.windows.get_mut(&win_id) {
            if let WindowKind::FunctionViewer(ref v) = win.kind {
                match file_ops::save_function_definition(v) {
                    Ok(()) => {
                        let path_display = v
                            .toml_path()
                            .map_or_else(|| String::from("(unknown)"), |p| p.display().to_string());
                        win.status = format!("Saved: {path_display}");
                        win.history.clear();
                    }
                    Err(e) => {
                        win.status = format!("Save failed: {e}");
                    }
                }
            }
        }
    }

    fn handle_func_io_name_changed(
        &mut self,
        win_id: window::Id,
        idx: usize,
        name: &str,
        is_input: bool,
    ) {
        let old_name = self.windows.get(&win_id).and_then(|win| {
            if let WindowKind::FunctionViewer(ref v) = win.kind {
                let ports = if is_input {
                    &v.func_def.inputs
                } else {
                    &v.func_def.outputs
                };
                let duplicate = ports
                    .iter()
                    .enumerate()
                    .any(|(i, io)| i != idx && io.name() == name);
                if duplicate {
                    return None;
                }
                ports.get(idx).map(|io| io.name().clone())
            } else {
                None
            }
        });
        let Some(ref old) = old_name else {
            return;
        };
        if old == name {
            return;
        }
        if let Some(win) = self.windows.get_mut(&win_id) {
            if let WindowKind::FunctionViewer(ref mut v) = win.kind {
                let ports = if is_input {
                    &mut v.func_def.inputs
                } else {
                    &mut v.func_def.outputs
                };
                if let Some(io) = ports.get_mut(idx) {
                    io.set_name(name.into());
                }
            }
            win.history.mark_modified();
        }
        self.sync_func_def_to_root(win_id);
        Self::propagate_function_ports(&mut self.windows, &mut self.root_flow, win_id);
        Self::rename_parent_connections_port(
            &mut self.windows,
            &mut self.root_flow,
            win_id,
            old,
            name,
            is_input,
        );
    }

    /// Handle function definition viewing/editing messages.
    fn handle_func_add_io(&mut self, win_id: window::Id, is_input: bool) {
        if let Some(win) = self.windows.get_mut(&win_id) {
            if let WindowKind::FunctionViewer(ref mut v) = win.kind {
                let (prefix, ports) = if is_input {
                    ("input", &v.func_def.inputs as &[IO])
                } else {
                    ("output", &v.func_def.outputs as &[IO])
                };
                let name = utils::next_unique_io_name(prefix, ports);
                let io = IO::new_named(vec![DataType::from("string")], Route::default(), &name);
                if is_input {
                    v.func_def.inputs.push(io);
                } else {
                    v.func_def.outputs.push(io);
                }
            }
            win.history.mark_modified();
        }
        self.sync_func_def_to_root(win_id);
        Self::propagate_function_ports(&mut self.windows, &mut self.root_flow, win_id);
    }

    fn handle_func_delete_io(&mut self, win_id: window::Id, idx: usize, is_input: bool) {
        let old_name = self.windows.get(&win_id).and_then(|win| {
            if let WindowKind::FunctionViewer(ref v) = win.kind {
                let ports = if is_input {
                    &v.func_def.inputs
                } else {
                    &v.func_def.outputs
                };
                ports.get(idx).map(|io| io.name().clone())
            } else {
                None
            }
        });
        if let Some(win) = self.windows.get_mut(&win_id) {
            if let WindowKind::FunctionViewer(ref mut v) = win.kind {
                let ports = if is_input {
                    &mut v.func_def.inputs
                } else {
                    &mut v.func_def.outputs
                };
                if idx < ports.len() {
                    ports.remove(idx);
                }
            }
            win.history.mark_modified();
        }
        self.sync_func_def_to_root(win_id);
        Self::propagate_function_ports(&mut self.windows, &mut self.root_flow, win_id);
        if let Some(port_name) = old_name {
            Self::remove_parent_connections_to_port(
                &mut self.windows,
                &mut self.root_flow,
                win_id,
                &port_name,
                is_input,
            );
        }
    }

    fn handle_func_io_type_changed(
        &mut self,
        win_id: window::Id,
        idx: usize,
        dtype: String,
        is_input: bool,
    ) {
        if let Some(win) = self.windows.get_mut(&win_id) {
            if let WindowKind::FunctionViewer(ref mut v) = win.kind {
                let ports = if is_input {
                    &mut v.func_def.inputs
                } else {
                    &mut v.func_def.outputs
                };
                if let Some(io) = ports.get_mut(idx) {
                    io.set_datatypes(&[DataType::from(dtype)]);
                }
            }
            win.history.mark_modified();
        }
        self.sync_func_def_to_root(win_id);
        Self::propagate_function_ports(&mut self.windows, &mut self.root_flow, win_id);
    }

    fn build_new_function_viewer(
        func_name: &str,
        rs_filename: &str,
        node_source: &str,
        path: &Path,
        parent_id: window::Id,
    ) -> FunctionViewer {
        let mut func_def = FunctionDefinition::default();
        func_def.name = func_name.into();
        func_def.source = rs_filename.into();
        if let Ok(url) = Url::from_file_path(path) {
            func_def.set_source_url(&url);
        }
        FunctionViewer {
            func_def,
            rs_content: String::from("// Save to generate skeleton source"),
            docs_content: None,
            active_tab: 0,
            parent_window: Some(parent_id),
            node_source: node_source.into(),
            read_only: false,
        }
    }

    /// Sync a function viewer's `func_def` back to the canonical copy in `root_flow.subprocesses`.
    ///
    /// After any mutation to a function viewer's definition, this method ensures the
    /// root flow's subprocess map stays in sync so that the canvas and other views
    /// reflect the edits.
    fn sync_func_def_to_root(&mut self, win_id: window::Id) {
        let sync_data = self.windows.get(&win_id).and_then(|win| {
            if !win.route.is_empty() {
                if let WindowKind::FunctionViewer(ref viewer) = win.kind {
                    return Some((win.route.clone(), viewer.func_def.clone()));
                }
            }
            // Fall back to source-based lookup for windows without a route
            if let WindowKind::FunctionViewer(ref viewer) = win.kind {
                return Some((Route::default(), viewer.func_def.clone()));
            }
            None
        });
        let Some((route, func_def)) = sync_data else {
            return;
        };
        // Try route-based sync first
        if !route.is_empty() {
            if let Some(Process::FunctionProcess(ref mut canonical)) =
                self.root_flow.process_from_route_mut(&route)
            {
                *canonical = func_def;
                return;
            }
        }
        // Fall back to source-based lookup (for viewers opened without hierarchy navigation)
        let node_source = self
            .windows
            .get(&win_id)
            .and_then(|win| {
                if let WindowKind::FunctionViewer(ref viewer) = win.kind {
                    Some(viewer.node_source.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();
        if !node_source.is_empty() {
            for pref in &self.root_flow.process_refs {
                if pref.source == node_source {
                    let alias = if pref.alias.is_empty() {
                        utils::derive_short_name(&pref.source)
                    } else {
                        pref.alias.clone()
                    };
                    if let Some(Process::FunctionProcess(ref mut canonical)) =
                        self.root_flow.subprocesses.get_mut(&alias)
                    {
                        *canonical = func_def;
                        return;
                    }
                }
            }
        }
    }

    fn handle_function_edit_message(&mut self, win_id: window::Id, func_msg: FunctionEditMessage) {
        match func_msg {
            FunctionEditMessage::TabSelected(tab) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    if let WindowKind::FunctionViewer(ref mut viewer) = win.kind {
                        viewer.active_tab = tab;
                    }
                }
            }
            FunctionEditMessage::NameChanged(new_name) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    if let WindowKind::FunctionViewer(ref mut viewer) = win.kind {
                        viewer.func_def.name = new_name;
                    }
                    win.history.mark_modified();
                }
                self.sync_func_def_to_root(win_id);
            }
            FunctionEditMessage::DescriptionChanged(new_desc) => {
                self.handle_func_description_changed(win_id, &new_desc);
            }
            FunctionEditMessage::BrowseSource => self.handle_func_browse_source(win_id),
            FunctionEditMessage::AddInput => self.handle_func_add_io(win_id, true),
            FunctionEditMessage::AddOutput => self.handle_func_add_io(win_id, false),
            FunctionEditMessage::DeleteInput(idx) => {
                self.handle_func_delete_io(win_id, idx, true);
            }
            FunctionEditMessage::DeleteOutput(idx) => {
                self.handle_func_delete_io(win_id, idx, false);
            }
            FunctionEditMessage::InputNameChanged(idx, name) => {
                self.handle_func_io_name_changed(win_id, idx, &name, true);
            }
            FunctionEditMessage::InputTypeChanged(idx, dtype) => {
                self.handle_func_io_type_changed(win_id, idx, dtype, true);
            }
            FunctionEditMessage::OutputNameChanged(idx, name) => {
                self.handle_func_io_name_changed(win_id, idx, &name, false);
            }
            FunctionEditMessage::OutputTypeChanged(idx, dtype) => {
                self.handle_func_io_type_changed(win_id, idx, dtype, false);
            }
            FunctionEditMessage::Save => self.handle_func_save(win_id),
        }
    }

    /// Propagate the current function viewer's ports (inputs/outputs) to matching
    /// nodes in the parent canvas window. This keeps the parent canvas node's port
    /// display in sync when ports are added, deleted, or renamed in the function viewer.
    fn propagate_function_ports(
        windows: &mut HashMap<window::Id, WindowState>,
        root_flow: &mut FlowDefinition,
        viewer_win_id: window::Id,
    ) {
        // Extract parent info and current ports from the viewer window
        let propagation_data = windows.get(&viewer_win_id).and_then(|win| {
            if let WindowKind::FunctionViewer(ref viewer) = win.kind {
                viewer.parent_window.map(|pid| {
                    (
                        pid,
                        viewer.node_source.clone(),
                        viewer.func_def.inputs.clone(),
                        viewer.func_def.outputs.clone(),
                    )
                })
            } else {
                None
            }
        });

        if let Some((parent_id, node_source, new_inputs, new_outputs)) = propagation_data {
            // Update subprocess definitions in root_flow for all process refs with matching source
            for pref in &root_flow.process_refs {
                if pref.source == node_source {
                    let alias = if pref.alias.is_empty() {
                        utils::derive_short_name(&pref.source)
                    } else {
                        pref.alias.clone()
                    };
                    if let Some(proc) = root_flow.subprocesses.get_mut(&alias) {
                        match proc {
                            Process::FunctionProcess(ref mut f) => {
                                f.inputs.clone_from(&new_inputs);
                                f.outputs.clone_from(&new_outputs);
                            }
                            Process::FlowProcess(ref mut f) => {
                                f.inputs.clone_from(&new_inputs);
                                f.outputs.clone_from(&new_outputs);
                            }
                        }
                    }
                }
            }
            if let Some(parent_win) = windows.get_mut(&parent_id) {
                parent_win.canvas_state.request_redraw();
            }
        }
    }

    fn remove_parent_connections_to_port(
        windows: &mut HashMap<window::Id, WindowState>,
        root_flow: &mut FlowDefinition,
        viewer_win_id: window::Id,
        port_name: &str,
        is_input: bool,
    ) {
        let info = windows.get(&viewer_win_id).and_then(|win| {
            if let WindowKind::FunctionViewer(ref v) = win.kind {
                v.parent_window.map(|pid| (pid, v.node_source.clone()))
            } else {
                None
            }
        });
        if let Some((parent_id, node_source)) = info {
            for pref in &root_flow.process_refs {
                if pref.source != node_source {
                    continue;
                }
                let alias = if pref.alias.is_empty() {
                    utils::derive_short_name(&pref.source)
                } else {
                    pref.alias.clone()
                };
                let route = format!("{alias}/{port_name}");
                if is_input {
                    for conn in &mut root_flow.connections {
                        let new_to: Vec<Route> = conn
                            .to()
                            .iter()
                            .filter(|r| r.as_ref() != route)
                            .cloned()
                            .collect();
                        conn.set_to(new_to);
                    }
                    root_flow.connections.retain(|c| !c.to().is_empty());
                } else {
                    root_flow.connections.retain(|c| c.from().as_ref() != route);
                }
            }
            if let Some(parent_win) = windows.get_mut(&parent_id) {
                parent_win.canvas_state.request_redraw();
            }
        }
    }

    fn rename_parent_connections_port(
        windows: &mut HashMap<window::Id, WindowState>,
        root_flow: &mut FlowDefinition,
        viewer_win_id: window::Id,
        old_port: &str,
        new_port: &str,
        is_input: bool,
    ) {
        let info = windows.get(&viewer_win_id).and_then(|win| {
            if let WindowKind::FunctionViewer(ref v) = win.kind {
                v.parent_window.map(|pid| (pid, v.node_source.clone()))
            } else {
                None
            }
        });
        if let Some((parent_id, node_source)) = info {
            for pref in &root_flow.process_refs {
                if pref.source != node_source {
                    continue;
                }
                let alias = if pref.alias.is_empty() {
                    utils::derive_short_name(&pref.source)
                } else {
                    pref.alias.clone()
                };
                let old_route = format!("{alias}/{old_port}");
                let new_route = format!("{alias}/{new_port}");
                for conn in &mut root_flow.connections {
                    if is_input {
                        let new_to: Vec<Route> = conn
                            .to()
                            .iter()
                            .map(|r| {
                                if r.as_ref() == old_route {
                                    Route::from(new_route.as_str())
                                } else {
                                    r.clone()
                                }
                            })
                            .collect();
                        conn.set_to(new_to);
                    } else if conn.from().as_ref() == old_route {
                        conn.set_from(new_route.as_str());
                    }
                }
            }
            if let Some(parent_win) = windows.get_mut(&parent_id) {
                parent_win.canvas_state.request_redraw();
            }
        }
    }
}
