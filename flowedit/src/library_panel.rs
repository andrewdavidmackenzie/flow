//! Library panel module that displays available flow processes from cached
//! library manifests and parsed definitions in a collapsible tree view.
//!
//! The panel is built from library manifests loaded after parsing a flow file.
//! Each library's full catalog of functions is shown (not just those used by the
//! flow), organized as Library > Category > Function. Clicking a function emits
//! a message to add it as a new node on the canvas.

use std::collections::{BTreeMap, HashMap};

use iced::widget::{button, container, scrollable, text, tooltip, Column, Row};
use iced::{Element, Length};
use url::Url;

use flowcore::model::lib_manifest::LibraryManifest;
use flowcore::model::process::Process;

/// Width of the library side panel in pixels.
const PANEL_WIDTH: f32 = 220.0;

/// Messages produced by the library panel.
#[derive(Debug, Clone)]
pub(crate) enum LibraryMessage {
    /// Toggle expansion of a library entry at the given index.
    ToggleLibrary(usize),
    /// Toggle expansion of a category within a library.
    ToggleCategory(usize, usize),
    /// Add a function to the canvas: (`source_url`, `function_name`).
    AddFunction(String, String),
    /// View a library function/flow definition.
    ViewFunction(String, String),
    /// Add a library manually via file dialog.
    AddLibrary,
}

/// Result of a library panel interaction.
#[derive(Debug, PartialEq)]
pub(crate) enum LibraryAction {
    None,
    Add(String, String),
    View(String, String),
    AddLibrary,
}

/// A single function entry in the library tree.
#[derive(Debug, Clone)]
pub(crate) struct FunctionEntry {
    /// Display name of the function (e.g., "add")
    pub name: String,
    /// Source URL for this function (e.g., `lib://flowstdlib/math/add`)
    pub source: String,
    /// Description from the canonical definition
    pub description: String,
}

/// A category grouping functions within a library.
#[derive(Debug, Clone)]
pub(crate) struct CategoryEntry {
    /// Category name (e.g., "math")
    pub name: String,
    /// Functions in this category
    pub functions: Vec<FunctionEntry>,
    /// Whether the category is expanded in the tree view
    pub expanded: bool,
}

/// A top-level library entry.
#[derive(Debug, Clone)]
pub(crate) struct LibraryEntry {
    /// Library name (e.g., "flowstdlib")
    pub name: String,
    /// Categories in this library
    pub categories: Vec<CategoryEntry>,
    /// Whether the library is expanded in the tree view
    pub expanded: bool,
}

/// The complete library tree built from cached manifests and definitions.
#[derive(Debug, Clone)]
pub(crate) struct LibraryTree {
    /// All discovered libraries
    pub libraries: Vec<LibraryEntry>,
}

impl LibraryTree {
    /// Build a library tree from cached manifests and parsed definitions.
    ///
    /// For each library manifest, all locator URLs are used to build the tree
    /// (library > category > function). Context functions are shown under a
    /// "Context" library entry, derived from the parsed context definitions.
    pub(crate) fn from_cache(
        library_cache: &HashMap<Url, LibraryManifest>,
        lib_definitions: &HashMap<Url, Process>,
        context_definitions: &HashMap<Url, Process>,
    ) -> Self {
        let mut libraries = Vec::new();

        // Build tree entries from library manifests
        for manifest in library_cache.values() {
            let lib_name = manifest.lib_url.host_str().unwrap_or("unknown").to_string();
            let categories = build_categories_from_manifest(&manifest.locators, lib_definitions);

            if !categories.is_empty() {
                libraries.push(LibraryEntry {
                    name: lib_name,
                    categories,
                    expanded: true,
                });
            }
        }

        // Build context functions entry from parsed context definitions
        let context_entry = build_context_entry(context_definitions);
        let has_context = !context_entry.categories.is_empty();
        if has_context {
            libraries.insert(0, context_entry);
        }

        // Sort non-context libraries by name for consistent display.
        // Skip index 0 only if we actually inserted a Context entry there.
        let sort_start = if has_context { 1 } else { 0 };
        if let Some(rest) = libraries.get_mut(sort_start..) {
            rest.sort_by(|a, b| a.name.cmp(&b.name));
        }

        LibraryTree { libraries }
    }

