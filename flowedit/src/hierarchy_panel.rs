//! Flow hierarchy panel that shows the structure of the loaded flow
//! as a collapsible tree view. The root flow is at the top, with child
//! sub-flows and functions as children, recursively.

use std::path::{Path, PathBuf};

use iced::widget::{button, container, scrollable, text, Column, Row};
use iced::{Color, Element, Length};

use flowcore::deserializers::deserializer::get;
use flowcore::model::process::Process;

const PANEL_WIDTH: f32 = 220.0;

#[derive(Debug, Clone)]
pub(crate) enum HierarchyMessage {
    Toggle(Vec<usize>),
    Open(String, PathBuf),
}

#[derive(Debug, Clone)]
pub(crate) enum NodeKind {
    Flow,
    Function,
    Library,
}

#[derive(Debug, Clone)]
pub(crate) struct HierarchyNode {
    pub name: String,
    pub kind: NodeKind,
    pub source: String,
    pub path: Option<PathBuf>,
    pub children: Vec<HierarchyNode>,
    pub expanded: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct FlowHierarchy {
    pub root: Option<HierarchyNode>,
}

impl FlowHierarchy {
    pub(crate) fn build(flow_path: &Path) -> Self {
        let root = build_node_from_path(flow_path, 0);
        Self { root }
    }

    pub(crate) fn empty() -> Self {
        Self { root: None }
    }

    pub(crate) fn update(&mut self, msg: &HierarchyMessage) -> Option<(String, PathBuf)> {
        match msg {
            HierarchyMessage::Toggle(indices) => {
                if let Some(ref mut root) = self.root {
                    toggle_at(root, indices, 0);
                }
                None
            }
            HierarchyMessage::Open(source, path) => Some((source.clone(), path.clone())),
        }
    }

    pub(crate) fn view(&self) -> Element<'_, HierarchyMessage> {
        let mut col = Column::new().spacing(2).push(
            container(text("Flow Hierarchy").size(14))
                .padding([6, 8])
                .width(Length::Fill),
        );

        if let Some(ref root) = self.root {
            col = col.push(view_node(root, &[]));
        }

        container(scrollable(col).height(Length::Fill))
            .width(PANEL_WIDTH)
            .style(|_theme: &iced::Theme| iced::widget::container::Style {
                border: iced::Border {
                    color: Color::from_rgb(0.3, 0.3, 0.3),
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            })
            .padding(4)
            .into()
    }
}

fn toggle_at(node: &mut HierarchyNode, indices: &[usize], depth: usize) {
    if depth >= indices.len() {
        node.expanded = !node.expanded;
        return;
    }
    let Some(&idx) = indices.get(depth) else {
        return;
    };
    if let Some(child) = node.children.get_mut(idx) {
        if depth + 1 == indices.len() {
            child.expanded = !child.expanded;
        } else {
            toggle_at(child, indices, depth + 1);
        }
    }
}

fn view_node<'a>(node: &'a HierarchyNode, path: &[usize]) -> Element<'a, HierarchyMessage> {
    let indent = path.len() as f32 * 16.0;
    let icon = match node.kind {
        NodeKind::Flow => {
            if node.expanded {
                "\u{25BC}"
            } else {
                "\u{25B6}"
            }
        }
        NodeKind::Function => "\u{25C6}",
        NodeKind::Library => "\u{25CB}",
    };

    let color = match node.kind {
        NodeKind::Flow => Color::from_rgb(0.9, 0.6, 0.2),
        NodeKind::Function => Color::from_rgb(0.6, 0.3, 0.8),
        NodeKind::Library => Color::from_rgb(0.3, 0.5, 0.9),
    };

    let path_vec: Vec<usize> = path.to_vec();
    let label = Row::new()
        .spacing(4)
        .push(text(icon).size(11).color(color))
        .push(text(&node.name).size(13).color(color));

    let label_btn = match node.kind {
        NodeKind::Flow => {
            // Flows toggle expand/collapse on click
            button(label)
                .on_press(HierarchyMessage::Toggle(path_vec))
                .style(button::text)
                .padding([2, 4])
        }
        NodeKind::Function => {
            // Functions open in editor on click
            if let Some(ref p) = node.path {
                button(label)
                    .on_press(HierarchyMessage::Open(node.source.clone(), p.clone()))
                    .style(button::text)
                    .padding([2, 4])
            } else {
                button(label).style(button::text).padding([2, 4])
            }
        }
        NodeKind::Library => {
            // Library nodes are non-interactive (no path)
            button(label).style(button::text).padding([2, 4])
        }
    };

    // Add a small open button for flows (except root)
    let mut row = Row::new().push(container(label_btn).padding(iced::Padding {
        top: 0.0,
        bottom: 0.0,
        left: indent,
        right: 0.0,
    }));
    if matches!(node.kind, NodeKind::Flow) && !path.is_empty() {
        if let Some(ref p) = node.path {
            row = row.push(
                button(text("\u{270E}").size(11).color(color))
                    .on_press(HierarchyMessage::Open(node.source.clone(), p.clone()))
                    .style(button::text)
                    .padding([2, 4]),
            );
        }
    }

    let mut col = Column::new().push(row);

    if node.expanded {
        for (i, child) in node.children.iter().enumerate() {
            let mut child_path = path.to_vec();
            child_path.push(i);
            col = col.push(view_node(child, &child_path));
        }
    }

    col.into()
}

