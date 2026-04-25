//! Flow hierarchy panel that shows the structure of the loaded flow
//! as a collapsible tree view. The root flow is at the top, with child
//! sub-flows and functions as children, recursively.
//!
//! Walks the `FlowDefinition` tree directly at render time rather than
//! maintaining a parallel tree structure. Only UI state (which nodes
//! are expanded) is stored separately.

use std::collections::HashSet;

use iced::widget::{button, container, scrollable, text, Column, Row};
use iced::{Color, Element, Length};

use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::process::Process;
use flowcore::model::route::Route;

const PANEL_WIDTH: f32 = 220.0;
const MAX_HIERARCHY_DEPTH: usize = 10;

#[derive(Debug, Clone)]
pub(crate) enum HierarchyMessage {
    Toggle(Vec<usize>),
    Open(Route),
}

/// Tracks which tree nodes are expanded in the hierarchy panel.
///
/// Walks the `FlowDefinition` process tree directly during rendering —
/// no parallel tree is maintained. Each expanded node is identified by
/// its index path from the root (e.g., `[0, 2]` = first child's third child).
#[derive(Debug, Clone)]
pub(crate) struct FlowHierarchy {
    expanded: HashSet<Vec<usize>>,
}

impl FlowHierarchy {
    pub(crate) fn from_flow_definition(flow_def: &FlowDefinition) -> Self {
        let mut expanded = HashSet::new();
        // Expand root
        expanded.insert(vec![]);
        // Expand all sub-flow children by default
        collect_flow_paths(flow_def, &[], &mut expanded, 0);
        Self { expanded }
    }

    pub(crate) fn empty() -> Self {
        Self {
            expanded: HashSet::new(),
        }
    }

    pub(crate) fn update(&mut self, msg: &HierarchyMessage) -> Option<Route> {
        match msg {
            HierarchyMessage::Toggle(path) => {
                if self.expanded.contains(path) {
                    self.expanded.remove(path);
                } else {
                    self.expanded.insert(path.clone());
                }
                None
            }
            HierarchyMessage::Open(route) => Some(route.clone()),
        }
    }

    pub(crate) fn view<'a>(
        &'a self,
        flow_def: &'a FlowDefinition,
    ) -> Element<'a, HierarchyMessage> {
        let mut col = Column::new().spacing(2).push(
            container(text("Flow Hierarchy").size(14))
                .padding([6, 8])
                .width(Length::Fill),
        );

        col = col.push(self.view_flow(flow_def, &[]));

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

    #[allow(clippy::cast_precision_loss)]
    fn view_flow<'a>(
        &'a self,
        flow_def: &'a FlowDefinition,
        path: &[usize],
    ) -> Element<'a, HierarchyMessage> {
        let indent = path.len() as f32 * 16.0;
        let expanded = self.expanded.contains(path);
        let color = Color::from_rgb(0.9, 0.6, 0.2);
        let icon = if expanded { "\u{25BC}" } else { "\u{25B6}" };

        let path_vec: Vec<usize> = path.to_vec();
        let label = Row::new()
            .spacing(4)
            .push(text(icon).size(11).color(color))
            .push(text(&flow_def.name).size(13).color(color));

        let label_btn = button(label)
            .on_press(HierarchyMessage::Toggle(path_vec))
            .style(button::text)
            .padding([2, 4]);

        let mut row = Row::new().push(container(label_btn).padding(iced::Padding {
            top: 0.0,
            bottom: 0.0,
            left: indent,
            right: 0.0,
        }));
        if !path.is_empty() && !flow_def.route.is_empty() {
            row = row.push(
                button(text("\u{270E}").size(11).color(color))
                    .on_press(HierarchyMessage::Open(flow_def.route.clone()))
                    .style(button::text)
                    .padding([2, 4]),
            );
        }

        let mut col = Column::new().push(row);

        if expanded && path.len() < MAX_HIERARCHY_DEPTH {
            for (i, pref) in flow_def.process_refs.iter().enumerate() {
                let alias = if pref.alias.is_empty() {
                    crate::utils::derive_short_name(&pref.source)
                } else {
                    pref.alias.clone()
                };

                let mut child_path = path.to_vec();
                child_path.push(i);

                match flow_def.subprocesses.get(&alias) {
                    Some(Process::FlowProcess(sub_flow)) => {
                        col = col.push(self.view_flow(sub_flow, &child_path));
                    }
                    Some(Process::FunctionProcess(func)) => {
                        let is_library = func.get_lib_reference().is_some()
                            || func.get_context_reference().is_some();
                        col = col.push(view_leaf(
                            &alias,
                            func.route.clone(),
                            is_library,
                            &child_path,
                        ));
                    }
                    None => {
                        let is_library = pref.source.starts_with("lib://")
                            || pref.source.starts_with("context://");
                        col =
                            col.push(view_leaf(&alias, Route::default(), is_library, &child_path));
                    }
                }
            }
        }

        col.into()
    }
}

