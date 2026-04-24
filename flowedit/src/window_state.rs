//! Per-window state and related types for the flow editor.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::path::{Path, PathBuf};

use iced::widget::{button, container, pick_list, stack, text_input, Column, Row, Text};
use iced::{window, Color, Element, Fill, Theme};
use log::info;
use url::Url;

use flowcore::model::connection::Connection;
use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::function_definition::FunctionDefinition;
use flowcore::model::input::InputInitializer;

use crate::file_ops;
use crate::flow_canvas::{
    build_render_nodes, connection_references_node, derive_short_name, split_route, CanvasAction,
    CanvasMessage, FlowCanvasState,
};
use crate::hierarchy_panel::FlowHierarchy;
use crate::history::{EditAction, EditHistory};
use crate::{Message, ViewMessage};

/// Tooltip text and screen position for hover display.
#[derive(Debug, Clone)]
pub(crate) struct Tooltip {
    pub(crate) text: String,
    pub(crate) x: f32,
    pub(crate) y: f32,
}

/// Screen position for a right-click context menu.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MenuPosition {
    pub(crate) x: f32,
    pub(crate) y: f32,
}

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
    /// The canonical function definition (owns name, description, source, inputs, outputs, `source_url`)
    pub(crate) func_def: FunctionDefinition,
    pub(crate) rs_content: String,
    pub(crate) docs_content: Option<String>,
    pub(crate) active_tab: usize,
    /// Parent window that opened this viewer (for propagating edits back to canvas)
    pub(crate) parent_window: Option<window::Id>,
    /// Source string of the node this viewer is editing (to find the `NodeLayout`)
    pub(crate) node_source: String,
    /// Whether this viewer is read-only (library/context functions cannot be edited)
    pub(crate) read_only: bool,
}

impl FunctionViewer {
    /// Derive the TOML file path from the function definition's source URL.
    pub(crate) fn toml_path(&self) -> Option<PathBuf> {
        self.func_def.get_source_url().to_file_path().ok()
    }
}

/// What kind of content a window displays.
pub(crate) enum WindowKind {
    FlowEditor,
    FunctionViewer(Box<FunctionViewer>),
}