    /// Handle a library message, updating expansion state.
    ///
    /// Result of a library panel interaction.
    pub(crate) fn update(&mut self, message: &LibraryMessage) -> LibraryAction {
        match message {
            LibraryMessage::ToggleLibrary(lib_idx) => {
                if let Some(lib) = self.libraries.get_mut(*lib_idx) {
                    lib.expanded = !lib.expanded;
                }
                LibraryAction::None
            }
            LibraryMessage::ToggleCategory(lib_idx, cat_idx) => {
                if let Some(lib) = self.libraries.get_mut(*lib_idx) {
                    if let Some(cat) = lib.categories.get_mut(*cat_idx) {
                        cat.expanded = !cat.expanded;
                    }
                }
                LibraryAction::None
            }
            LibraryMessage::AddFunction(source, name) => {
                LibraryAction::Add(source.clone(), name.clone())
            }
            LibraryMessage::ViewFunction(source, name) => {
                LibraryAction::View(source.clone(), name.clone())
            }
            LibraryMessage::AddLibrary => LibraryAction::AddLibrary,
        }
    }

    /// Render the library panel as an iced `Element`.
    pub(crate) fn view(&self) -> Element<'_, LibraryMessage> {
        let mut content = Column::new().spacing(2).padding(6);

        let header = text("Process Library").size(14);
        let add_lib_btn = button(text("+ Library").size(11))
            .on_press(LibraryMessage::AddLibrary)
            .style(button::secondary)
            .padding([2, 6]);

        content = content.push(Row::new().spacing(8).push(header).push(add_lib_btn));

        if self.libraries.is_empty() {
            content = content.push(
                text("No libraries referenced\nby this flow.")
                    .size(12)
                    .color(iced::Color::from_rgb(0.6, 0.6, 0.6)),
            );
        }

        for (lib_idx, lib) in self.libraries.iter().enumerate() {
            let toggle_icon = if lib.expanded {
                "\u{25BC}" // down-pointing triangle
            } else {
                "\u{25B6}" // right-pointing triangle
            };
            let lib_btn = button(
                Row::new()
                    .spacing(4)
                    .push(text(toggle_icon).size(10))
                    .push(text(&lib.name).size(13)),
            )
            .on_press(LibraryMessage::ToggleLibrary(lib_idx))
            .style(button::text)
            .padding(2);

            content = content.push(lib_btn);

            if lib.expanded {
                for (cat_idx, cat) in lib.categories.iter().enumerate() {
                    let cat_icon = if cat.expanded { "\u{25BC}" } else { "\u{25B6}" };
                    let cat_btn = button(
                        Row::new()
                            .spacing(4)
                            .push(text(cat_icon).size(9))
                            .push(text(&cat.name).size(12)),
                    )
                    .on_press(LibraryMessage::ToggleCategory(lib_idx, cat_idx))
                    .style(button::text)
                    .padding(iced::Padding {
                        top: 1.0,
                        right: 1.0,
                        bottom: 1.0,
                        left: 14.0,
                    });

                    content = content.push(cat_btn);

                    if cat.expanded {
                        for func in &cat.functions {
                            let view_btn = button(text("\u{270E}").size(10))
                                .on_press(LibraryMessage::ViewFunction(
                                    func.source.clone(),
                                    func.name.clone(),
                                ))
                                .style(button::text)
                                .padding([1, 3]);

                            let func_btn = button(text(&func.name).size(11))
                                .on_press(LibraryMessage::AddFunction(
                                    func.source.clone(),
                                    func.name.clone(),
                                ))
                                .style(button::text)
                                .padding([2, 4]);

                            let row = Row::new()
                                .spacing(2)
                                .align_y(iced::Alignment::Center)
                                .push(view_btn)
                                .push(func_btn);

                            let entry_widget: Element<'_, LibraryMessage> =
                                if func.description.is_empty() {
                                    row.into()
                                } else {
                                    tooltip(
                                        row,
                                        text(&func.description).size(14),
                                        tooltip::Position::Bottom,
                                    )
                                    .gap(2)
                                    .style(|_theme: &iced::Theme| container::Style {
                                        background: Some(iced::Background::Color(
                                            iced::Color::from_rgb(0.12, 0.12, 0.12),
                                        )),
                                        border: iced::Border {
                                            color: iced::Color::WHITE,
                                            width: 1.0,
                                            radius: 4.0.into(),
                                        },
                                        ..Default::default()
                                    })
                                    .into()
                                };

                            content =
                                content.push(container(entry_widget).padding(iced::Padding {
                                    top: 0.0,
                                    right: 0.0,
                                    bottom: 0.0,
                                    left: 24.0,
                                }));
                        }
                    }
                }
            }
        }

