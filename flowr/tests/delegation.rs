#![allow(missing_docs)]
// Force recompile: diagnostic logging for CI flake investigation

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
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::{Duration, Instant};

    // Timeout for waiting for the coordinator process to exit.
    // The coordinator itself takes ~5-30s on CI (mDNS discovery + flow
    // execution + shutdown). A 60s timeout provides ample headroom while
    // still catching genuine hangs much faster than the CI step timeout.
    const COORDINATOR_TIMEOUT_SECS: u64 = 60;

    let test_start = Instant::now();

    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().expect("Could not find project root");

    let example_dir = project_root
        .join("flowr")
        .join("examples")
        .join("mandlebrot");

    // Compile the mandlebrot example
    eprintln!(
        "[delegation-test +{:.1}s] Compiling mandlebrot example...",
        test_start.elapsed().as_secs_f64()
    );
    let compile_status = Command::new("flowc")
        .args(["-d", "-g", "-c", "-O", "-r", "flowrcli"])
        .arg(example_dir.to_str().expect("path"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("Could not run flowc");
    assert!(compile_status.success(), "flowc compilation failed");
    eprintln!(
        "[delegation-test +{:.1}s] Compilation complete",
        test_start.elapsed().as_secs_f64()
    );

    // Use the binary from the current cargo build (same target directory as this test).
    // This ensures coverage instrumentation is captured when running under cargo llvm-cov.
    let flowrcli = env!("CARGO_BIN_EXE_flowrcli");
    eprintln!("[delegation-test] Using flowrcli binary: {flowrcli}");

    // Capture peer stderr for diagnostics
    let peer_stderr_log = std::env::temp_dir().join("delegate_to_peer_test_peer_stderr.log");

    // Start a second flowrcli as a peer coordinator (no manifest = coordinator-only mode)
    eprintln!(
        "[delegation-test +{:.1}s] Spawning peer coordinator...",
        test_start.elapsed().as_secs_f64()
    );
    let mut peer = Command::new(flowrcli)
        .args(["-v", "info"])
        .stdout(Stdio::piped())
        .stderr({
            let f = std::fs::File::create(&peer_stderr_log)
                .expect("Could not create peer stderr log file");
            Stdio::from(f)
        })
        .spawn()
        .expect("Could not spawn flowrcli peer");
    eprintln!(
        "[delegation-test +{:.1}s] Peer spawned with PID {}",
        test_start.elapsed().as_secs_f64(),
        peer.id()
    );

    // Wait for the peer to print "ready" on stdout instead of using a fixed sleep.
    // Use a background thread with a timeout so the test doesn't hang forever if
    // the peer fails to start.
    let peer_stdout = peer.stdout.take().expect("Could not take peer stdout");
    eprintln!(
        "[delegation-test +{:.1}s] Waiting for peer 'ready' signal (timeout=30s)...",
        test_start.elapsed().as_secs_f64()
    );
    let peer_wait_start = Instant::now();
    let ready_handle = thread::spawn(move || {
        let mut reader = BufReader::new(peer_stdout);
        let mut ready_line = String::new();
        let bytes_read = reader.read_line(&mut ready_line).ok().unwrap_or(0);
        (bytes_read, ready_line)
    });
    let ready_result = ready_handle.join();
    let (bytes_read, ready_line) = match ready_result {
        Ok(result) if peer_wait_start.elapsed() < Duration::from_secs(30) => result,
        _ => {
            peer.kill().ok();
            peer.wait().ok();
            panic!(
                "Peer did not report ready within 30s (waited {:.1}s)",
                peer_wait_start.elapsed().as_secs_f64()
            );
        }
    };
    assert!(
        bytes_read > 0 && ready_line.trim() == "ready",
        "Peer did not report ready. First stdout line: {ready_line:?}"
    );
    eprintln!(
        "[delegation-test +{:.1}s] Peer is ready",
        test_start.elapsed().as_secs_f64()
    );

    // Give mDNS a moment to propagate after the peer is ready
    let mdns_wait = Duration::from_secs(3);
    eprintln!(
        "[delegation-test +{:.1}s] Waiting {:.0}s for mDNS propagation...",
        test_start.elapsed().as_secs_f64(),
        mdns_wait.as_secs_f64()
    );
    thread::sleep(mdns_wait);

    // Create a temp file for the output
    let output_file = std::env::temp_dir().join("delegate_to_peer_test.png");

    // Run flowrcli --delegate
    let stderr_log = std::env::temp_dir().join("delegate_to_peer_test_stderr.log");
    eprintln!(
        "[delegation-test +{:.1}s] Spawning coordinator with --delegate...",
        test_start.elapsed().as_secs_f64()
    );
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
    eprintln!(
        "[delegation-test +{:.1}s] Coordinator spawned with PID {}",
        test_start.elapsed().as_secs_f64(),
        coordinator.id()
    );

    eprintln!(
        "[delegation-test +{:.1}s] Waiting up to {COORDINATOR_TIMEOUT_SECS}s for coordinator...",
        test_start.elapsed().as_secs_f64()
    );
    let deadline = Instant::now() + Duration::from_secs(COORDINATOR_TIMEOUT_SECS);
    let mut last_log = Instant::now();
    let exit_status = loop {
        if Instant::now() > deadline {
            let elapsed = test_start.elapsed().as_secs_f64();
            let stderr_output = std::fs::read_to_string(&stderr_log).unwrap_or_default();
            let peer_stderr_output = std::fs::read_to_string(&peer_stderr_log).unwrap_or_default();
            eprintln!("[delegation-test +{elapsed:.1}s] TIMEOUT — killing processes");
            coordinator.kill().ok();
            peer.kill().ok();
            coordinator.wait().ok();
            peer.wait().ok();
            panic!(
                "flowrcli --delegate did not finish within {COORDINATOR_TIMEOUT_SECS}s.\n\
                 === coordinator stderr ===\n{stderr_output}\n\
                 === peer stderr ===\n{peer_stderr_output}"
            );
        }
        if let Some(status) = coordinator.try_wait().expect("Could not check coordinator") {
            eprintln!(
                "[delegation-test +{:.1}s] Coordinator exited with status: {status}",
                test_start.elapsed().as_secs_f64()
            );
            break status;
        }
        // Log every 10 seconds while waiting
        if last_log.elapsed() >= Duration::from_secs(10) {
            eprintln!(
                "[delegation-test +{:.1}s] Still waiting for coordinator...",
                test_start.elapsed().as_secs_f64()
            );
            last_log = Instant::now();
        }
        thread::sleep(Duration::from_secs(1));
    };

    // Read stderr to verify remote delegation
    let stderr_output = std::fs::read_to_string(&stderr_log).unwrap_or_default();
    let peer_stderr_output = std::fs::read_to_string(&peer_stderr_log).unwrap_or_default();

    // Always print both logs for CI diagnostics
    eprintln!(
        "[delegation-test +{:.1}s] === coordinator stderr ({} bytes) ===",
        test_start.elapsed().as_secs_f64(),
        stderr_output.len()
    );
    for line in stderr_output.lines() {
        eprintln!("[coordinator] {line}");
    }
    eprintln!(
        "[delegation-test +{:.1}s] === peer stderr ({} bytes) ===",
        test_start.elapsed().as_secs_f64(),
        peer_stderr_output.len()
    );
    for line in peer_stderr_output.lines() {
        eprintln!("[peer] {line}");
    }

    // Clean up peer
    peer.kill().ok();
    peer.wait().ok();

    assert!(
        exit_status.success(),
        "flowrcli --delegate failed with status {exit_status}.\n\
         === coordinator stderr ===\n{stderr_output}\n\
         === peer stderr ===\n{peer_stderr_output}"
    );

    // Verify the output was delegated to a remote peer
    assert!(
        stderr_output.contains("will be executed on remote peer")
            || stderr_output.contains("delegate remotely"),
        "Expected remote delegation in log.\n\
         === coordinator stderr ===\n{stderr_output}\n\
         === peer stderr ===\n{peer_stderr_output}"
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
    std::fs::remove_file(&peer_stderr_log).ok();

    eprintln!(
        "[delegation-test +{:.1}s] Test complete",
        test_start.elapsed().as_secs_f64()
    );

    // Allow mDNS goodbye packets to propagate
    thread::sleep(Duration::from_secs(3));
}