/// Per-window state for the flow editor.
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct WindowState {
    /// What this window displays
    pub(crate) kind: WindowKind,
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
    /// The original flow definition, used to preserve metadata when saving
    pub(crate) flow_definition: FlowDefinition,
    /// Tooltip text and screen position to display (full source path on hover)
    pub(crate) tooltip: Option<Tooltip>,
    /// Active initializer editor dialog, if any
    pub(crate) initializer_editor: Option<InitializerEditor>,
    /// Whether this is the root (main) window
    pub(crate) is_root: bool,
    /// Context menu position (screen coords), if showing
    pub(crate) context_menu: Option<MenuPosition>,
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
            canvas_state: FlowCanvasState::default(),
            status: String::new(),
            selected_node: None,
            selected_connection: None,
            history: EditHistory::default(),
            auto_fit_pending: false,
            auto_fit_enabled: false,
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
}
impl WindowState {
    /// Handle a [`CanvasMessage`] by mutating canvas/selection state.
    ///
    /// Returns a [`CanvasAction`] when the caller needs to perform cross-window
    /// operations (e.g. opening a sub-flow in a new editor window).
    pub(crate) fn handle_canvas_message(&mut self, msg: CanvasMessage) -> CanvasAction {
        match msg {
            CanvasMessage::Selected(idx) => self.handle_selected(idx),
            CanvasMessage::Moved(idx, x, y) => {
                if let Some(pref) = self.flow_definition.process_refs.get_mut(idx) {
                    pref.x = Some(x);
                    pref.y = Some(y);
                    self.canvas_state.request_redraw();
                }
            }
            CanvasMessage::Resized(idx, x, y, w, h) => {
                if let Some(pref) = self.flow_definition.process_refs.get_mut(idx) {
                    pref.x = Some(x);
                    pref.y = Some(y);
                    pref.width = Some(w);
                    pref.height = Some(h);
                    self.canvas_state.request_redraw();
                }
            }
            CanvasMessage::MoveCompleted(idx, old_x, old_y, new_x, new_y) => {
                info!("MoveCompleted: idx={idx}, ({old_x},{old_y}) -> ({new_x},{new_y})");
                if (old_x - new_x).abs() > 0.5 || (old_y - new_y).abs() > 0.5 {
                    self.history.record(EditAction::MoveNode {
                        index: idx,
                        old_x,
                        old_y,
                        new_x,
                        new_y,
                    });
                    self.trigger_auto_fit_if_enabled();
                }
            }
            #[allow(clippy::similar_names)]
            CanvasMessage::ResizeCompleted(
                idx,
                old_x,
                old_y,
                old_w,
                old_h,
                new_x,
                new_y,
                new_w,
                new_h,
            ) => {
                self.handle_resize_completed(
                    idx, old_x, old_y, old_w, old_h, new_x, new_y, new_w, new_h,
                );
            }
            CanvasMessage::Deleted(idx) => self.handle_deleted(idx),
            CanvasMessage::ConnectionCreated {
                from_node,
                from_port,
                to_node,
                to_port,
            } => self.handle_connection_created(&from_node, &from_port, &to_node, &to_port),
            CanvasMessage::ConnectionSelected(idx) => self.handle_connection_selected(idx),
            CanvasMessage::ConnectionDeleted(idx) => self.handle_connection_deleted(idx),
            CanvasMessage::HoverChanged(data) => {
                self.tooltip = data;
            }
            CanvasMessage::AutoFitViewport(viewport) => {
                if self.auto_fit_enabled || self.auto_fit_pending {
                    let render_nodes = build_render_nodes(&self.flow_definition);
                    let is_subflow = !self.is_root;
                    self.canvas_state.auto_fit(
                        &render_nodes,
                        &self.flow_definition.inputs,
                        &self.flow_definition.outputs,
                        is_subflow,
                        viewport,
                    );
                    self.auto_fit_pending = false;
                }
            }
            CanvasMessage::Pan(dx, dy) => {
                self.auto_fit_enabled = false;
                self.auto_fit_pending = false;
                self.canvas_state.scroll_offset.x += dx;
                self.canvas_state.scroll_offset.y += dy;
                self.canvas_state.request_redraw();
            }
            CanvasMessage::ZoomBy(factor) => {
                self.auto_fit_enabled = false;
                self.auto_fit_pending = false;
                self.canvas_state.zoom = (self.canvas_state.zoom * factor).clamp(0.1, 5.0);
                self.canvas_state.request_redraw();
                let pct = (self.canvas_state.zoom * 100.0) as u32;
                self.status = format!("Zoom: {pct}%");
            }
            CanvasMessage::InitializerEdit(node_idx, port_name) => {
                self.handle_initializer_edit(node_idx, port_name);
            }
            CanvasMessage::OpenNode(idx) => {
                return CanvasAction::OpenNode(idx);
            }
            CanvasMessage::ContextMenu(x, y) => {
                self.context_menu = Some(crate::window_state::MenuPosition { x, y });
            }
        }
        CanvasAction::None
    }

    pub(crate) fn trigger_auto_fit_if_enabled(&mut self) {
        if self.auto_fit_enabled {
            self.auto_fit_pending = true;
            self.canvas_state.request_redraw();
        }
    }

    fn handle_selected(&mut self, idx: Option<usize>) {
        self.selected_node = idx;
        self.context_menu = None;
        if self.selected_connection.is_some() {
            self.selected_connection = None;
            self.canvas_state.request_redraw();
        }
        if let Some(i) = idx {
            if let Some(pref) = self.flow_definition.process_refs.get(i) {
                let alias = if pref.alias.is_empty() {
                    derive_short_name(&pref.source)
                } else {
                    pref.alias.clone()
                };
                self.status = format!("Selected: {alias}");
            }
        } else {
            self.status = String::from("Ready");
        }
    }