        container(scrollable(content).height(Length::Fill))
            .width(PANEL_WIDTH)
            .height(Length::Fill)
            .style(|_theme: &iced::Theme| container::Style {
                border: iced::Border {
                    color: iced::Color::from_rgb(0.3, 0.3, 0.3),
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
    }
}

/// Build category entries from a manifest's locator map.
///
/// Each locator URL has the form `lib://library/category/function`.
/// We extract category and function names from the URL path segments.
fn build_categories_from_manifest(
    locators: &BTreeMap<Url, flowcore::model::lib_manifest::ImplementationLocator>,
    lib_definitions: &HashMap<Url, Process>,
) -> Vec<CategoryEntry> {
    // Group functions by category
    let mut category_map: BTreeMap<String, Vec<FunctionEntry>> = BTreeMap::new();

    for url in locators.keys() {
        // URL path looks like "/category/function" (leading slash)
        let path = url.path();
        let segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();

        let (cat_name, func_name) = match segments.as_slice() {
            [] => continue,
            [single] => ("general".to_string(), (*single).to_string()),
            [first, rest @ ..] => ((*first).to_string(), rest.join("/")),
        };

        if func_name.is_empty() {
            continue;
        }

        // Extract description from definition if available
        let description = lib_definitions
            .get(url)
            .map(|process| match process {
                Process::FlowProcess(flow_def) => flow_def.description.clone(),
                Process::FunctionProcess(func_def) => func_def.description.clone(),
            })
            .unwrap_or_default();

        category_map
            .entry(cat_name)
            .or_default()
            .push(FunctionEntry {
                name: func_name,
                source: url.to_string(),
                description,
            });
    }

    let mut categories: Vec<CategoryEntry> = category_map
        .into_iter()
        .map(|(name, mut functions)| {
            functions.sort_by(|a, b| a.name.cmp(&b.name));
            CategoryEntry {
                name,
                functions,
                expanded: false,
            }
        })
        .collect();

    categories.sort_by(|a, b| a.name.cmp(&b.name));
    categories
}

/// Build a "Context" library entry from parsed context definitions.
///
/// Context URLs have the form `context://category/function`.
fn build_context_entry(context_definitions: &HashMap<Url, Process>) -> LibraryEntry {
    let mut category_map: BTreeMap<String, Vec<FunctionEntry>> = BTreeMap::new();

    for (url, process) in context_definitions {
        // context://category/function
        let cat_name = url.host_str().unwrap_or("general").to_string();
        let func_name = url
            .path()
            .trim_start_matches('/')
            .split('/')
            .last()
            .unwrap_or("")
            .to_string();

        if func_name.is_empty() {
            continue;
        }

        // Extract description from definition
        let description = match process {
            Process::FlowProcess(flow_def) => flow_def.description.clone(),
            Process::FunctionProcess(func_def) => func_def.description.clone(),
        };

        category_map
            .entry(cat_name)
            .or_default()
            .push(FunctionEntry {
                name: func_name,
                source: url.to_string(),
                description,
            });
    }

    let mut categories: Vec<CategoryEntry> = category_map
        .into_iter()
        .map(|(name, mut functions)| {
            functions.sort_by(|a, b| a.name.cmp(&b.name));
            CategoryEntry {
                name,
                functions,
                expanded: false,
            }
        })
        .collect();

    categories.sort_by(|a, b| a.name.cmp(&b.name));

    LibraryEntry {
        name: "Context".to_string(),
        categories,
        expanded: true,
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod test {
    use super::*;

    #[test]
    fn empty_library_tree_view() {
        let tree = LibraryTree {
            libraries: Vec::new(),
        };
        // Should render without panic
        let _element: Element<'_, LibraryMessage> = tree.view();
    }

    #[test]
    fn toggle_library_expansion() {
        let mut tree = LibraryTree {
            libraries: vec![LibraryEntry {
                name: "test".into(),
                categories: Vec::new(),
                expanded: false,
            }],
        };
        let result = tree.update(&LibraryMessage::ToggleLibrary(0));
        assert_eq!(result, LibraryAction::None);
        assert!(tree.libraries.first().is_some_and(|l| l.expanded));
    }

    #[test]
    fn toggle_category_expansion() {
        let mut tree = LibraryTree {
            libraries: vec![LibraryEntry {
                name: "test".into(),
                categories: vec![CategoryEntry {
                    name: "math".into(),
                    functions: Vec::new(),
                    expanded: false,
                }],
                expanded: true,
            }],
        };
        let result = tree.update(&LibraryMessage::ToggleCategory(0, 0));
        assert_eq!(result, LibraryAction::None);
        assert!(tree
            .libraries
            .first()
            .and_then(|l| l.categories.first())
            .is_some_and(|c| c.expanded));
    }

    #[test]
    fn add_function_returns_source() {
        let mut tree = LibraryTree {
            libraries: Vec::new(),
        };
        let result = tree.update(&LibraryMessage::AddFunction(
            "lib://flowstdlib/math/add".into(),
            "add".into(),
        ));
        assert_eq!(
            result,
            LibraryAction::Add("lib://flowstdlib/math/add".into(), "add".into())
        );
    }

    #[test]
    fn toggle_out_of_bounds_does_not_panic() {
        let mut tree = LibraryTree {
            libraries: Vec::new(),
        };
        let result = tree.update(&LibraryMessage::ToggleLibrary(99));
        assert_eq!(result, LibraryAction::None);
        let result = tree.update(&LibraryMessage::ToggleCategory(99, 0));
        assert_eq!(result, LibraryAction::None);
    }

    #[test]
    fn from_cache_empty() {
        let tree = LibraryTree::from_cache(&HashMap::new(), &HashMap::new(), &HashMap::new());
        assert!(tree.libraries.is_empty());
    }

    #[test]
    fn from_cache_with_context_only() {
        let mut context_defs = HashMap::new();
        let url = Url::parse("context://stdio/stdout").expect("valid url");
        context_defs.insert(
            url,
            Process::FunctionProcess(
                flowcore::model::function_definition::FunctionDefinition::default(),
            ),
        );
        let tree = LibraryTree::from_cache(&HashMap::new(), &HashMap::new(), &context_defs);
        assert_eq!(tree.libraries.len(), 1);
        assert_eq!(tree.libraries[0].name, "Context");
        assert_eq!(tree.libraries[0].categories.len(), 1);
        assert_eq!(tree.libraries[0].categories[0].name, "stdio");
        assert_eq!(tree.libraries[0].categories[0].functions.len(), 1);
        assert_eq!(tree.libraries[0].categories[0].functions[0].name, "stdout");
    }

    #[test]
    fn build_categories_from_locators() {
        let mut locators = BTreeMap::new();
        locators.insert(
            Url::parse("lib://flowstdlib/math/add").expect("valid url"),
            flowcore::model::lib_manifest::ImplementationLocator::RelativePath(
                "math/add.wasm".into(),
            ),
        );
        locators.insert(
            Url::parse("lib://flowstdlib/math/subtract").expect("valid url"),
            flowcore::model::lib_manifest::ImplementationLocator::RelativePath(
                "math/subtract.wasm".into(),
            ),
        );
        locators.insert(
            Url::parse("lib://flowstdlib/control/tap").expect("valid url"),
            flowcore::model::lib_manifest::ImplementationLocator::RelativePath(
                "control/tap.wasm".into(),
            ),
        );

        let categories = build_categories_from_manifest(&locators, &HashMap::new());
        assert_eq!(categories.len(), 2);
        // Categories should be sorted alphabetically
        assert_eq!(categories[0].name, "control");
        assert_eq!(categories[1].name, "math");
        assert_eq!(categories[1].functions.len(), 2);
        assert_eq!(categories[1].functions[0].name, "add");
        assert_eq!(categories[1].functions[1].name, "subtract");
    }
}