fn build_node_from_path(flow_path: &Path, depth: usize) -> Option<HierarchyNode> {
    if depth > 10 || !flow_path.exists() {
        return None;
    }

    let contents = std::fs::read_to_string(flow_path).ok()?;
    let url = url::Url::from_file_path(std::fs::canonicalize(flow_path).ok()?).ok()?;
    let deserializer = get::<Process>(&url).ok()?;
    let process = deserializer.deserialize(&contents, Some(&url)).ok()?;

    let dir = flow_path.parent()?;
    let name = match &process {
        Process::FlowProcess(f) => f.name.clone(),
        Process::FunctionProcess(f) => f.name.clone(),
    };

    match process {
        Process::FlowProcess(flow) => {
            let mut children = Vec::new();
            for pref in &flow.process_refs {
                let alias = if pref.alias.is_empty() {
                    pref.source
                        .rsplit('/')
                        .next()
                        .unwrap_or(&pref.source)
                        .to_string()
                } else {
                    pref.alias.clone()
                };

                if pref.source.starts_with("lib://") || pref.source.starts_with("context://") {
                    children.push(HierarchyNode {
                        name: alias,
                        kind: NodeKind::Library,
                        source: pref.source.clone(),
                        path: None,
                        children: Vec::new(),
                        expanded: false,
                    });
                } else {
                    // Relative source — resolve to file
                    let resolved = resolve_source(dir, &pref.source);
                    if let Some(ref resolved_path) = resolved {
                        if let Some(child) = build_node_from_path(resolved_path, depth + 1) {
                            children.push(HierarchyNode {
                                name: alias,
                                path: Some(resolved_path.clone()),
                                ..child
                            });
                        } else {
                            children.push(HierarchyNode {
                                name: alias,
                                kind: NodeKind::Function,
                                source: pref.source.clone(),
                                path: resolved,
                                children: Vec::new(),
                                expanded: false,
                            });
                        }
                    }
                }
            }

            Some(HierarchyNode {
                name,
                kind: NodeKind::Flow,
                source: String::new(),
                path: Some(flow_path.to_path_buf()),
                children,
                expanded: true,
            })
        }
        Process::FunctionProcess(_) => Some(HierarchyNode {
            name,
            kind: NodeKind::Function,
            source: String::new(),
            path: Some(flow_path.to_path_buf()),
            children: Vec::new(),
            expanded: false,
        }),
    }
}

fn resolve_source(dir: &Path, source: &str) -> Option<PathBuf> {
    let candidate = dir.join(source);
    if candidate.exists() {
        return Some(std::fs::canonicalize(&candidate).unwrap_or(candidate));
    }
    let with_ext = dir.join(format!("{source}.toml"));
    if with_ext.exists() {
        return Some(std::fs::canonicalize(&with_ext).unwrap_or(with_ext));
    }
    let dir_default = dir.join(source).join("default.toml");
    if dir_default.exists() {
        return Some(std::fs::canonicalize(&dir_default).unwrap_or(dir_default));
    }
    None
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn empty_hierarchy() {
        let h = FlowHierarchy::empty();
        assert!(h.root.is_none());
    }
}