    #[allow(clippy::similar_names, clippy::too_many_arguments)]
    fn handle_resize_completed(
        &mut self,
        idx: usize,
        old_x: f32,
        old_y: f32,
        old_w: f32,
        old_h: f32,
        new_x: f32,
        new_y: f32,
        new_w: f32,
        new_h: f32,
    ) {
        if (old_x - new_x).abs() > 0.5
            || (old_y - new_y).abs() > 0.5
            || (old_w - new_w).abs() > 0.5
            || (old_h - new_h).abs() > 0.5
        {
            self.history.record(EditAction::ResizeNode {
                index: idx,
                old_x,
                old_y,
                old_w,
                old_h,
                new_x,
                new_y,
                new_w,
                new_h,
            });
            self.trigger_auto_fit_if_enabled();
        }
    }

    fn handle_deleted(&mut self, idx: usize) {
        if idx < self.flow_definition.process_refs.len() {
            let Some(pref) = self.flow_definition.process_refs.get(idx).cloned() else {
                return;
            };
            let alias = if pref.alias.is_empty() {
                derive_short_name(&pref.source)
            } else {
                pref.alias.clone()
            };
            let removed_connections: Vec<Connection> = self
                .flow_definition
                .connections
                .iter()
                .filter(|c| connection_references_node(c, &alias))
                .cloned()
                .collect();
            let removed_pref = self.flow_definition.process_refs.remove(idx);
            let removed_subprocess = self.flow_definition.subprocesses.remove(&alias);
            self.flow_definition
                .connections
                .retain(|c| !connection_references_node(c, &alias));
            self.history.record(EditAction::DeleteNode {
                index: idx,
                process_ref: removed_pref,
                subprocess: removed_subprocess.map(|p| (alias, p)),
                removed_connections,
            });
            self.selected_node = None;
            self.selected_connection = None;
            self.canvas_state.request_redraw();
            let nc = self.flow_definition.process_refs.len();
            let ec = self.flow_definition.connections.len();
            self.status = format!("Node deleted - {nc} nodes, {ec} connections");
            self.trigger_auto_fit_if_enabled();
        }
    }

    fn handle_connection_created(
        &mut self,
        from_node: &str,
        from_port: &str,
        to_node: &str,
        to_port: &str,
    ) {
        let from_route = if from_port.is_empty() {
            from_node.to_string()
        } else {
            format!("{from_node}/{from_port}")
        };
        let to_route = if to_port.is_empty() {
            to_node.to_string()
        } else {
            format!("{to_node}/{to_port}")
        };
        let connection = Connection::new(from_route, to_route);
        self.history.record(EditAction::CreateConnection {
            connection: connection.clone(),
        });
        self.flow_definition.connections.push(connection);
        self.canvas_state.request_redraw();
        let nc = self.flow_definition.process_refs.len();
        let ec = self.flow_definition.connections.len();
        self.status = format!(
            "Connection created: {from_node}/{from_port} -> {to_node}/{to_port} - {nc} nodes, {ec} connections"
        );
        self.trigger_auto_fit_if_enabled();
    }

    fn handle_connection_selected(&mut self, idx: Option<usize>) {
        self.context_menu = None;
        self.selected_connection = idx;
        self.selected_node = None;
        self.canvas_state.request_redraw();
        if let Some(i) = idx {
            if let Some(conn) = self.flow_definition.connections.get(i) {
                let (from_node, from_port) = split_route(conn.from().as_ref());
                let to_str = conn
                    .to()
                    .first()
                    .map_or_else(String::new, ToString::to_string);
                let (to_node, to_port) = split_route(&to_str);
                self.status = format!(
                    "Connection: {} -> {}",
                    file_ops::format_endpoint(&from_node, &from_port),
                    file_ops::format_endpoint(&to_node, &to_port),
                );
            }
        } else {
            self.status = String::from("Ready");
        }
    }

