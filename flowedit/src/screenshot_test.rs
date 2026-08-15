#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used)]

//! Screenshot tests for `flowedit`.
//!
//! These are ignored by default and only run via `make screenshots`
//! or `cargo test -- --ignored --test-threads=1`. They generate PNG
//! screenshots into `assets/screenshots/` using the headless `tiny-skia`
//! renderer and compare against existing gold standard images.
//!
//! All screenshots load real flow definitions from flowr/examples/ via the
//! compiler parser, so nodes have proper subprocesses, ports, colors, and
//! connections — matching what users actually see in the editor.

use super::*;
use crate::library_panel::{self, LibraryTree};
use iced::window;
use iced_test::simulator::simulator;
use std::collections::HashMap;
use url::Url;

/// Force the tiny-skia backend for deterministic cross-platform screenshots.
/// Must be called before creating a simulator.
fn force_tiny_skia() {
    std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
}

/// Return the path to `assets/screenshots/` relative to the project root.
fn screenshot_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("project root")
        .join("assets")
        .join("screenshots")
        .join(name)
}

/// Build a representative flowstdlib library tree with all 6 categories.
fn full_library_tree() -> LibraryTree {
    LibraryTree {
        libraries: vec![library_panel::LibraryEntry {
            name: "flowstdlib".into(),
            categories: vec![
                library_panel::CategoryEntry {
                    name: "charts".into(),
                    function_urls: vec![
                        Url::parse("lib://flowstdlib/charts/histogram").expect("url"),
                        Url::parse("lib://flowstdlib/charts/time_series").expect("url"),
                    ],
                    expanded: false,
                },
                library_panel::CategoryEntry {
                    name: "control".into(),
                    function_urls: vec![
                        Url::parse("lib://flowstdlib/control/compare_switch").expect("url"),
                        Url::parse("lib://flowstdlib/control/index").expect("url"),
                        Url::parse("lib://flowstdlib/control/join").expect("url"),
                        Url::parse("lib://flowstdlib/control/route").expect("url"),
                        Url::parse("lib://flowstdlib/control/select").expect("url"),
                        Url::parse("lib://flowstdlib/control/tap").expect("url"),
                    ],
                    expanded: false,
                },
                library_panel::CategoryEntry {
                    name: "data".into(),
                    function_urls: vec![
                        Url::parse("lib://flowstdlib/data/accumulate").expect("url"),
                        Url::parse("lib://flowstdlib/data/append").expect("url"),
                        Url::parse("lib://flowstdlib/data/count").expect("url"),
                        Url::parse("lib://flowstdlib/data/duplicate").expect("url"),
                        Url::parse("lib://flowstdlib/data/info").expect("url"),
                        Url::parse("lib://flowstdlib/data/sort").expect("url"),
                        Url::parse("lib://flowstdlib/data/split").expect("url"),
                        Url::parse("lib://flowstdlib/data/zip").expect("url"),
                    ],
                    expanded: false,
                },
                library_panel::CategoryEntry {
                    name: "fmt".into(),
                    function_urls: vec![
                        Url::parse("lib://flowstdlib/fmt/reverse").expect("url"),
                        Url::parse("lib://flowstdlib/fmt/to_json").expect("url"),
                        Url::parse("lib://flowstdlib/fmt/to_string").expect("url"),
                    ],
                    expanded: false,
                },
                library_panel::CategoryEntry {
                    name: "math".into(),
                    function_urls: vec![
                        Url::parse("lib://flowstdlib/math/add").expect("url"),
                        Url::parse("lib://flowstdlib/math/compare").expect("url"),
                        Url::parse("lib://flowstdlib/math/divide").expect("url"),
                        Url::parse("lib://flowstdlib/math/multiply").expect("url"),
                        Url::parse("lib://flowstdlib/math/sqrt").expect("url"),
                        Url::parse("lib://flowstdlib/math/subtract").expect("url"),
                    ],
                    expanded: true,
                },
                library_panel::CategoryEntry {
                    name: "matrix".into(),
                    function_urls: vec![
                        Url::parse("lib://flowstdlib/matrix/compose_matrix").expect("url"),
                        Url::parse("lib://flowstdlib/matrix/duplicate_rows").expect("url"),
                        Url::parse("lib://flowstdlib/matrix/multiply_row").expect("url"),
                        Url::parse("lib://flowstdlib/matrix/transpose").expect("url"),
                    ],
                    expanded: false,
                },
            ],
            expanded: true,
        }],
    }
}

