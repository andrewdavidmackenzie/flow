#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used)]

//! Screenshot tests for `flowedit`.
//!
//! These are ignored by default and only run via `make screenshots`
//! or `cargo test -- --ignored --test-threads=1`. They generate PNG
//! screenshots into `assets/screenshots/` using the headless `tiny-skia`
//! renderer and compare against existing gold standard images.

use super::*;
use crate::library_panel::{self, LibraryTree};
use flowcore::model::connection::Connection;
use flowcore::model::flow_definition::FlowDefinition;
use flowcore::model::name::Name;
use flowcore::model::process_reference::ProcessReference;
use iced::window;
use iced_test::simulator::simulator;
use std::collections::{BTreeMap, HashMap};
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

/// Build the full flowstdlib library tree with all 6 categories.
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
                        Url::parse("lib://flowstdlib/data/enumerate").expect("url"),
                        Url::parse("lib://flowstdlib/data/info").expect("url"),
                        Url::parse("lib://flowstdlib/data/max").expect("url"),
                        Url::parse("lib://flowstdlib/data/min").expect("url"),
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

fn screenshot_app_with_flow(flow: FlowDefinition) -> (FlowEdit, window::Id) {
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

fn sample_flow() -> FlowDefinition {
    FlowDefinition {
        name: Name::from("fibonacci"),
        process_refs: vec![
            ProcessReference {
                alias: Name::from("add"),
                source: "lib://flowstdlib/math/add".into(),
                initializations: BTreeMap::new(),
                x: Some(100.0),
                y: Some(80.0),
                width: Some(180.0),
                height: Some(120.0),
            },
            ProcessReference {
                alias: Name::from("compare"),
                source: "lib://flowstdlib/control/compare_switch".into(),
                initializations: BTreeMap::new(),
                x: Some(350.0),
                y: Some(80.0),
                width: Some(180.0),
                height: Some(120.0),
            },
            ProcessReference {
                alias: Name::from("stdout"),
                source: "context://stdio/stdout".into(),
                initializations: BTreeMap::new(),
                x: Some(600.0),
                y: Some(80.0),
                width: Some(180.0),
                height: Some(120.0),
            },
            ProcessReference {
                alias: Name::from("const_1"),
                source: "lib://flowstdlib/data/append".into(),
                initializations: BTreeMap::new(),
                x: Some(100.0),
                y: Some(280.0),
                width: Some(180.0),
                height: Some(120.0),
            },
        ],
        connections: vec![
            Connection::new("add", "compare"),
            Connection::new_named("next value", "add", "stdout"),
            Connection::new("compare", "const_1"),
            Connection::new("const_1", "add"),
        ],
        ..FlowDefinition::default()
    }
}

#[test]
#[ignore = "screenshot tests only run via make screenshots"]
fn screenshot_startup() {
    force_tiny_skia();
    let (app, win_id) = screenshot_app_with_flow(FlowDefinition::default());
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
    let (app, win_id) = screenshot_app_with_flow(sample_flow());
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
fn screenshot_node_selected() {
    force_tiny_skia();
    let (mut app, win_id) = screenshot_app_with_flow(sample_flow());
    // Select the first node
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
    let flow = FlowDefinition {
        name: Name::from("fibonacci"),
        metadata: flowcore::model::metadata::MetaData {
            name: "fibonacci".into(),
            version: "1.0.0".into(),
            description: "Calculate the Fibonacci series".into(),
            authors: vec!["Andrew Mackenzie".into()],
        },
        ..FlowDefinition::default()
    };
    let (mut app, win_id) = screenshot_app_with_flow(flow);
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
