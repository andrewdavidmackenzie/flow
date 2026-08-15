#![allow(clippy::indexing_slicing, clippy::unwrap_used, clippy::expect_used)]

//! Screenshot tests for `flowrgui`.
//!
//! These are ignored by default and only run via `make screenshots`
//! or `cargo test -- --ignored --test-threads=1`.

use super::*;

/// Force the tiny-skia backend for deterministic cross-platform screenshots.
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

/// Create a test GUI with a `Connected` coordinator state (dummy channels).
fn test_gui_connected() -> FlowrGui {
    let (sender, _rx) = tokio::sync::mpsc::channel(1);
    let (blocking_sender, _brx) = tokio::sync::mpsc::channel(1);
    let mut gui = test::test_gui();
    gui.coordinator_state = CoordinatorState::Connected(sender, blocking_sender);
    gui
}

#[test]
#[ignore = "screenshot tests only run via make screenshots"]
fn screenshot_startup() {
    use iced_test::simulator::simulator;
    force_tiny_skia();
    let gui = test::test_gui();
    let view = gui.view();
    let mut ui = simulator(view);
    let snapshot = ui
        .snapshot(&iced::Theme::CatppuccinMocha)
        .expect("snapshot");
    assert!(snapshot
        .matches_image(screenshot_path("flowrgui-startup"))
        .expect("write/compare"));
}

#[test]
#[ignore = "screenshot tests only run via make screenshots"]
fn screenshot_submitted_stdout() {
    use iced_test::simulator::simulator;
    force_tiny_skia();
    let mut gui = test_gui_connected();
    gui.submitted = true;
    gui.running = true;
    gui.submission_settings.flow_manifest_url = "flowr/examples/fibonacci".into();
    drop(
        gui.update(Message::CoordinatorSent(CoordinatorMessage::Stdout(
            "1\n1\n2\n3\n5\n8\n13\n21\n34\n55".into(),
        ))),
    );
    let view = gui.view();
    let mut ui = simulator(view);
    let snapshot = ui
        .snapshot(&iced::Theme::CatppuccinMocha)
        .expect("snapshot");
    assert!(snapshot
        .matches_image(screenshot_path("flowrgui-submitted"))
        .expect("write/compare"));
}

#[test]
#[ignore = "screenshot tests only run via make screenshots"]
fn screenshot_stderr_tab() {
    use iced_test::simulator::simulator;
    force_tiny_skia();
    let mut gui = test_gui_connected();
    gui.submitted = true;
    gui.running = true;
    drop(
        gui.update(Message::CoordinatorSent(CoordinatorMessage::Stderr(
            "INFO - Starting coordinator\nINFO - Flow loaded\nINFO - Executing flow".into(),
        ))),
    );
    drop(gui.update(Message::TabSelected(1)));
    let view = gui.view();
    let mut ui = simulator(view);
    let snapshot = ui
        .snapshot(&iced::Theme::CatppuccinMocha)
        .expect("snapshot");
    assert!(snapshot
        .matches_image(screenshot_path("flowrgui-stderr-tab"))
        .expect("write/compare"));
}

#[test]
#[ignore = "screenshot tests only run via make screenshots"]
fn screenshot_settings_panel() {
    use iced_test::simulator::simulator;
    force_tiny_skia();
    let mut gui = test_gui_connected();
    gui.active_panel = Some(PanelKind::Settings);
    let view = gui.view();
    let mut ui = simulator(view);
    let snapshot = ui
        .snapshot(&iced::Theme::CatppuccinMocha)
        .expect("snapshot");
    assert!(snapshot
        .matches_image(screenshot_path("flowrgui-settings"))
        .expect("write/compare"));
}

#[test]
#[ignore = "screenshot tests only run via make screenshots"]
fn screenshot_metrics_panel() {
    use flowcore::model::metrics::Metrics;
    use iced_test::simulator::simulator;
    force_tiny_skia();
    let mut gui = test_gui_connected();
    gui.active_panel = Some(PanelKind::Metrics);
    gui.submitted = true;
    let mut metrics = Metrics::new(3, 3);
    metrics.set_jobs_created(42);
    metrics.record_executor("100-0");
    metrics.record_executor("100-0");
    metrics.record_executor("100-1");
    metrics.record_executor("200-0");
    gui.last_metrics = Some(metrics);
    let view = gui.view();
    let mut ui = simulator(view);
    let snapshot = ui
        .snapshot(&iced::Theme::CatppuccinMocha)
        .expect("snapshot");
    assert!(snapshot
        .matches_image(screenshot_path("flowrgui-metrics"))
        .expect("write/compare"));
}
