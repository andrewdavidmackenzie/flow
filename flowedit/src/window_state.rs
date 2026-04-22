//! Per-window state and related types for the flow editor.

use std::path::{Path, PathBuf};

use iced::window;
use url::Url;

use flowcore::model::flow_definition::FlowDefinition;

use crate::canvas_view::{FlowCanvasState, NodeLayout, PortInfo};
use crate::hierarchy_panel::FlowHierarchy;
use crate::history;
use crate::history::EditHistory;

/// State for the initializer editing dialog.
pub(crate) struct InitializerEditor {
    /// Index of the node being edited
    pub(crate) node_index: usize,
    /// Name of the input port being edited
    pub(crate) port_name: String,
    /// Selected type: "none", "once", or "always"
    pub(crate) init_type: String,
    /// The value as a string (JSON)
    pub(crate) value_text: String,
}

/// State for a function definition viewer/editor window.
pub(crate) struct FunctionViewer {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) source_file: String,
    pub(crate) inputs: Vec<PortInfo>,
    pub(crate) outputs: Vec<PortInfo>,
    pub(crate) rs_content: String,
    pub(crate) docs_content: Option<String>,
    pub(crate) active_tab: usize,
    pub(crate) toml_path: PathBuf,
    /// Parent window that opened this viewer (for propagating edits back to canvas)
    pub(crate) parent_window: Option<window::Id>,
    /// Source string of the node this viewer is editing (to find the `NodeLayout`)
    pub(crate) node_source: String,
    /// Whether this viewer is read-only (library/context functions cannot be edited)
    pub(crate) read_only: bool,
}

/// What kind of content a window displays.
pub(crate) enum WindowKind {
    FlowEditor,
    FunctionViewer(FunctionViewer),
}

/// Per-window state for the flow editor.
pub(crate) struct WindowState {
    /// What this window displays
    pub(crate) kind: WindowKind,
    /// Positioned nodes derived from the flow's process references
    pub(crate) nodes: Vec<NodeLayout>,
    /// Canvas state for caching rendered geometry
    pub(crate) canvas_state: FlowCanvasState,
    /// Status message displayed in the bottom bar
    pub(crate) status: String,
    /// Index of the currently selected node, if any
    pub(crate) selected_node: Option<usize>,
    /// Index of the currently selected connection, if any
    pub(crate) selected_connection: Option<usize>,
    /// Edit history for undo/redo
    pub(crate) history: EditHistory,
    /// Whether auto-fit should be performed on the next opportunity
    pub(crate) auto_fit_pending: bool,
    /// Whether auto-fit mode is active (continuously fits to window)
    pub(crate) auto_fit_enabled: bool,
    /// Count of unsaved edits (increments on edit/redo, decrements on undo)
    pub(crate) unsaved_edits: i32,
    /// Path to the last compiled manifest (None if not compiled or edited since)
    pub(crate) compiled_manifest: Option<PathBuf>,
    /// The original flow definition, used to preserve metadata when saving
    pub(crate) flow_definition: FlowDefinition,
    /// Tooltip text and screen position to display (full source path on hover)
    pub(crate) tooltip: Option<(String, f32, f32)>,
    /// Active initializer editor dialog, if any
    pub(crate) initializer_editor: Option<InitializerEditor>,
    /// Whether this is the root (main) window
    pub(crate) is_root: bool,
    /// Context menu position (screen coords), if showing
    pub(crate) context_menu: Option<(f32, f32)>,
    /// Whether the metadata editor is visible
    pub(crate) show_metadata: bool,
    /// Flow hierarchy tree for this window's navigation panel
    pub(crate) flow_hierarchy: FlowHierarchy,
    /// Last known window size (tracked via resize events)
    pub(crate) last_size: Option<iced::Size>,
    /// Last known window position (tracked via move events)
    pub(crate) last_position: Option<iced::Point>,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            kind: WindowKind::FlowEditor,
            nodes: Vec::new(),
            canvas_state: FlowCanvasState::default(),
            status: String::new(),
            selected_node: None,
            selected_connection: None,
            history: EditHistory::default(),
            auto_fit_pending: false,
            auto_fit_enabled: false,
            unsaved_edits: 0,
            compiled_manifest: None,
            flow_definition: FlowDefinition::default(),
            tooltip: None,
            initializer_editor: None,
            is_root: false,
            context_menu: None,
            show_metadata: false,
            flow_hierarchy: FlowHierarchy::empty(),
            last_size: None,
            last_position: None,
        }
    }
}

impl WindowState {
    /// Get the file path from the flow definition's source URL.
    /// Returns `None` if no file has been saved/loaded yet.
    pub(crate) fn file_path(&self) -> Option<PathBuf> {
        self.flow_definition.source_url.to_file_path().ok()
    }

    /// Set the file path by updating the flow definition's source URL.
    pub(crate) fn set_file_path(&mut self, path: &Path) {
        let abs = path.canonicalize().unwrap_or_else(|_| {
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                std::env::current_dir().map_or_else(|_| path.to_path_buf(), |cwd| cwd.join(path))
            }
        });
        if let Ok(url) = Url::from_file_path(&abs) {
            self.flow_definition.source_url = url;
        }
    }

    /// Clear the file path by resetting the source URL to the default.
    pub(crate) fn clear_file_path(&mut self) {
        self.flow_definition.source_url = FlowDefinition::default_url();
    }

    /// Undo the last edit action.
    pub(crate) fn handle_undo(&mut self) {
        history::handle_undo(self);
    }

    /// Redo the last undone action.
    pub(crate) fn handle_redo(&mut self) {
        history::handle_redo(self);
    }
}
