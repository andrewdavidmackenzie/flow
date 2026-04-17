//! Library panel module that discovers and displays available flow processes
//! from installed libraries in a collapsible tree view.
//!
//! The panel scans `FLOW_LIB_PATH` (defaulting to `~/.flow/lib`) for installed
//! library directories and builds a tree of Library > Category > Function entries.
//! Clicking a function emits a message to add it as a new node on the canvas.

use iced::widget::{button, container, scrollable, text, Column, Row};
use iced::{Element, Length};

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
}

/// A single function entry in the library tree.
#[derive(Debug, Clone)]
pub(crate) struct FunctionEntry {
    /// Display name of the function (e.g., "add")
    pub name: String,
    /// Source URL for this function (e.g., `lib://flowstdlib/math/add`)
    pub source: String,
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

/// The complete library tree discovered from the filesystem.
#[derive(Debug, Clone)]
pub(crate) struct LibraryTree {
    /// All discovered libraries
    pub libraries: Vec<LibraryEntry>,
}

impl LibraryTree {
    /// Scan the library path and build the tree structure.
    ///
    /// Looks in `FLOW_LIB_PATH` (comma-separated) with `~/.flow/lib` as the
    /// default fallback. Each top-level directory is a library; subdirectories
    /// are categories; subdirectories of categories containing `.toml` files
    /// are functions. TOML files directly in a category directory are also
    /// treated as functions (e.g., flow definitions like `sequence.toml`).
    pub(crate) fn scan() -> Self {
        let lib_path = resolve_lib_path();
        let mut libraries = Vec::new();

        for dir in &lib_path {
            let lib_dir = std::path::Path::new(dir);
            if !lib_dir.is_dir() {
                continue;
            }

            // Each subdirectory of a lib path entry is a library
            let Ok(entries) = std::fs::read_dir(lib_dir) else {
                continue;
            };

            for lib_entry in entries.flatten() {
                let lib_path_buf = lib_entry.path();
                if !lib_path_buf.is_dir() {
                    continue;
                }

                let lib_name = lib_entry.file_name().to_string_lossy().to_string();
                let categories = scan_categories(&lib_path_buf, &lib_name);

                if !categories.is_empty() {
                    libraries.push(LibraryEntry {
                        name: lib_name,
                        categories,
                        expanded: true,
                    });
                }
            }
        }

        // Sort libraries by name for consistent display
        libraries.sort_by(|a, b| a.name.cmp(&b.name));
        LibraryTree { libraries }
    }

    /// Handle a library message, updating expansion state.
    ///
    /// Returns `Some((source, name))` if a function was clicked (for the caller
    /// to create a node), or `None` if just a toggle.
    pub(crate) fn update(&mut self, message: &LibraryMessage) -> Option<(String, String)> {
        match message {
            LibraryMessage::ToggleLibrary(lib_idx) => {
                if let Some(lib) = self.libraries.get_mut(*lib_idx) {
                    lib.expanded = !lib.expanded;
                }
                None
            }
            LibraryMessage::ToggleCategory(lib_idx, cat_idx) => {
                if let Some(lib) = self.libraries.get_mut(*lib_idx) {
                    if let Some(cat) = lib.categories.get_mut(*cat_idx) {
                        cat.expanded = !cat.expanded;
                    }
                }
                None
            }
            LibraryMessage::AddFunction(source, name) => Some((source.clone(), name.clone())),
        }
    }

    /// Render the library panel as an iced `Element`.
    pub(crate) fn view(&self) -> Element<'_, LibraryMessage> {
        let mut content = Column::new().spacing(2).padding(6);

        let header = text("Process Library").size(14);
        content = content.push(header);