    fn handle_connection_deleted(&mut self, idx: usize) {
        if idx < self.flow_definition.connections.len() {
            let connection = self.flow_definition.connections.remove(idx);
            self.history.record(EditAction::DeleteConnection {
                index: idx,
                connection,
            });
            self.selected_connection = None;
            self.canvas_state.request_redraw();
            let nc = self.flow_definition.process_refs.len();
            let ec = self.flow_definition.connections.len();
            self.status = format!("Connection deleted - {nc} nodes, {ec} connections");
            self.trigger_auto_fit_if_enabled();
        }
    }

    fn handle_initializer_edit(&mut self, node_idx: usize, port_name: String) {
        self.context_menu = None;
        let (init_type, value_text) = self
            .flow_definition
            .process_refs
            .get(node_idx)
            .and_then(|pr| pr.initializations.get(&port_name))
            .map_or_else(
                || ("none".to_string(), String::new()),
                |init| match init {
                    InputInitializer::Once(v) => (
                        "once".to_string(),
                        serde_json::to_string(v).unwrap_or_default(),
                    ),
                    InputInitializer::Always(v) => (
                        "always".to_string(),
                        serde_json::to_string(v).unwrap_or_default(),
                    ),
                },
            );

        self.initializer_editor = Some(InitializerEditor {
            node_index: node_idx,
            port_name,
            init_type,
            value_text,
        });
    }

    pub(crate) fn handle_view_message(&mut self, msg: &ViewMessage) {
        match msg {
            ViewMessage::ZoomIn => {
                self.auto_fit_enabled = false;
                self.auto_fit_pending = false;
                self.canvas_state.zoom_in();
                let pct = (self.canvas_state.zoom * 100.0) as u32;
                self.status = format!("Zoom: {pct}%");
            }
            ViewMessage::ZoomOut => {
                self.auto_fit_enabled = false;
                self.auto_fit_pending = false;
                self.canvas_state.zoom_out();
                let pct = (self.canvas_state.zoom * 100.0) as u32;
                self.status = format!("Zoom: {pct}%");
            }
            ViewMessage::ToggleAutoFit => {
                self.auto_fit_enabled = !self.auto_fit_enabled;
                if self.auto_fit_enabled {
                    self.auto_fit_pending = true;
                    self.canvas_state.request_redraw();
                    self.status = String::from("Auto-fit enabled");
                } else {
                    self.status = String::from("Auto-fit disabled");
                }
            }
        }
    }
}
impl WindowState {
    pub(crate) fn view_canvas_area(&self, window_id: window::Id) -> Element<'_, Message> {
        let canvas = self
            .canvas_state
            .view(
                &self.flow_definition,
                &self.flow_definition.connections,
                &self.flow_definition.name,
                &self.flow_definition.inputs,
                &self.flow_definition.outputs,
                !self.is_root,
                self.auto_fit_pending,
                self.auto_fit_enabled,
            )
            .map(move |msg| Message::WindowCanvas(window_id, msg));