/// Render a leaf node (function or library) in the hierarchy.
#[allow(clippy::cast_precision_loss)]
fn view_leaf<'a>(
    name: &str,
    route: Route,
    is_library: bool,
    tree_path: &[usize],
) -> Element<'a, HierarchyMessage> {
    let indent = tree_path.len() as f32 * 16.0;
    let (icon, color) = if is_library {
        ("\u{25CB}", Color::from_rgb(0.3, 0.5, 0.9))
    } else {
        ("\u{25C6}", Color::from_rgb(0.6, 0.3, 0.8))
    };

    let label = Row::new()
        .spacing(4)
        .push(text(icon).size(11).color(color))
        .push(text(name.to_string()).size(13).color(color));

    let label_btn = if is_library || route.is_empty() {
        button(label).style(button::text).padding([2, 4])
    } else {
        button(label)
            .on_press(HierarchyMessage::Open(route))
            .style(button::text)
            .padding([2, 4])
    };

    container(label_btn)
        .padding(iced::Padding {
            top: 0.0,
            bottom: 0.0,
            left: indent,
            right: 0.0,
        })
        .into()
}

/// Recursively collect paths of flow nodes for default expansion.
fn collect_flow_paths(
    flow_def: &FlowDefinition,
    path: &[usize],
    expanded: &mut HashSet<Vec<usize>>,
    depth: usize,
) {
    if depth >= MAX_HIERARCHY_DEPTH {
        return;
    }
    for (i, pref) in flow_def.process_refs.iter().enumerate() {
        let alias = if pref.alias.is_empty() {
            crate::utils::derive_short_name(&pref.source)
        } else {
            pref.alias.clone()
        };
        if let Some(Process::FlowProcess(sub_flow)) = flow_def.subprocesses.get(&alias) {
            let mut child_path = path.to_vec();
            child_path.push(i);
            expanded.insert(child_path.clone());
            collect_flow_paths(sub_flow, &child_path, expanded, depth + 1);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use flowcore::model::function_definition::FunctionDefinition;
    use flowcore::model::name::Name;
    use flowcore::model::process_reference::ProcessReference;
    use std::collections::BTreeMap;

    fn simple_flow() -> FlowDefinition {
        let mut flow = FlowDefinition {
            name: Name::from("root"),
            ..FlowDefinition::default()
        };
        flow.process_refs.push(ProcessReference {
            alias: Name::from("func"),
            source: "func.toml".into(),
            initializations: BTreeMap::new(),
            x: None,
            y: None,
            width: None,
            height: None,
        });
        flow.subprocesses.insert(
            "func".into(),
            Process::FunctionProcess(FunctionDefinition::default()),
        );
        flow
    }

    fn nested_flow() -> FlowDefinition {
        let child = simple_flow();
        let mut root = FlowDefinition {
            name: Name::from("root"),
            ..FlowDefinition::default()
        };
        root.process_refs.push(ProcessReference {
            alias: Name::from("sub"),
            source: "sub.toml".into(),
            initializations: BTreeMap::new(),
            x: None,
            y: None,
            width: None,
            height: None,
        });
        root.subprocesses
            .insert("sub".into(), Process::FlowProcess(child));
        root
    }

    #[test]
    fn empty_hierarchy() {
        let h = FlowHierarchy::empty();
        assert!(h.expanded.is_empty());
    }

    #[test]
    fn from_flow_expands_root_and_subflows() {
        let flow = nested_flow();
        let h = FlowHierarchy::from_flow_definition(&flow);
        assert!(h.expanded.contains(&vec![]));
        assert!(h.expanded.contains(&vec![0]));
    }

    #[test]
    fn toggle_collapses_and_expands() {
        let flow = nested_flow();
        let mut h = FlowHierarchy::from_flow_definition(&flow);
        assert!(h.expanded.contains(&vec![]));
        h.update(&HierarchyMessage::Toggle(vec![]));
        assert!(!h.expanded.contains(&vec![]));
        h.update(&HierarchyMessage::Toggle(vec![]));
        assert!(h.expanded.contains(&vec![]));
    }

    #[test]
    fn open_returns_source_and_path() {
        let mut h = FlowHierarchy::empty();
        let result = h.update(&HierarchyMessage::Open(Route::from("/root/sub")));
        assert!(result.is_some());
        if let Some(route) = result {
            assert_eq!(route, Route::from("/root/sub"));
        }
    }

    #[test]
    fn toggle_empty_hierarchy_no_panic() {
        let mut h = FlowHierarchy::empty();
        h.update(&HierarchyMessage::Toggle(vec![]));
    }

    #[test]
    fn view_with_flow_renders() {
        let flow = simple_flow();
        let h = FlowHierarchy::from_flow_definition(&flow);
        let _element: Element<'_, HierarchyMessage> = h.view(&flow);
    }

    #[test]
    fn view_nested_renders() {
        let flow = nested_flow();
        let h = FlowHierarchy::from_flow_definition(&flow);
        let _element: Element<'_, HierarchyMessage> = h.view(&flow);
    }

    #[test]
    fn view_empty_renders() {
        let h = FlowHierarchy::empty();
        let flow = FlowDefinition::default();
        let _element: Element<'_, HierarchyMessage> = h.view(&flow);
    }
}
