#![allow(missing_docs)]

use serial_test::serial;

/// End-to-end test: `flowrcli --delegate` delegates a sub-flow to a second
/// `flowrcli` instance (no manifest, coordinator-only mode) acting as a peer.
///
/// 1. Compiles the mandlebrot example
/// 2. Starts `flowrcli` (no manifest) as the peer coordinator
/// 3. Runs `flowrcli --delegate` with the mandlebrot manifest
/// 4. Verifies the output PNG matches the expected file
/// 5. Confirms delegation happened remotely via log
#[cfg_attr(target_os = "windows", ignore)]
#[test]
#[serial]
#[allow(clippy::too_many_lines)]
fn test_delegate_to_peer_flowrcli() {
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::Duration;

    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().expect("Could not find project root");

    let example_dir = project_root
        .join("flowr")
        .join("examples")
        .join("mandlebrot");

    // Compile the mandlebrot example
    let compile_status = Command::new("flowc")
        .args(["-d", "-g", "-c", "-O", "-r", "flowrcli"])
        .arg(example_dir.to_str().expect("path"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("Could not run flowc");
    assert!(compile_status.success(), "flowc compilation failed");

    // Use the binary from the current cargo build (same target directory as this test).
    // This ensures coverage instrumentation is captured when running under cargo llvm-cov.
    let flowrcli = env!("CARGO_BIN_EXE_flowrcli");

    // Start a second flowrcli as a peer coordinator (no manifest = coordinator-only mode)
    let mut peer = Command::new(flowrcli)
        .args(["-v", "info"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Could not spawn flowrcli peer");

    // Wait for the peer to start and advertise via mDNS
    thread::sleep(Duration::from_secs(8));

    // Create a temp file for the output
    let output_file = std::env::temp_dir().join("delegate_to_peer_test.png");

    // Run flowrcli --delegate
    let stderr_log = std::env::temp_dir().join("delegate_to_peer_test_stderr.log");
    let mut coordinator = Command::new(flowrcli)
        .args([
            "-n",
            "-v",
            "info",
            "--delegate",
            "manifest.json",
            "--",
            output_file.to_str().expect("temp path"),
            "[20,15]",
            "[[-1.20,0.35],[-1,0.20]]",
        ])
        .current_dir(
            example_dir
                .canonicalize()
                .expect("Could not canonicalize path"),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr({
            let f = std::fs::File::create(&stderr_log).expect("Could not create stderr log file");
            Stdio::from(f)
        })
        .spawn()
        .expect("Could not spawn flowrcli --delegate");

    // Wait with timeout
    let deadline = std::time::Instant::now() + Duration::from_mins(2);
    let exit_status = loop {
        if std::time::Instant::now() > deadline {
            coordinator.kill().ok();
            peer.kill().ok();
            coordinator.wait().ok();
            peer.wait().ok();
            let stderr_output = std::fs::read_to_string(&stderr_log).unwrap_or_default();
            panic!(
                "flowrcli --delegate did not finish within 2 minutes.\nstderr:\n{stderr_output}"
            );
        }
        match coordinator.try_wait().expect("Could not check coordinator") {
            Some(status) => break status,
            None => thread::sleep(Duration::from_secs(1)),
        }
    };

    // Read stderr to verify remote delegation
    let stderr_output = std::fs::read_to_string(&stderr_log).unwrap_or_default();

    // Clean up peer
    peer.kill().ok();
    peer.wait().ok();

    assert!(
        exit_status.success(),
        "flowrcli --delegate failed.\nstderr:\n{stderr_output}"
    );

    // Verify the output was delegated to a remote peer
    assert!(
        stderr_output.contains("will be executed on remote peer")
            || stderr_output.contains("delegate remotely"),
        "Expected remote delegation in log.\nstderr:\n{stderr_output}"
    );

    // Verify output file matches expected
    let expected_file = example_dir.join("expected.file");
    let expected = std::fs::read(&expected_file).expect("Could not read expected.file");
    let actual = std::fs::read(&output_file).expect("Could not read output file");

    assert_eq!(
        expected,
        actual,
        "Delegated output does not match expected.file (expected {} bytes, got {} bytes)",
        expected.len(),
        actual.len()
    );

    // Clean up
    std::fs::remove_file(&output_file).ok();
    std::fs::remove_file(&stderr_log).ok();

    // Allow mDNS goodbye packets to propagate
    thread::sleep(Duration::from_secs(3));
}
