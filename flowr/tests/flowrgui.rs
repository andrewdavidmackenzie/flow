#![allow(missing_docs)]

use serial_test::serial;

#[test]
#[serial]
#[cfg_attr(
    target_os = "windows",
    ignore = "winit event loop hangs in non-interactive Windows CI session (no WM_PAINT)"
)]
fn test_fibonacci_flowrgui_example() {
    utilities::test_example("flowr/examples/fibonacci/main.rs", "flowrgui", false, true);
}

#[cfg(feature = "debugger")]
#[test]
#[serial]
#[cfg_attr(
    target_os = "windows",
    ignore = "winit event loop hangs in non-interactive Windows CI session (no WM_PAINT)"
)]
fn test_fibonacci_flowrgui_debug() {
    let example_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/fibonacci");
    let mut session = utilities::DebugSession::start_with_runner(&example_dir, "flowrgui", &[]);
    session.send("c");
    std::thread::sleep(std::time::Duration::from_secs(3));
    session.send("e");
    let stdout = session.finish();

    assert!(
        stdout.contains("Flow has completed"),
        "No completion in flowrgui debug session:\n{stdout}"
    );
    assert!(
        stdout.contains("Debugger is exiting"),
        "No exit in flowrgui debug session:\n{stdout}"
    );
}
