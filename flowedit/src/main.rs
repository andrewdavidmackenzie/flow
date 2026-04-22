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
use flowcore::model::io::IO;
use flowcore::model::lib_manifest::LibraryManifest;
use flowcore::model::name::HasName;
use flowcore::model::process::Process;
use flowcore::model::process_reference::ProcessReference;
use flowcore::model::route::Route;
use flowcore::provider::Provider;

use canvas_view::{CanvasMessage, FlowCanvasState, NodeLayout, PortInfo};
use hierarchy_panel::{FlowHierarchy, HierarchyMessage};
use history::EditHistory;
use library_panel::{LibraryAction, LibraryMessage, LibraryTree};

mod canvas_view;
mod flow_io;
mod hierarchy_panel;
mod history;
mod initializer;
mod library_mgmt;
mod library_panel;
mod window_state;

pub(crate) use window_state::{FunctionViewer, InitializerEditor, WindowKind, WindowState};

#[cfg(test)]
mod ui_test;

/// Messages for flow metadata and I/O editing, tagged by window
#[derive(Debug, Clone)]
enum FlowEditMessage {
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
    /// Flow input port type changed
    InputTypeChanged(usize, String),
    /// Flow output port name changed
    OutputNameChanged(usize, String),
    /// Flow output port type changed
    OutputTypeChanged(usize, String),
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

/// Messages handled by the flowedit application
#[derive(Debug, Clone)]
enum Message {
    /// A message from the interactive canvas, tagged with its window ID
    WindowCanvas(window::Id, CanvasMessage),
    /// A message from the library side panel, tagged with the originating window ID
    Library(window::Id, LibraryMessage),
    /// A message from the flow hierarchy panel, tagged with window ID
    Hierarchy(window::Id, HierarchyMessage),
    /// Zoom in by one step, tagged with the originating window ID
    ZoomIn(window::Id),
    /// Zoom out by one step, tagged with the originating window ID
    ZoomOut(window::Id),
    /// Toggle auto-fit mode, tagged with the originating window ID
    ToggleAutoFit(window::Id),
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
    /// Flow metadata and I/O editing messages
    FlowEdit(window::Id, FlowEditMessage),
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

impl FlowEdit {
    /// Create the application, parsing CLI args and optionally loading a flow file.
    fn new() -> (Self, Task<Message>) {
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

        // Collect -L library directories, same as flowrgui
        let lib_dirs: Vec<String> = if matches.contains_id("lib_dir") {
            matches
                .get_many::<String>("lib_dir")
                .map(|dirs| dirs.map(std::string::ToString::to_string).collect())
                .unwrap_or_default()
        } else {
            vec![]
        };

        // Build the library search path from FLOW_LIB_PATH + -L args
        let mut lib_search_path = Simpath::new_with_separator("FLOW_LIB_PATH", ',');
        for addition in &lib_dirs {
            lib_search_path.add(addition);
            info!("'{addition}' added to the Library Search Path");
        }
        if lib_search_path.is_empty() {
            if let Ok(home) = std::env::var("HOME") {
                lib_search_path.add(&format!("{home}/.flow/lib"));
            }
        }

        // Set FLOW_LIB_PATH with any -L additions so other code can find libraries
        if !lib_dirs.is_empty() {
            let current = std::env::var("FLOW_LIB_PATH").unwrap_or_default();
            let additions = lib_dirs.join(",");
            if current.is_empty() {
                std::env::set_var("FLOW_LIB_PATH", additions);
            } else {
                std::env::set_var("FLOW_LIB_PATH", format!("{current},{additions}"));
            }
        }

        let (nodes, edges, status, flow_definition, lib_refs) =
            if let Some(flow_path_str) = matches.get_one::<String>("flow-file") {
                let flow_path = PathBuf::from(flow_path_str);
                match flow_io::load_flow(&flow_path) {
                    Ok(loaded) => {
                        let nc = loaded.nodes.len();
                        let ec = loaded.edges.len();
                        let mut fd = loaded.flow_def;
                        if let Ok(url) = Url::from_file_path(&flow_path) {
                            fd.source_url = url;
                        }
                        (
                            loaded.nodes,
                            loaded.edges,
                            format!("Ready - {nc} nodes, {ec} connections"),
                            fd,
                            loaded.lib_references,
                        )
                    }
                    Err(e) => {
                        let fd = FlowDefinition {
                            name: String::from("(error)"),
                            ..FlowDefinition::default()
                        };
                        (
                            Vec::new(),
                            Vec::new(),
                            format!("Error loading flow: {e}"),
                            fd,
                            BTreeSet::new(),
                        )
                    }
                }
            } else {
                let fd = FlowDefinition {
                    name: String::from("(new flow)"),
                    ..FlowDefinition::default()
                };
                (
                    Vec::new(),
                    Vec::new(),
                    String::from("Ready"),
                    fd,
                    BTreeSet::new(),
                )
            };

        let has_nodes = !nodes.is_empty();

        // Load full library catalogs from manifests and parse all definitions
        let (library_cache, all_definitions) = library_mgmt::load_library_catalogs(&lib_refs);
        let library_tree = LibraryTree::from_cache(&library_cache, &all_definitions);

        // Open the root window via daemon API
        let file_path = flow_definition.source_url.to_file_path().ok();
        let saved_prefs = file_path
            .as_ref()
            .and_then(|p| flow_io::load_editor_prefs(p));
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

        let flow_hierarchy = file_path
            .as_ref()
            .map_or_else(FlowHierarchy::empty, |p| FlowHierarchy::build(p));

        let win_state = WindowState {
            kind: WindowKind::FlowEditor,
            nodes,
            edges,
            canvas_state: FlowCanvasState::default(),
            status,
            selected_node: None,
            selected_connection: None,
            auto_fit_pending: has_nodes,
            auto_fit_enabled: true,
            history: EditHistory::default(),
            unsaved_edits: 0,
            compiled_manifest: None,
            flow_definition,
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

        let lib_paths = flow_io::resolve_lib_paths();
        let app = FlowEdit {
            windows,
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
            let modified = if win.unsaved_edits > 0 { " *" } else { "" };
            let file = win
                .file_path()
                .as_ref()
                .and_then(|p| p.file_name().map(ToOwned::to_owned))
                .and_then(|n| n.to_str().map(String::from))
                .unwrap_or_else(|| String::from("untitled"));
            format!(
                "flowedit - {} ({}){modified}",
                win.flow_definition.name, file
            )
        } else {
            String::from("flowedit")
        }
    }

    /// Handle messages from canvas interactions.
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WindowCanvas(win_id, canvas_msg) => {
                let Some(win) = self.windows.get_mut(&win_id) else {
                    return Task::none();
                };
                match canvas_view::handle_canvas_message(win, canvas_msg) {
                    canvas_view::CanvasAction::OpenNode(idx) => {
                        return self.open_node(win_id, idx);
                    }
                    canvas_view::CanvasAction::None => {}
                }
            }
            Message::Hierarchy(hier_win_id, ref hier_msg) => {
                let open_result = self
                    .windows
                    .get_mut(&hier_win_id)
                    .and_then(|win| win.flow_hierarchy.update(hier_msg));
                if let Some((_source, path)) = open_result {
                    // Check if already open
                    for (&win_id, win) in &self.windows {
                        if win.file_path().as_ref() == Some(&path) {
                            return window::gain_focus(win_id);
                        }
                    }
                    // Open the flow or function
                    if let Ok(loaded) = flow_io::load_flow(&path) {
                        let (new_id, open_task) =
                            window::open(self.child_window_settings(1024.0, 768.0));
                        let has_nodes = !loaded.nodes.is_empty();
                        let nc = loaded.nodes.len();
                        let ec = loaded.edges.len();
                        let mut flow_def = loaded.flow_def;
                        if let Ok(url) = Url::from_file_path(&path) {
                            flow_def.source_url = url;
                        }
                        let child = WindowState {
                            kind: WindowKind::FlowEditor,
                            nodes: loaded.nodes,
                            edges: loaded.edges,
                            canvas_state: FlowCanvasState::default(),
                            status: format!("Ready - {nc} nodes, {ec} connections"),
                            selected_node: None,
                            selected_connection: None,
                            history: EditHistory::default(),
                            auto_fit_pending: has_nodes,
                            auto_fit_enabled: true,
                            unsaved_edits: 0,
                            compiled_manifest: None,
                            flow_definition: flow_def,
                            tooltip: None,
                            initializer_editor: None,
                            is_root: false,
                            context_menu: None,
                            show_metadata: false,
                            flow_hierarchy: self.build_hierarchy(),
                            last_size: None,
                            last_position: None,
                        };
                        self.windows.insert(new_id, child);
                        return open_task.discard();
                    }
                    // Try as function definition
                    let abs = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
                    if let Ok(contents) = std::fs::read_to_string(&abs) {
                        if let Ok(url) = Url::from_file_path(&abs) {
                            if let Ok(deser) = get::<Process>(&url) {
                                if let Ok(Process::FunctionProcess(ref func)) =
                                    deser.deserialize(&contents, Some(&url))
                                {
                                    return self.open_function_viewer(
                                        hier_win_id,
                                        &path,
                                        func,
                                        &path.to_string_lossy(),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Message::Library(win_id, ref lib_msg) => match self.library_tree.update(lib_msg) {
                LibraryAction::Add(source, func_name) => {
                    if let Some(win) = self.windows.get_mut(&win_id) {
                        library_mgmt::add_library_function(win, &source, &func_name);
                    }
                }
                LibraryAction::View(source, _name) => {
                    return self.open_library_function(&source);
                }
                LibraryAction::AddLibrary => {
                    let dialog = rfd::FileDialog::new();
                    if let Some(dir) = dialog.pick_folder() {
                        let lib_name = dir
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();

                        // Construct lib:// URL from directory name
                        if let Ok(lib_url) = Url::parse(&format!("lib://{lib_name}")) {
                            // Add parent directory to provider's search path
                            if let Some(parent) = dir.parent() {
                                if let Some(parent_str) = parent.to_str() {
                                    let mut lib_search_path =
                                        Simpath::new_with_separator("FLOW_LIB_PATH", ',');
                                    lib_search_path.add_directory(parent_str);

                                    // Add default lib path if it exists
                                    if let Ok(home) = std::env::var("HOME") {
                                        let default_lib =
                                            PathBuf::from(&home).join(".flow").join("lib");
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
                                    let provider = MetaProvider::new(
                                        lib_search_path.clone(),
                                        context_root.clone(),
                                    );
                                    let arc_provider: Arc<dyn Provider> = Arc::new(provider);

                                    // Load the library manifest
                                    match LibraryManifest::load(&arc_provider, &lib_url) {
                                        Ok((manifest, _manifest_url)) => {
                                            info!(
                                                "Loaded library manifest for '{}' with {} locators",
                                                lib_url,
                                                manifest.locators.len()
                                            );

                                            // Parse all definitions in the manifest
                                            for locator_url in manifest.locators.keys() {
                                                let meta_provider = MetaProvider::new(
                                                    lib_search_path.clone(),
                                                    context_root.clone(),
                                                );
                                                match flowrclib::compiler::parser::parse(
                                                    locator_url,
                                                    &meta_provider,
                                                ) {
                                                    Ok(process) => {
                                                        self.all_definitions
                                                            .insert(locator_url.clone(), process);
                                                    }
                                                    Err(e) => {
                                                        warn!(
                                                            "Could not parse library definition '{locator_url}': {e}"
                                                        );
                                                    }
                                                }
                                            }

                                            // Add to library cache
                                            self.library_cache.insert(lib_url.clone(), manifest);

                                            // Persist the parent directory so that
                                            // subsequent MetaProvider rebuilds can
                                            // resolve this library.
                                            if !self.lib_paths.contains(&parent_str.to_string()) {
                                                self.lib_paths.push(parent_str.to_string());
                                                self.update_lib_paths();
                                            }

                                            // Rebuild the library tree
                                            self.library_tree = LibraryTree::from_cache(
                                                &self.library_cache,
                                                &self.all_definitions,
                                            );

                                            if let Some(win) = self.windows.get_mut(&win_id) {
                                                win.status = format!("Added library: {lib_name}");
                                            }
                                        }
                                        Err(e) => {
                                            warn!(
                                                "Could not load library manifest for '{lib_url}': {e}"
                                            );
                                            if let Some(win) = self.windows.get_mut(&win_id) {
                                                win.status = format!(
                                                    "Failed to load library '{lib_name}': {e}"
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                LibraryAction::None => {}
            },
            Message::ZoomIn(win_id) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    canvas_view::handle_zoom_in(win);
                }
            }
            Message::ZoomOut(win_id) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    canvas_view::handle_zoom_out(win);
                }
            }
            Message::ToggleAutoFit(win_id) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    canvas_view::handle_toggle_auto_fit(win);
                }
            }
            Message::Undo => {
                let target = self.focused_window.or(self.root_window);
                if let Some(win) = target.and_then(|id| self.windows.get_mut(&id)) {
                    win.handle_undo();
                }
            }
            Message::Redo => {
                let target = self.focused_window.or(self.root_window);
                if let Some(win) = target.and_then(|id| self.windows.get_mut(&id)) {
                    win.handle_redo();
                }
            }
            Message::Save => {
                let target = self.focused_window.or(self.root_window);
                if let Some(win) = target.and_then(|id| self.windows.get_mut(&id)) {
                    flow_io::handle_save(win);
                }
            }
            Message::SaveAs => {
                let target = self.focused_window.or(self.root_window);
                if let Some(win) = target.and_then(|id| self.windows.get_mut(&id)) {
                    flow_io::handle_save_as(win);
                }
            }
            Message::Open => {
                if let Some(root_id) = self.root_window {
                    if let Some(win) = self.windows.get_mut(&root_id) {
                        if let Some((lib_refs, _ctx_refs)) = flow_io::perform_open(win) {
                            win.flow_hierarchy = win
                                .file_path()
                                .as_ref()
                                .map_or_else(FlowHierarchy::empty, |p| FlowHierarchy::build(p));

                            // Rebuild library cache with new flow's references
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
                    flow_io::perform_new(win);
                    // Clear the library cache for a new (empty) flow
                    self.library_cache.clear();
                    self.all_definitions.clear();
                    self.library_tree =
                        LibraryTree::from_cache(&self.library_cache, &self.all_definitions);
                }
            }
            Message::Compile => {
                let target = self.focused_window.or(self.root_window);
                if let Some(win) = target.and_then(|id| self.windows.get_mut(&id)) {
                    if !win.nodes.is_empty() {
                        match flow_io::perform_compile(win) {
                            Ok(path) => {
                                win.compiled_manifest = Some(path.clone());
                                win.status = format!("Compiled: {}", path.display());
                            }
                            Err(e) => {
                                win.compiled_manifest = None;
                                win.status = e;
                            }
                        }
                    }
                }
            }
            Message::FlowEdit(win_id, flow_msg) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    match flow_msg {
                        FlowEditMessage::NameChanged(new_name) => {
                            win.flow_definition.name = new_name;
                            win.unsaved_edits += 1;
                        }
                        FlowEditMessage::VersionChanged(version) => {
                            win.flow_definition.metadata.version = version;
                            win.unsaved_edits += 1;
                        }
                        FlowEditMessage::DescriptionChanged(desc) => {
                            win.flow_definition.metadata.description = desc;
                            win.unsaved_edits += 1;
                        }
                        FlowEditMessage::AuthorsChanged(authors_str) => {
                            win.flow_definition.metadata.authors = authors_str
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                            win.unsaved_edits += 1;
                        }
                        FlowEditMessage::ToggleMetadata => {
                            win.show_metadata = !win.show_metadata;
                        }
                        FlowEditMessage::AddInput => {
                            let name = format!("input{}", win.flow_definition.inputs.len());
                            win.flow_definition.inputs.push(IO::new_named(
                                vec![DataType::from("string")],
                                Route::default(),
                                name,
                            ));
                            win.unsaved_edits += 1;
                            win.canvas_state.request_redraw();
                        }
                        FlowEditMessage::AddOutput => {
                            let name = format!("output{}", win.flow_definition.outputs.len());
                            win.flow_definition.outputs.push(IO::new_named(
                                vec![DataType::from("string")],
                                Route::default(),
                                name,
                            ));
                            win.unsaved_edits += 1;
                            win.canvas_state.request_redraw();
                        }
                        FlowEditMessage::DeleteInput(idx) => {
                            if let Some(io) = win.flow_definition.inputs.get(idx) {
                                let name = io.name().clone();
                                win.flow_definition.inputs.remove(idx);
                                // Remove edges referencing this flow input
                                win.edges
                                    .retain(|e| !(e.from_node == "input" && e.from_port == name));
                                win.unsaved_edits += 1;
                                win.canvas_state.request_redraw();
                            }
                        }
                        FlowEditMessage::DeleteOutput(idx) => {
                            if let Some(io) = win.flow_definition.outputs.get(idx) {
                                let name = io.name().clone();
                                win.flow_definition.outputs.remove(idx);
                                // Remove edges referencing this flow output
                                win.edges
                                    .retain(|e| !(e.to_node == "output" && e.to_port == name));
                                win.unsaved_edits += 1;
                                win.canvas_state.request_redraw();
                            }
                        }
                        FlowEditMessage::InputNameChanged(idx, name) => {
                            if let Some(io) = win.flow_definition.inputs.get_mut(idx) {
                                let old_name = io.name().clone();
                                io.set_name(name.clone());
                                // Update edges referencing the old flow input name
                                for edge in &mut win.edges {
                                    if edge.from_node == "input" && edge.from_port == old_name {
                                        edge.from_port.clone_from(&name);
                                    }
                                }
                            }
                            win.unsaved_edits += 1;
                            win.canvas_state.request_redraw();
                        }
                        FlowEditMessage::InputTypeChanged(idx, dtype) => {
                            if let Some(io) = win.flow_definition.inputs.get_mut(idx) {
                                io.set_datatypes(&[DataType::from(dtype)]);
                            }
                            win.unsaved_edits += 1;
                            win.canvas_state.request_redraw();
                        }
                        FlowEditMessage::OutputNameChanged(idx, name) => {
                            if let Some(io) = win.flow_definition.outputs.get_mut(idx) {
                                let old_name = io.name().clone();
                                io.set_name(name.clone());
                                // Update edges referencing the old flow output name
                                for edge in &mut win.edges {
                                    if edge.to_node == "output" && edge.to_port == old_name {
                                        edge.to_port.clone_from(&name);
                                    }
                                }
                            }
                            win.unsaved_edits += 1;
                            win.canvas_state.request_redraw();
                        }
                        FlowEditMessage::OutputTypeChanged(idx, dtype) => {
                            if let Some(io) = win.flow_definition.outputs.get_mut(idx) {
                                io.set_datatypes(&[DataType::from(dtype)]);
                            }
                            win.unsaved_edits += 1;
                            win.canvas_state.request_redraw();
                        }
                    }
                }
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
            Message::InitializerTypeChanged(win_id, new_type) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    initializer::handle_type_changed(win, new_type);
                }
            }
            Message::InitializerValueChanged(win_id, new_value) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    initializer::handle_value_changed(win, new_value);
                }
            }
            Message::InitializerApply(win_id) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    initializer::handle_apply(win);
                }
            }
            Message::InitializerCancel(win_id) => {
                if let Some(win) = self.windows.get_mut(&win_id) {
                    initializer::handle_cancel(win);
                }
            }
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
                                viewer.name = new_name;
                            }
                            win.unsaved_edits += 1;
                        }
                    }
                    FunctionEditMessage::DescriptionChanged(new_desc) => {
                        // Extract parent info before mutating
                        let parent_info = self.windows.get(&win_id).and_then(|win| {
                            if let WindowKind::FunctionViewer(ref viewer) = win.kind {
                                viewer
                                    .parent_window
                                    .map(|pid| (pid, viewer.node_source.clone()))
                            } else {
                                None
                            }
                        });
                        if let Some(win) = self.windows.get_mut(&win_id) {
                            if let WindowKind::FunctionViewer(ref mut viewer) = win.kind {
                                viewer.description.clone_from(&new_desc);
                            }
                            win.unsaved_edits += 1;
                        }
                        // Propagate to the parent window's NodeLayout for ALL nodes
                        // with the same source (multiple nodes may reference it)
                        if let Some((parent_id, node_source)) = parent_info {
                            if let Some(parent_win) = self.windows.get_mut(&parent_id) {
                                for node in &mut parent_win.nodes {
                                    if node.source == node_source {
                                        node.description.clone_from(&new_desc);
                                    }
                                }
                                parent_win.canvas_state.request_redraw();
                            }
                        }
                    }
                    FunctionEditMessage::BrowseSource => {
                        let dialog = rfd::FileDialog::new().add_filter("Rust", &["rs"]);
                        if let Some(selected) = dialog.pick_file() {
                            if let Some(win) = self.windows.get_mut(&win_id) {
                                if let WindowKind::FunctionViewer(ref mut viewer) = win.kind {
                                    let base = viewer.toml_path.parent().unwrap_or(Path::new("."));
                                    let rel = selected.strip_prefix(base).map_or_else(
                                        |_| selected.to_string_lossy().to_string(),
                                        |p| p.to_string_lossy().to_string(),
                                    );
                                    viewer.source_file = rel;
                                    viewer.rs_content = std::fs::read_to_string(&selected)
                                        .unwrap_or_else(|_| String::from("// Could not read file"));
                                }
                                win.unsaved_edits += 1;
                            }
                        }
                    }
                    FunctionEditMessage::AddInput => {
                        if let Some(win) = self.windows.get_mut(&win_id) {
                            if let WindowKind::FunctionViewer(ref mut v) = win.kind {
                                v.inputs.push(PortInfo {
                                    name: format!("input{}", v.inputs.len()),
                                    datatypes: vec![String::from("string")],
                                });
                            }
                            win.unsaved_edits += 1;
                        }
                        Self::propagate_function_ports(&mut self.windows, win_id);
                    }
                    FunctionEditMessage::AddOutput => {
                        if let Some(win) = self.windows.get_mut(&win_id) {
                            if let WindowKind::FunctionViewer(ref mut v) = win.kind {
                                v.outputs.push(PortInfo {
                                    name: format!("output{}", v.outputs.len()),
                                    datatypes: vec![String::from("string")],
                                });
                            }
                            win.unsaved_edits += 1;
                        }
                        Self::propagate_function_ports(&mut self.windows, win_id);
                    }
                    FunctionEditMessage::DeleteInput(idx) => {
                        if let Some(win) = self.windows.get_mut(&win_id) {
                            if let WindowKind::FunctionViewer(ref mut v) = win.kind {
                                if idx < v.inputs.len() {
                                    v.inputs.remove(idx);
                                }
                            }
                            win.unsaved_edits += 1;
                        }
                        Self::propagate_function_ports(&mut self.windows, win_id);
                    }
                    FunctionEditMessage::DeleteOutput(idx) => {
                        if let Some(win) = self.windows.get_mut(&win_id) {
                            if let WindowKind::FunctionViewer(ref mut v) = win.kind {
                                if idx < v.outputs.len() {
                                    v.outputs.remove(idx);
                                }
                            }
                            win.unsaved_edits += 1;
                        }
                        Self::propagate_function_ports(&mut self.windows, win_id);
                    }
                    FunctionEditMessage::InputNameChanged(idx, name) => {
                        if let Some(win) = self.windows.get_mut(&win_id) {
                            if let WindowKind::FunctionViewer(ref mut v) = win.kind {
                                if let Some(port) = v.inputs.get_mut(idx) {
                                    port.name = name;
                                }
                            }
                            win.unsaved_edits += 1;
                        }
                        Self::propagate_function_ports(&mut self.windows, win_id);
                    }
                    FunctionEditMessage::InputTypeChanged(idx, dtype) => {
                        if let Some(win) = self.windows.get_mut(&win_id) {
                            if let WindowKind::FunctionViewer(ref mut v) = win.kind {
                                if let Some(port) = v.inputs.get_mut(idx) {
                                    port.datatypes = vec![dtype];
                                }
                            }
                            win.unsaved_edits += 1;
                        }
                        Self::propagate_function_ports(&mut self.windows, win_id);
                    }
                    FunctionEditMessage::OutputNameChanged(idx, name) => {
                        if let Some(win) = self.windows.get_mut(&win_id) {
                            if let WindowKind::FunctionViewer(ref mut v) = win.kind {
                                if let Some(port) = v.outputs.get_mut(idx) {
                                    port.name = name;
                                }
                            }
                            win.unsaved_edits += 1;
                        }
                        Self::propagate_function_ports(&mut self.windows, win_id);
                    }
                    FunctionEditMessage::OutputTypeChanged(idx, dtype) => {
                        if let Some(win) = self.windows.get_mut(&win_id) {
                            if let WindowKind::FunctionViewer(ref mut v) = win.kind {
                                if let Some(port) = v.outputs.get_mut(idx) {
                                    port.datatypes = vec![dtype];
                                }
                            }
                            win.unsaved_edits += 1;
                        }
                        Self::propagate_function_ports(&mut self.windows, win_id);
                    }
                    FunctionEditMessage::Save => {
                        if let Some(win) = self.windows.get_mut(&win_id) {
                            if let WindowKind::FunctionViewer(ref v) = win.kind {
                                match flow_io::save_function_definition(v) {
                                    Ok(()) => {
                                        win.status = format!("Saved: {}", v.toml_path.display());
                                        win.unsaved_edits = 0;
                                    }
                                    Err(e) => {
                                        win.status = format!("Save failed: {e}");
                                    }
                                }
                            }
                        }
                    }
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
            Message::CloseRequested(_) | Message::CloseActiveWindow => {
                let target = match message {
                    Message::CloseRequested(win_id) => Some(win_id),
                    Message::CloseActiveWindow => self.focused_window.or(self.root_window),
                    _ => None,
                };
                let Some(id) = target else {
                    return Task::none();
                };
                if let Some(win) = self.windows.get(&id) {
                    if win.unsaved_edits > 0 {
                        let dialog = rfd::MessageDialog::new()
                            .set_title("Unsaved Changes")
                            .set_description(
                                "This window has unsaved changes. Close without saving?",
                            )
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
                return window::close(id);
            }
            Message::QuitAll => {
                // Check for unsaved edits in any window
                let has_unsaved = self.windows.values().any(|w| w.unsaved_edits > 0);
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
                return iced::exit();
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
            return Self::view_function(window_id, viewer, &win.status, win.unsaved_edits);
        }

        let canvas_with_controls = canvas_view::view_canvas_area(win, window_id);

        let hierarchy_panel = win
            .flow_hierarchy
            .view()
            .map(move |msg| Message::Hierarchy(window_id, msg));

        let library_panel = self
            .library_tree
            .view()
            .map(move |msg| Message::Library(window_id, msg));

        let left_panel = Column::new()
            .push(hierarchy_panel)
            .push(library_panel)
            .height(Fill);

        let mut right_col: Column<'_, Message> =
            Column::new().push(container(canvas_with_controls).width(Fill).height(Fill));

        // Flow I/O editor panel for sub-flow windows
        if !win.is_root && matches!(win.kind, WindowKind::FlowEditor) {
            right_col = right_col.push(Self::view_flow_io_panel(window_id, win));
        }

        // Metadata editor panel (toggled by Info button)
        if win.show_metadata && matches!(win.kind, WindowKind::FlowEditor) {
            right_col = right_col.push(Self::view_metadata_panel(win, window_id));
        }

        // Library paths panel (toggled by Libs button)
        if self.show_lib_paths {
            right_col = right_col.push(self.view_lib_paths_panel());
        }

        right_col = right_col.push(self.view_toolbar(win, window_id));

        let layout = Row::new().push(left_panel).push(right_col.width(Fill));
        layout.into()
    }

    /// Build the toolbar/status bar with action buttons and status text.
    fn view_toolbar<'a>(
        &'a self,
        win: &'a WindowState,
        window_id: window::Id,
    ) -> Element<'a, Message> {
        let edit_indicator = if win.unsaved_edits > 0 {
            format!("  |  {} unsaved edit(s)", win.unsaved_edits)
        } else {
            String::from("  |  saved")
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
            if !win.nodes.is_empty() {
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
                .push(Text::new(format!("{}{}", win.status, edit_indicator)).size(14))
                .push(iced::widget::Space::new().width(Fill))
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
            Row::new()
                .spacing(8)
                .padding([4, 8])
                .push(Text::new(format!("{}{}", win.status, edit_indicator)).size(14))
                .push(iced::widget::Space::new().width(Fill))
        };

        container(status_bar).width(Fill).padding(5).into()
    }

    /// Build the metadata editor panel.
    fn view_metadata_panel(win: &WindowState, window_id: window::Id) -> Element<'_, Message> {
        let authors_str = win.flow_definition.metadata.authors.join(", ");
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
                            text_input("Flow name", &win.flow_definition.name)
                                .on_input(move |s| {
                                    Message::FlowEdit(window_id, FlowEditMessage::NameChanged(s))
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
                            text_input("0.1.0", &win.flow_definition.metadata.version)
                                .on_input(move |s| {
                                    Message::FlowEdit(window_id, FlowEditMessage::VersionChanged(s))
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
                            text_input(
                                "A short description",
                                &win.flow_definition.metadata.description,
                            )
                            .on_input(move |s| {
                                Message::FlowEdit(window_id, FlowEditMessage::DescriptionChanged(s))
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
                                    Message::FlowEdit(window_id, FlowEditMessage::AuthorsChanged(s))
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

    fn view_flow_io_panel(window_id: window::Id, win: &WindowState) -> Element<'_, Message> {
        let input_color = Color::from_rgb(0.4, 0.8, 1.0);
        let output_color = Color::from_rgb(1.0, 0.6, 0.3);

        let mut input_col = Column::new().spacing(4);
        for (i, port) in win.flow_definition.inputs.iter().enumerate() {
            let port_name = port.name().clone();
            let dtype = port
                .datatypes()
                .first()
                .map(ToString::to_string)
                .unwrap_or_default();
            let row = Row::new()
                .spacing(4)
                .align_y(iced::Alignment::Center)
                .push(Text::new("\u{25D7}").size(18).color(input_color))
                .push(
                    text_input("name", &port_name)
                        .on_input(move |s| {
                            Message::FlowEdit(window_id, FlowEditMessage::InputNameChanged(i, s))
                        })
                        .size(12)
                        .padding(3)
                        .width(80),
                )
                .push(
                    text_input("type", &dtype)
                        .on_input(move |s| {
                            Message::FlowEdit(window_id, FlowEditMessage::InputTypeChanged(i, s))
                        })
                        .size(11)
                        .padding(3)
                        .width(70),
                )
                .push(
                    button(Text::new("\u{2715}").size(10).center())
                        .on_press(Message::FlowEdit(
                            window_id,
                            FlowEditMessage::DeleteInput(i),
                        ))
                        .style(button::danger)
                        .padding([2, 5]),
                );
            input_col = input_col.push(row);
        }
        input_col = input_col.push(
            button(Text::new("+ Input").size(11).center())
                .on_press(Message::FlowEdit(window_id, FlowEditMessage::AddInput))
                .style(button::secondary)
                .padding([2, 8]),
        );

        let mut output_col = Column::new().spacing(4).align_x(iced::Alignment::End);
        for (i, port) in win.flow_definition.outputs.iter().enumerate() {
            let port_name = port.name().clone();
            let dtype = port
                .datatypes()
                .first()
                .map(ToString::to_string)
                .unwrap_or_default();
            let row = Row::new()
                .spacing(4)
                .align_y(iced::Alignment::Center)
                .push(
                    button(Text::new("\u{2715}").size(10).center())
                        .on_press(Message::FlowEdit(
                            window_id,
                            FlowEditMessage::DeleteOutput(i),
                        ))
                        .style(button::danger)
                        .padding([2, 5]),
                )
                .push(
                    text_input("type", &dtype)
                        .on_input(move |s| {
                            Message::FlowEdit(window_id, FlowEditMessage::OutputTypeChanged(i, s))
                        })
                        .size(11)
                        .padding(3)
                        .width(70),
                )
                .push(
                    text_input("name", &port_name)
                        .on_input(move |s| {
                            Message::FlowEdit(window_id, FlowEditMessage::OutputNameChanged(i, s))
                        })
                        .size(12)
                        .padding(3)
                        .width(80),
                )
                .push(Text::new("\u{25D6}").size(18).color(output_color));
            output_col = output_col.push(row);
        }
        output_col = output_col.push(
            button(Text::new("+ Output").size(11).center())
                .on_press(Message::FlowEdit(window_id, FlowEditMessage::AddOutput))
                .style(button::secondary)
                .padding([2, 8]),
        );

        let io_box = container(
            Column::new()
                .spacing(12)
                .padding(iced::Padding {
                    top: 8.0,
                    bottom: 8.0,
                    left: 0.0,
                    right: 0.0,
                })
                .push(
                    Row::new()
                        .push(input_col)
                        .push(iced::widget::Space::new().width(Fill))
                        .push(output_col),
                ),
        )
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.18, 0.18, 0.22))),
            border: iced::Border {
                color: Color::from_rgb(0.9, 0.6, 0.2),
                width: 2.0,
                radius: 12.0.into(),
            },
            ..Default::default()
        })
        .width(Fill)
        .padding([0, 8]);

        container(io_box).padding([6, 12]).width(Fill).into()
    }

    fn view_function<'a>(
        window_id: window::Id,
        viewer: &'a FunctionViewer,
        status: &'a str,
        unsaved_edits: i32,
    ) -> Element<'a, Message> {
        let content: Element<'_, Message> = match viewer.active_tab {
            0 => {
                let input_color = Color::from_rgb(0.4, 0.8, 1.0);
                let output_color = Color::from_rgb(1.0, 0.6, 0.3);

                // Input ports inside box: semicircle, name, type, (delete if editable)
                let editable = !viewer.read_only;
                let mut input_col = Column::new().spacing(6);
                for (i, port) in viewer.inputs.iter().enumerate() {
                    let dtype = port.datatypes.first().cloned().unwrap_or_default();
                    let mut name_widget =
                        text_input("name", &port.name).size(13).padding(3).width(90);
                    let mut type_widget = text_input("type", &dtype).size(11).padding(3).width(75);
                    if editable {
                        name_widget = name_widget.on_input(move |s| {
                            Message::FunctionEdit(
                                window_id,
                                FunctionEditMessage::InputNameChanged(i, s),
                            )
                        });
                        type_widget = type_widget.on_input(move |s| {
                            Message::FunctionEdit(
                                window_id,
                                FunctionEditMessage::InputTypeChanged(i, s),
                            )
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

                // Output ports inside box: (delete if editable), type, name, semicircle
                let mut output_col = Column::new().spacing(6).align_x(iced::Alignment::End);
                for (i, port) in viewer.outputs.iter().enumerate() {
                    let dtype = port.datatypes.first().cloned().unwrap_or_default();
                    let mut type_widget = text_input("type", &dtype).size(11).padding(3).width(75);
                    let mut name_widget =
                        text_input("name", &port.name).size(13).padding(3).width(90);
                    if editable {
                        type_widget = type_widget.on_input(move |s| {
                            Message::FunctionEdit(
                                window_id,
                                FunctionEditMessage::OutputTypeChanged(i, s),
                            )
                        });
                        name_widget = name_widget.on_input(move |s| {
                            Message::FunctionEdit(
                                window_id,
                                FunctionEditMessage::OutputNameChanged(i, s),
                            )
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

                let mut name_widget = text_input("Function name", &viewer.name)
                    .size(16)
                    .padding(6)
                    .width(250);
                if editable {
                    name_widget = name_widget.on_input(move |s| {
                        Message::FunctionEdit(window_id, FunctionEditMessage::NameChanged(s))
                    });
                }
                let name_input = container(name_widget).center_x(Fill);

                let mut desc_widget = text_input("Description", &viewer.description)
                    .size(13)
                    .padding(6)
                    .width(480);
                if editable {
                    let ext = std::path::Path::new(&viewer.source_file)
                        .extension()
                        .unwrap_or_default();
                    let is_provided =
                        ext.eq_ignore_ascii_case("rs") || ext.eq_ignore_ascii_case("wasm");
                    if is_provided {
                        desc_widget = desc_widget.on_input(move |s| {
                            Message::FunctionEdit(
                                window_id,
                                FunctionEditMessage::DescriptionChanged(s),
                            )
                        });
                    }
                }
                let desc_input = container(desc_widget).center_x(Fill);

                let mut source_row = Row::new().spacing(6).align_y(iced::Alignment::Center).push(
                    button(
                        Text::new(&viewer.source_file)
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
            1 => {
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
                                Text::new(&viewer.rs_content)
                                    .size(14)
                                    .font(iced::Font::MONOSPACE),
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
            _ => {
                let back_btn = button(Text::new("\u{2190} Definition").size(13).center())
                    .on_press(Message::FunctionEdit(
                        window_id,
                        FunctionEditMessage::TabSelected(0),
                    ))
                    .style(button::secondary)
                    .padding([6, 14]);
                let docs = viewer.docs_content.as_deref().unwrap_or("");
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
        };

        let mut save_btn = button(Text::new("\u{1F4BE} Save").size(14).center())
            .style(if unsaved_edits > 0 && !viewer.read_only {
                button::primary
            } else {
                button::secondary
            })
            .padding([6, 14]);
        if unsaved_edits > 0 && !viewer.read_only {
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

        let provider = flow_io::build_meta_provider();
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
            if win.file_path().as_ref() == Some(&path) {
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
                self.open_function_viewer(parent, &path, func, &path.to_string_lossy())
            }
            Ok(Process::FlowProcess(_)) => match flow_io::load_flow(&path) {
                Ok(loaded) => {
                    let has_nodes = !loaded.nodes.is_empty();
                    let nc = loaded.nodes.len();
                    let ec = loaded.edges.len();
                    let (new_id, open_task) =
                        window::open(self.child_window_settings(1024.0, 768.0));
                    let mut flow_def = loaded.flow_def;
                    if let Ok(url) = Url::from_file_path(&path) {
                        flow_def.source_url = url;
                    }
                    let child = WindowState {
                        kind: WindowKind::FlowEditor,
                        nodes: loaded.nodes,
                        edges: loaded.edges,
                        canvas_state: FlowCanvasState::default(),
                        status: format!("Library flow - {nc} nodes, {ec} connections"),
                        selected_node: None,
                        selected_connection: None,
                        history: EditHistory::default(),
                        auto_fit_pending: has_nodes,
                        auto_fit_enabled: true,
                        unsaved_edits: 0,
                        compiled_manifest: None,
                        flow_definition: flow_def,
                        tooltip: None,
                        initializer_editor: None,
                        is_root: false,
                        context_menu: None,
                        show_metadata: false,
                        flow_hierarchy: self.build_hierarchy(),
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
        // Gather lib_references from the root window's flow.
        let lib_refs = self
            .root_window
            .and_then(|id| self.windows.get(&id))
            .map(|win| win.flow_definition.lib_references.clone())
            .unwrap_or_default();
        let (lc, ad) = library_mgmt::load_library_catalogs(&lib_refs);
        self.library_cache = lc;
        self.all_definitions = ad;
        self.library_tree = LibraryTree::from_cache(&self.library_cache, &self.all_definitions);
    }

    fn root_flow_path(&self) -> Option<PathBuf> {
        self.root_window
            .and_then(|id| self.windows.get(&id))
            .and_then(WindowState::file_path)
    }

    fn build_hierarchy(&self) -> FlowHierarchy {
        self.root_flow_path()
            .as_ref()
            .map_or_else(FlowHierarchy::empty, |p| FlowHierarchy::build(p))
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
        // Extract source and resolved path from the parent window (immutable borrow)
        let (source, resolved_path) = {
            let Some(win) = self.windows.get(&parent_win_id) else {
                return Task::none();
            };
            let Some(node) = win.nodes.get(idx) else {
                return Task::none();
            };
            let source = node.source.clone();
            let path = library_mgmt::resolve_node_source(win, &source);
            (source, path)
        };

        let Some(path) = resolved_path else {
            if let Some(win) = self.windows.get_mut(&parent_win_id) {
                win.status = format!("Could not resolve source: {source}");
            }
            return Task::none();
        };

        // If a window already has this file open, focus it instead of opening a duplicate
        for (&win_id, win) in &self.windows {
            if win.file_path().as_ref() == Some(&path) && win_id != parent_win_id {
                return window::gain_focus(win_id);
            }
        }

        // Check whether the resolved file is a flow or a function
        let abs_path = match std::fs::canonicalize(&path) {
            Ok(p) => p,
            Err(_) => path.clone(),
        };
        if let Ok(contents) = std::fs::read_to_string(&abs_path) {
            if let Ok(url) = Url::from_file_path(&abs_path) {
                if let Ok(deserializer) = get::<Process>(&url) {
                    if let Ok(Process::FunctionProcess(ref func)) =
                        deserializer.deserialize(&contents, Some(&url))
                    {
                        return self.open_function_viewer(parent_win_id, &path, func, &source);
                    }
                }
            }
        }

        // Load the sub-flow and open it in a new window
        match flow_io::load_flow(&path) {
            Ok(loaded) => {
                let has_nodes = !loaded.nodes.is_empty();
                let (new_id, open_task) = window::open(self.child_window_settings(1024.0, 768.0));
                let nc = loaded.nodes.len();
                let ec = loaded.edges.len();
                let mut flow_def = loaded.flow_def;
                if let Ok(url) = Url::from_file_path(&path) {
                    flow_def.source_url = url;
                }
                let child = WindowState {
                    kind: WindowKind::FlowEditor,
                    nodes: loaded.nodes,
                    edges: loaded.edges,
                    canvas_state: FlowCanvasState::default(),
                    status: format!("Ready - {nc} nodes, {ec} connections"),
                    selected_node: None,
                    selected_connection: None,
                    history: EditHistory::default(),
                    auto_fit_pending: has_nodes,
                    auto_fit_enabled: true,
                    unsaved_edits: 0,
                    compiled_manifest: None,
                    flow_definition: flow_def,
                    tooltip: None,
                    initializer_editor: None,
                    is_root: false,
                    context_menu: None,
                    show_metadata: false,
                    flow_hierarchy: self.build_hierarchy(),
                    last_size: None,
                    last_position: None,
                };
                self.windows.insert(new_id, child);
                if let Some(win) = self.windows.get_mut(&parent_win_id) {
                    win.status = format!("Opened: {}", path.display());
                }
                open_task.discard()
            }
            Err(e) => {
                if let Some(win) = self.windows.get_mut(&parent_win_id) {
                    win.status = format!("Could not open '{source}': {e}");
                }
                Task::none()
            }
        }
    }

    fn open_function_viewer(
        &mut self,
        parent_win_id: window::Id,
        toml_path: &Path,
        func: &flowcore::model::function_definition::FunctionDefinition,
        node_source: &str,
    ) -> Task<Message> {
        let dir = toml_path.parent().unwrap_or(Path::new("."));
        let func_name = &func.name;

        let rs_path = dir.join(&func.source);
        let rs_content = std::fs::read_to_string(&rs_path)
            .unwrap_or_else(|_| String::from("// Source file not found"));
        let docs_content = std::fs::read_to_string(dir.join(format!("{func_name}.md"))).ok();

        let (inputs, outputs) = flow_io::extract_ports(&func.inputs, &func.outputs);

        let (new_id, open_task) = window::open(self.child_window_settings(700.0, 500.0));

        let read_only = node_source.starts_with("lib://") || node_source.starts_with("context://");
        let viewer = FunctionViewer {
            name: func_name.clone(),
            description: func.description.clone(),
            source_file: func.source.clone(),
            inputs,
            outputs,
            rs_content,
            docs_content,
            active_tab: 0,
            toml_path: toml_path.to_path_buf(),
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
            kind: WindowKind::FunctionViewer(viewer),
            nodes: Vec::new(),
            edges: Vec::new(),
            canvas_state: FlowCanvasState::default(),
            status: format!("Function: {func_name}"),
            selected_node: None,
            selected_connection: None,
            history: EditHistory::default(),
            auto_fit_pending: false,
            auto_fit_enabled: false,
            unsaved_edits: 0,
            compiled_manifest: None,
            flow_definition: func_flow_def,
            tooltip: None,
            initializer_editor: None,
            is_root: false,
            context_menu: None,
            show_metadata: false,
            flow_hierarchy: self.build_hierarchy(),
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
        let base_dir = self
            .windows
            .get(&target_id)
            .and_then(WindowState::file_path)
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

        // Add a process reference in the target flow
        if let Some(win) = self.windows.get_mut(&target_id) {
            let alias = flow_io::generate_unique_alias(&flow_name, &win.nodes);
            let (x, y) = flow_io::next_node_position(&win.nodes);

            let node = NodeLayout {
                alias: alias.clone(),
                source: source.clone(),
                description: String::new(),
                x,
                y,
                width: 180.0,
                height: 120.0,
                inputs: Vec::new(),
                outputs: Vec::new(),
                initializers: HashMap::new(),
            };
            win.nodes.push(node);
            win.flow_definition.process_refs.push(ProcessReference {
                alias: alias.clone(),
                source,
                initializations: std::collections::BTreeMap::new(),
                x: Some(x),
                y: Some(y),
                width: Some(180.0),
                height: Some(120.0),
            });
            win.unsaved_edits += 1;
            win.canvas_state.request_redraw();
            win.status = format!("Created sub-flow: {alias}");
        }

        // Open the new sub-flow in a child window
        let (new_id, open_task) = window::open(self.child_window_settings(1024.0, 768.0));

        let child = WindowState {
            kind: WindowKind::FlowEditor,
            nodes: Vec::new(),
            edges: Vec::new(),
            canvas_state: FlowCanvasState::default(),
            status: format!("New sub-flow: {flow_name}"),
            selected_node: None,
            selected_connection: None,
            history: EditHistory::default(),
            auto_fit_pending: false,
            auto_fit_enabled: true,
            unsaved_edits: 0,
            compiled_manifest: None,
            flow_definition: flow_def,
            tooltip: None,
            initializer_editor: None,
            is_root: false,
            context_menu: None,
            show_metadata: false,
            flow_hierarchy: self.build_hierarchy(),
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

        let base_dir = self
            .windows
            .get(&target_id)
            .and_then(WindowState::file_path)
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

        // Add process reference in the target flow
        if let Some(win) = self.windows.get_mut(&target_id) {
            let alias = flow_io::generate_unique_alias(&func_name, &win.nodes);
            let (x, y) = flow_io::next_node_position(&win.nodes);

            let node = NodeLayout {
                alias: alias.clone(),
                source: source.clone(),
                description: String::new(),
                x,
                y,
                width: 180.0,
                height: 120.0,
                inputs: Vec::new(),
                outputs: Vec::new(),
                initializers: HashMap::new(),
            };
            win.nodes.push(node);
            win.flow_definition.process_refs.push(ProcessReference {
                alias: alias.clone(),
                source,
                initializations: std::collections::BTreeMap::new(),
                x: Some(x),
                y: Some(y),
                width: Some(180.0),
                height: Some(120.0),
            });
            win.unsaved_edits += 1;
            win.canvas_state.request_redraw();
            win.status = format!("Created function: {alias}");
        }

        // Open the function viewer window
        let (new_id, open_task) = window::open(self.child_window_settings(700.0, 500.0));

        let viewer = FunctionViewer {
            name: func_name.clone(),
            description: String::new(),
            source_file: rs_filename.clone(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            rs_content: String::from("// Save to generate skeleton source"),
            docs_content: None,
            active_tab: 0,
            toml_path: path.clone(),
            parent_window: Some(target_id),
            node_source: rs_filename,
            read_only: false,
        };

        let mut func_flow_def = FlowDefinition {
            name: func_name,
            ..FlowDefinition::default()
        };
        if let Ok(url) = Url::from_file_path(&path) {
            func_flow_def.source_url = url;
        }
        let child = WindowState {
            kind: WindowKind::FunctionViewer(viewer),
            nodes: Vec::new(),
            edges: Vec::new(),
            canvas_state: FlowCanvasState::default(),
            status: String::from("New function — add ports and Save"),
            selected_node: None,
            selected_connection: None,
            history: EditHistory::default(),
            auto_fit_pending: false,
            auto_fit_enabled: false,
            unsaved_edits: 1,
            compiled_manifest: None,
            flow_definition: func_flow_def,
            tooltip: None,
            initializer_editor: None,
            is_root: false,
            context_menu: None,
            show_metadata: false,
            flow_hierarchy: self.build_hierarchy(),
            last_size: None,
            last_position: None,
        };

        self.windows.insert(new_id, child);
        open_task.discard()
    }

    /// Propagate the current function viewer's ports (inputs/outputs) to matching
    /// nodes in the parent canvas window. This keeps the parent canvas node's port
    /// display in sync when ports are added, deleted, or renamed in the function viewer.
    fn propagate_function_ports(
        windows: &mut HashMap<window::Id, WindowState>,
        viewer_win_id: window::Id,
    ) {
        // Extract parent info and current ports from the viewer window
        let propagation_data = windows.get(&viewer_win_id).and_then(|win| {
            if let WindowKind::FunctionViewer(ref viewer) = win.kind {
                viewer.parent_window.map(|pid| {
                    (
                        pid,
                        viewer.node_source.clone(),
                        viewer.inputs.clone(),
                        viewer.outputs.clone(),
                    )
                })
            } else {
                None
            }
        });

        if let Some((parent_id, node_source, new_inputs, new_outputs)) = propagation_data {
            if let Some(parent_win) = windows.get_mut(&parent_id) {
                for node in &mut parent_win.nodes {
                    if node.source == node_source {
                        node.inputs.clone_from(&new_inputs);
                        node.outputs.clone_from(&new_outputs);
                    }
                }
                parent_win.canvas_state.request_redraw();
            }
        }
    }
}