/// Load a real flow definition from flowr/examples/ and build a `FlowEdit` app.
fn load_example_flow(example_name: &str) -> (FlowEdit, window::Id) {
    // Ensure default lib dir is on FLOW_LIB_PATH so lib:// URLs resolve.
    if let Some(default_lib) = flowcore::dirs::lib_dir() {
        if default_lib.exists() {
            let current = std::env::var("FLOW_LIB_PATH").unwrap_or_default();
            let default_str = default_lib.to_string_lossy();
            if !current.contains(default_str.as_ref()) {
                let new_val = if current.is_empty() {
                    default_str.to_string()
                } else {
                    format!("{current},{default_str}")
                };
                std::env::set_var("FLOW_LIB_PATH", new_val);
            }
        }
    }

    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("project root")
        .join("flowr")
        .join("examples")
        .join(example_name)
        .join("root.toml");

    let flow = file_ops::load_flow(&path).unwrap_or_else(|e| {
        panic!("Could not load {example_name} flow: {e}");
    });

    let win_id = window::Id::unique();
    let win_state = WindowState {
        is_root: true,
        ..Default::default()
    };

    let app = FlowEdit {
        windows: HashMap::from([(win_id, win_state)]),
        root_flow: flow,
        root_window: Some(win_id),
        focused_window: Some(win_id),
        library_tree: full_library_tree(),
        ..Default::default()
    };
    (app, win_id)
}

#[test]
#[ignore = "screenshot tests only run via make screenshots"]
fn screenshot_startup() {
    force_tiny_skia();
    let win_id = window::Id::unique();
    let win_state = WindowState {
        is_root: true,
        ..Default::default()
    };
    let app = FlowEdit {
        windows: HashMap::from([(win_id, win_state)]),
        root_window: Some(win_id),
        focused_window: Some(win_id),
        library_tree: full_library_tree(),
        ..Default::default()
    };
    let view = app.view(win_id);
    let mut ui = simulator(view);
    let snapshot = ui
        .snapshot(&iced::Theme::CatppuccinMocha)
        .expect("snapshot");
    assert!(snapshot
        .matches_image(screenshot_path("flowedit-startup"))
        .expect("write/compare"));
}

#[test]
#[ignore = "screenshot tests only run via make screenshots"]
fn screenshot_fibonacci() {
    force_tiny_skia();
    let (app, win_id) = load_example_flow("fibonacci");
    let view = app.view(win_id);
    let mut ui = simulator(view);
    let snapshot = ui
        .snapshot(&iced::Theme::CatppuccinMocha)
        .expect("snapshot");
    assert!(snapshot
        .matches_image(screenshot_path("flowedit-fibonacci"))
        .expect("write/compare"));
}

#[test]
#[ignore = "screenshot tests only run via make screenshots"]
fn screenshot_mandlebrot() {
    force_tiny_skia();
    let (app, win_id) = load_example_flow("mandlebrot");
    let view = app.view(win_id);
    let mut ui = simulator(view);
    let snapshot = ui
        .snapshot(&iced::Theme::CatppuccinMocha)
        .expect("snapshot");
    assert!(snapshot
        .matches_image(screenshot_path("flowedit-mandlebrot"))
        .expect("write/compare"));
}

#[test]
#[ignore = "screenshot tests only run via make screenshots"]
fn screenshot_node_selected() {
    force_tiny_skia();
    let (mut app, win_id) = load_example_flow("fibonacci");
    if let Some(ws) = app.windows.get_mut(&win_id) {
        ws.selected_node = Some(0);
    }
    let view = app.view(win_id);
    let mut ui = simulator(view);
    let snapshot = ui
        .snapshot(&iced::Theme::CatppuccinMocha)
        .expect("snapshot");
    assert!(snapshot
        .matches_image(screenshot_path("flowedit-node-selected"))
        .expect("write/compare"));
}

#[test]
#[ignore = "screenshot tests only run via make screenshots"]
fn screenshot_metadata_panel() {
    force_tiny_skia();
    let (mut app, win_id) = load_example_flow("fibonacci");
    if let Some(ws) = app.windows.get_mut(&win_id) {
        ws.show_metadata = true;
    }
    let view = app.view(win_id);
    let mut ui = simulator(view);
    let snapshot = ui
        .snapshot(&iced::Theme::CatppuccinMocha)
        .expect("snapshot");
    assert!(snapshot
        .matches_image(screenshot_path("flowedit-metadata-panel"))
        .expect("write/compare"));
}