        let zoom_btn = |_theme: &Theme, status: button::Status| -> button::Style {
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
        let zoom_btn_active = |_theme: &Theme, status: button::Status| -> button::Style {
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
        let btn_width = 40;
        let zoom_controls = container(
            Column::new()
                .spacing(4)
                .push(
                    button(Text::new("+").center())
                        .on_press(Message::View(window_id, ViewMessage::ZoomIn))
                        .width(btn_width)
                        .style(zoom_btn),
                )
                .push(
                    button(Text::new("\u{2212}").center())
                        .on_press(Message::View(window_id, ViewMessage::ZoomOut))
                        .width(btn_width)
                        .style(zoom_btn),
                )
                .push(
                    button(Text::new("Fit").center())
                        .on_press(Message::View(window_id, ViewMessage::ToggleAutoFit))
                        .width(btn_width)
                        .style(if self.auto_fit_enabled {
                            zoom_btn_active
                        } else {
                            zoom_btn
                        }),
                ),
        )
        .align_right(Fill)
        .align_bottom(Fill)
        .padding(10);

        let mut canvas_stack: Vec<Element<'_, Message>> = vec![canvas, zoom_controls.into()];

        if let Some(ref tip) = self.tooltip {
            canvas_stack.push(Self::build_tooltip_overlay(tip));
        }

        if let Some(ref editor) = self.initializer_editor {
            canvas_stack.push(self.build_initializer_dialog(window_id, editor));
        }

        if let Some(menu_pos) = self.context_menu {
            canvas_stack.push(Self::build_context_menu(window_id, menu_pos));
        }

        stack(canvas_stack).into()
    }

    fn build_tooltip_overlay<'a>(tip: &crate::window_state::Tooltip) -> Element<'a, Message> {
        container(
            container(Text::new(tip.text.clone()).size(20).color(Color::WHITE))
                .padding(8)
                .style(|_theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgb(0.12, 0.12, 0.12))),
                    border: iced::Border {
                        color: Color::WHITE,
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                }),
        )
        .padding(iced::Padding {
            top: tip.y + 26.0,
            right: 0.0,
            bottom: 0.0,
            left: (tip.x - 80.0).max(0.0),
        })
        .into()
    }

    fn build_initializer_dialog<'a>(
        &self,
        window_id: window::Id,
        editor: &InitializerEditor,
    ) -> Element<'a, Message> {
        let port_label =
            if let Some(pref) = self.flow_definition.process_refs.get(editor.node_index) {
                let alias = if pref.alias.is_empty() {
                    derive_short_name(&pref.source)
                } else {
                    pref.alias.clone()
                };
                format!("{}/{}", alias, editor.port_name)
            } else {
                editor.port_name.clone()
            };

        let init_types = vec!["none", "once", "always"];
        let selected: Option<&str> = init_types.iter().find(|&&t| t == editor.init_type).copied();

        let mut dialog_col = Column::new()
            .spacing(8)
            .padding(12)
            .push(Text::new(format!("Initializer: {port_label}")).size(14))
            .push(
                pick_list(init_types, selected, move |s: &str| {
                    Message::InitializerTypeChanged(window_id, s.to_string())
                })
                .text_size(12),
            );

        if editor.init_type != "none" {
            dialog_col = dialog_col.push(
                text_input("JSON value (e.g. 42, \"hello\", true)", &editor.value_text)
                    .on_input(move |v| Message::InitializerValueChanged(window_id, v))
                    .size(12)
                    .padding(6),
            );
        }

        dialog_col = dialog_col.push(
            Row::new()
                .spacing(8)
                .push(
                    button(Text::new("Apply").size(12).center())
                        .on_press(Message::InitializerApply(window_id))
                        .style(button::primary)
                        .padding(6),
                )
                .push(
                    button(Text::new("Cancel").size(12).center())
                        .on_press(Message::InitializerCancel(window_id))
                        .style(button::secondary)
                        .padding(6),
                ),
        );

        container(
            container(dialog_col)
                .width(280)
                .style(|_theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(Color::from_rgb(0.15, 0.15, 0.15))),
                    border: iced::Border {
                        color: Color::from_rgb(0.4, 0.4, 0.4),
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                }),
        )
        .center(Fill)
        .into()
    }

    fn build_context_menu(
        window_id: window::Id,
        menu_pos: crate::window_state::MenuPosition,
    ) -> Element<'static, Message> {
        let menu = container(
            Column::new()
                .spacing(2)
                .push(
                    button(Text::new("+ New Sub-flow").size(13))
                        .on_press(Message::NewSubFlow(window_id))
                        .style(button::text)
                        .padding([6, 16])
                        .width(Fill),
                )
                .push(
                    button(Text::new("+ New Function").size(13))
                        .on_press(Message::NewFunction(window_id))
                        .style(button::text)
                        .padding([6, 16])
                        .width(Fill),
                ),
        )
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.18, 0.18, 0.22))),
            border: iced::Border {
                color: Color::from_rgb(0.4, 0.4, 0.4),
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        })
        .width(160)
        .padding(4);

        container(menu)
            .padding(iced::Padding {
                top: menu_pos.y,
                left: menu_pos.x,
                right: 0.0,
                bottom: 0.0,
            })
            .into()
    }
} // impl WindowState (view)