        if self.libraries.is_empty() {
            content = content.push(
                text("No libraries found.\nCheck FLOW_LIB_PATH or\n~/.flow/lib")
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
                            let func_btn = button(text(&func.name).size(11))
                                .on_press(LibraryMessage::AddFunction(
                                    func.source.clone(),
                                    func.name.clone(),
                                ))
                                .style(button::secondary)
                                .padding(iced::Padding {
                                    top: 2.0,
                                    right: 6.0,
                                    bottom: 2.0,
                                    left: 28.0,
                                });

                            content = content.push(func_btn);
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

/// Resolve the library search path directories.
///
/// Reads `FLOW_LIB_PATH` as a comma-separated list and appends `~/.flow/lib`
/// as a default if it exists.
fn resolve_lib_path() -> Vec<String> {
    let mut paths = Vec::new();

    // Check FLOW_LIB_PATH environment variable
    if let Ok(env_path) = std::env::var("FLOW_LIB_PATH") {
        for p in env_path.split(',') {
            let trimmed = p.trim();
            if !trimmed.is_empty() {
                paths.push(trimmed.to_string());
            }
        }
    }

    // Add ~/.flow/lib as default
    if let Ok(home) = std::env::var("HOME") {
        let default_lib = format!("{home}/.flow/lib");
        if std::path::Path::new(&default_lib).is_dir() && !paths.contains(&default_lib) {
            paths.push(default_lib);
        }
    }

    paths
}

/// Scan a library directory for categories and their functions.
fn scan_categories(lib_dir: &std::path::Path, lib_name: &str) -> Vec<CategoryEntry> {
    let mut categories = Vec::new();

    let Ok(entries) = std::fs::read_dir(lib_dir) else {
        return categories;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let cat_name = entry.file_name().to_string_lossy().to_string();
        let functions = scan_functions(&path, lib_name, &cat_name);

        if !functions.is_empty() {
            categories.push(CategoryEntry {
                name: cat_name,
                functions,
                expanded: false,
            });
        }
    }

    categories.sort_by(|a, b| a.name.cmp(&b.name));
    categories
}

/// Scan a category directory for functions.
///
/// A function is either:
/// - A subdirectory containing a `.toml` file (e.g., `add/add.toml`)
/// - A `.toml` file directly in the category directory (e.g., `sequence.toml`)
fn scan_functions(cat_dir: &std::path::Path, lib_name: &str, cat_name: &str) -> Vec<FunctionEntry> {
    let mut functions = Vec::new();

    let Ok(entries) = std::fs::read_dir(cat_dir) else {
        return functions;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let entry_name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            // Check if subdirectory contains a .toml file
            let has_toml = std::fs::read_dir(&path).is_ok_and(|entries| {
                entries
                    .flatten()
                    .any(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("toml"))
            });

            if has_toml {
                let source = format!("lib://{lib_name}/{cat_name}/{entry_name}");
                functions.push(FunctionEntry {
                    name: entry_name,
                    source,
                });
            }
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
            // TOML file directly in category (e.g., sequence.toml, range.toml)
            let func_name = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            if !func_name.is_empty() {
                let source = format!("lib://{lib_name}/{cat_name}/{func_name}");
                functions.push(FunctionEntry {
                    name: func_name,
                    source,
                });
            }
        }
    }

    functions.sort_by(|a, b| a.name.cmp(&b.name));
    functions
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn resolve_lib_path_includes_default() {
        let paths = resolve_lib_path();
        // Should at least not panic; whether ~/.flow/lib exists depends on environment
        let _ = paths;
    }

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
        assert!(result.is_none());
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
        assert!(result.is_none());
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
            Some(("lib://flowstdlib/math/add".into(), "add".into()))
        );
    }

    #[test]
    fn toggle_out_of_bounds_does_not_panic() {
        let mut tree = LibraryTree {
            libraries: Vec::new(),
        };
        let result = tree.update(&LibraryMessage::ToggleLibrary(99));
        assert!(result.is_none());
        let result = tree.update(&LibraryMessage::ToggleCategory(99, 0));
        assert!(result.is_none());
    }
}
