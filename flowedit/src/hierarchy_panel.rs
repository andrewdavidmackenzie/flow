//! Flow hierarchy panel that shows the structure of the loaded flow
//! as a collapsible tree view. The root flow is at the top, with child
//! sub-flows and functions as children, recursively.

use std::path::PathBuf;

use iced::widget::{button, container, scrollable, text, Column, Row};
use iced::{Color, Element, Length};

use flowcore::model::flow_definition::FlowDefinition;
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
    pub(crate) fn from_flow_definition(flow_def: &FlowDefinition) -> Self {
        let root = build_node_from_flow(flow_def);
        Self { root: Some(root) }
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

#[allow(clippy::cast_precision_loss)]
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

fn build_node_from_flow(flow_def: &FlowDefinition) -> HierarchyNode {
    let mut children = Vec::new();

    for pref in &flow_def.process_refs {
        let alias = if pref.alias.is_empty() {
            crate::canvas_view::derive_short_name(&pref.source)
        } else {
            pref.alias.clone()
        };

        match flow_def.subprocesses.get(&alias) {
            Some(Process::FlowProcess(sub_flow)) => {
                // Recursively build children from the sub-flow
                let child = build_node_from_flow(sub_flow);
                children.push(HierarchyNode {
                    name: alias,
                    kind: NodeKind::Flow,
                    source: pref.source.clone(),
                    path: sub_flow.source_url.to_file_path().ok(),
                    children: child.children,
                    expanded: true,
                });
            }
            Some(Process::FunctionProcess(func)) => {
                let kind = if func.lib_reference.is_some() || func.context_reference.is_some() {
                    NodeKind::Library
                } else {
                    NodeKind::Function
                };
                children.push(HierarchyNode {
                    name: alias,
                    kind,
                    source: pref.source.clone(),
                    path: func.source_url.to_file_path().ok(),
                    children: Vec::new(),
                    expanded: false,
                });
            }
            None => {
                // Unresolved - determine kind from source string
                let kind =
                    if pref.source.starts_with("lib://") || pref.source.starts_with("context://") {
                        NodeKind::Library
                    } else {
                        NodeKind::Function
                    };
                children.push(HierarchyNode {
                    name: alias,
                    kind,
                    source: pref.source.clone(),
                    path: None,
                    children: Vec::new(),
                    expanded: false,
                });
            }
        }
    }

    HierarchyNode {
        name: flow_def.name.clone(),
        kind: NodeKind::Flow,
        source: String::new(),
        path: flow_def.source_url.to_file_path().ok(),
        children,
        expanded: true,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn empty_hierarchy() {
        let h = FlowHierarchy::empty();
        assert!(h.root.is_none());
    }

    #[test]
    fn toggle_root_node() {
        let mut h = FlowHierarchy {
            root: Some(HierarchyNode {
                name: "root_flow".into(),
                kind: NodeKind::Flow,
                source: String::new(),
                path: None,
                children: Vec::new(),
                expanded: true,
            }),
        };
        h.update(&HierarchyMessage::Toggle(vec![]));
        assert!(h.root.as_ref().is_some_and(|r| !r.expanded));
        h.update(&HierarchyMessage::Toggle(vec![]));
        assert!(h.root.as_ref().is_some_and(|r| r.expanded));
    }

    #[test]
    fn toggle_child_node() {
        let mut h = FlowHierarchy {
            root: Some(HierarchyNode {
                name: "root".into(),
                kind: NodeKind::Flow,
                source: String::new(),
                path: None,
                children: vec![HierarchyNode {
                    name: "child_flow".into(),
                    kind: NodeKind::Flow,
                    source: "child.toml".into(),
                    path: None,
                    children: Vec::new(),
                    expanded: false,
                }],
                expanded: true,
            }),
        };
        h.update(&HierarchyMessage::Toggle(vec![0]));
        assert!(h
            .root
            .as_ref()
            .and_then(|r| r.children.first())
            .is_some_and(|c| c.expanded));
    }

    #[test]
    fn toggle_nested_child() {
        let mut h = FlowHierarchy {
            root: Some(HierarchyNode {
                name: "root".into(),
                kind: NodeKind::Flow,
                source: String::new(),
                path: None,
                children: vec![HierarchyNode {
                    name: "sub".into(),
                    kind: NodeKind::Flow,
                    source: String::new(),
                    path: None,
                    children: vec![HierarchyNode {
                        name: "deep".into(),
                        kind: NodeKind::Flow,
                        source: String::new(),
                        path: None,
                        children: Vec::new(),
                        expanded: false,
                    }],
                    expanded: true,
                }],
                expanded: true,
            }),
        };
        h.update(&HierarchyMessage::Toggle(vec![0, 0]));
        assert!(h
            .root
            .as_ref()
            .and_then(|r| r.children.first())
            .and_then(|c| c.children.first())
            .is_some_and(|c| c.expanded));
    }

    #[test]
    fn toggle_invalid_index_no_panic() {
        let mut h = FlowHierarchy {
            root: Some(HierarchyNode {
                name: "root".into(),
                kind: NodeKind::Flow,
                source: String::new(),
                path: None,
                children: Vec::new(),
                expanded: true,
            }),
        };
        h.update(&HierarchyMessage::Toggle(vec![99]));
    }

    #[test]
    fn open_returns_source_and_path() {
        let mut h = FlowHierarchy::empty();
        let result = h.update(&HierarchyMessage::Open(
            "sub.toml".into(),
            PathBuf::from("/tmp/sub.toml"),
        ));
        assert!(result.is_some());
        if let Some((source, path)) = result {
            assert_eq!(source, "sub.toml");
            assert_eq!(path, PathBuf::from("/tmp/sub.toml"));
        }
    }

    #[test]
    fn toggle_empty_hierarchy_no_panic() {
        let mut h = FlowHierarchy::empty();
        h.update(&HierarchyMessage::Toggle(vec![]));
    }

    #[test]
    fn view_with_nodes_renders() {
        let h = FlowHierarchy {
            root: Some(HierarchyNode {
                name: "test_flow".into(),
                kind: NodeKind::Flow,
                source: String::new(),
                path: None,
                children: vec![
                    HierarchyNode {
                        name: "func".into(),
                        kind: NodeKind::Function,
                        source: "func.rs".into(),
                        path: Some(PathBuf::from("/tmp/func.toml")),
                        children: Vec::new(),
                        expanded: false,
                    },
                    HierarchyNode {
                        name: "lib_func".into(),
                        kind: NodeKind::Library,
                        source: "lib://flowstdlib/math/add".into(),
                        path: None,
                        children: Vec::new(),
                        expanded: false,
                    },
                ],
                expanded: true,
            }),
        };
        let _element: Element<'_, HierarchyMessage> = h.view();
    }

    #[test]
    fn view_empty_renders() {
        let h = FlowHierarchy::empty();
        let _element: Element<'_, HierarchyMessage> = h.view();
    }
}
