#![allow(missing_docs)]

use serial_test::serial;

#[cfg_attr(target_os = "windows", ignore)]
#[test]
#[serial]
fn test_hello_world_flowrex_example() {
    let source = std::path::PathBuf::from("flowr")
        .join("examples")
        .join("hello-world")
        .join("main.rs");
    utilities::test_example(source.to_str().expect("path"), "flowrcli", true, true);
}

/// Test that flowrex can join mid-run and execute jobs for a more demanding flow.
/// Starts the coordinator with 0 local executor threads running the fibonacci example,
/// then starts flowrex which discovers the coordinator and executes all jobs.
#[cfg_attr(target_os = "windows", ignore)]
#[test]
#[serial]
fn test_fibonacci_with_flowrex_mid_run() {
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::Duration;

    // Change to the project root
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().expect("Could not find project root");

    let example_dir = project_root
        .join("flowr")
        .join("examples")
        .join("fibonacci");

    // Compile the example first
    let compile_status = Command::new("flowc")
        .args(["-d", "-g", "-c", "-O", "-r", "flowrcli"])
        .arg(example_dir.to_str().expect("path"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("Could not run flowc to compile example");
    assert!(compile_status.success(), "flowc compilation failed");

    // Start coordinator with 0 threads and metrics — no local executors
    let mut coordinator = Command::new("flowrcli")
        .args(["--threads", "0", "-m", "manifest.json"])
        .current_dir(
            example_dir
                .canonicalize()
                .expect("Could not canonicalize path"),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Could not spawn coordinator");

    let coordinator_pid = coordinator.id();

    // Wait for the coordinator to start and advertise mDNS services.
    // mDNS advertisement can take a few seconds on some platforms.
    thread::sleep(Duration::from_secs(3));

    // Start flowrex — it should discover the coordinator and start executing jobs
    let mut flowrex = Command::new("flowrex")
        .args(["--threads", "2"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Could not spawn flowrex");

    let flowrex_pid = flowrex.id();

    // Wait for coordinator with a timeout to avoid hanging CI
    let deadline = std::time::Instant::now() + Duration::from_mins(2);
    let exit_status = loop {
        if std::time::Instant::now() > deadline {
            coordinator.kill().ok();
            flowrex.kill().ok();
            coordinator.wait().ok();
            flowrex.wait().ok();
            panic!("Coordinator did not finish within 60 seconds");
        }
        match coordinator.try_wait().expect("Could not check coordinator") {
            Some(status) => break status,
            None => thread::sleep(Duration::from_secs(1)),
        }
    };

    // Read stdout before killing flowrex
    let mut stdout_bytes = Vec::new();
    if let Some(mut out) = coordinator.stdout.take() {
        std::io::Read::read_to_end(&mut out, &mut stdout_bytes).ok();
    }

    // Kill flowrex
    flowrex.kill().ok();
    flowrex.wait().ok();

    // Verify the coordinator ran successfully
    assert!(
        exit_status.success(),
        "Coordinator failed with status: {exit_status}"
    );

    // Verify fibonacci output matches expected first lines
    let stdout = String::from_utf8_lossy(&stdout_bytes);
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines.len() > 5,
        "Expected fibonacci output lines, got {}: {stdout}",
        lines.len()
    );
    // Check the known fibonacci sequence start
    assert_eq!(lines.first().copied(), Some("1"), "First fibonacci number");
    assert_eq!(lines.get(1).copied(), Some("2"), "Second fibonacci number");
    assert_eq!(lines.get(2).copied(), Some("3"), "Third fibonacci number");
    assert_eq!(lines.get(3).copied(), Some("5"), "Fourth fibonacci number");
    assert_eq!(lines.get(4).copied(), Some("8"), "Fifth fibonacci number");

    // Verify flowrex executor IDs appear in metrics (format: "pid-N:count")
    let flowrex_prefix = format!("{flowrex_pid}-");
    let coordinator_prefix = format!("{coordinator_pid}-");
    assert!(
        stdout.contains(&flowrex_prefix),
        "Metrics should contain flowrex executor IDs (prefix '{flowrex_prefix}'), \
         but stdout was:\n{stdout}"
    );
    assert!(
        !stdout.contains(&coordinator_prefix),
        "With --threads 0, no coordinator executor IDs (prefix '{coordinator_prefix}') \
         should appear in metrics, but stdout was:\n{stdout}"
    );
}

/// Test that flowrex can execute WASM jobs fetched over HTTP from the coordinator.
/// Uses mandlebrot which has user-supplied WASM functions (`escapes.wasm`, `pixel_to_point.wasm`).
/// Starts the coordinator with 0 local executor threads, so ALL jobs (including WASM) must
/// be executed by flowrex. The coordinator's WASM HTTP server rewrites file:// URLs to
/// http:// so flowrex can fetch the WASM modules.
#[cfg_attr(target_os = "windows", ignore)]
#[test]
#[serial]
fn test_flowrex_executes_wasm_over_http() {
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::Duration;

    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().expect("Could not find project root");

    let example_dir = project_root
        .join("flowr")
        .join("examples")
        .join("mandlebrot");

    // Compile the example
    let compile_status = Command::new("flowc")
        .args(["-d", "-g", "-c", "-O", "-r", "flowrcli"])
        .arg(example_dir.to_str().expect("path"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("Could not run flowc to compile example");
    assert!(compile_status.success(), "flowc compilation failed");

    // Start coordinator with 0 local threads — ALL jobs go to flowrex
    let mut coordinator = Command::new("flowrcli")
        .args([
            "--threads",
            "0",
            "-m",
            "manifest.json",
            "--",
            "/dev/null",
            "[20,15]",
            "[[-1.20,0.35],[-1,0.20]]",
        ])
        .current_dir(
            example_dir
                .canonicalize()
                .expect("Could not canonicalize path"),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Could not spawn coordinator");

    let coordinator_pid = coordinator.id();

    // Drain stdout and stderr in background threads to prevent pipe deadlocks
    let stdout_handle = {
        let out = coordinator.stdout.take().expect("stdout");
        thread::spawn(move || {
            let mut bytes = Vec::new();
            std::io::Read::read_to_end(&mut { out }, &mut bytes).ok();
            bytes
        })
    };
    let stderr_handle = {
        let err = coordinator.stderr.take().expect("stderr");
        thread::spawn(move || {
            let mut bytes = Vec::new();
            std::io::Read::read_to_end(&mut { err }, &mut bytes).ok();
            bytes
        })
    };

    // Wait for coordinator to advertise mDNS services
    thread::sleep(Duration::from_secs(3));

    // Start flowrex with 2 threads
    let mut flowrex = Command::new("flowrex")
        .args(["--threads", "2"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Could not spawn flowrex");

    let flowrex_pid = flowrex.id();

    // Wait for coordinator with a timeout
    let deadline = std::time::Instant::now() + Duration::from_mins(2);
    let exit_status = loop {
        if std::time::Instant::now() > deadline {
            coordinator.kill().ok();
            flowrex.kill().ok();
            coordinator.wait().ok();
            flowrex.wait().ok();
            panic!("Coordinator did not finish within 120 seconds");
        }
        match coordinator.try_wait().expect("Could not check coordinator") {
            Some(status) => break status,
            None => thread::sleep(Duration::from_secs(1)),
        }
    };

    let stdout_bytes = stdout_handle.join().unwrap_or_default();
    let stderr_bytes = stderr_handle.join().unwrap_or_default();
    let stderr = String::from_utf8_lossy(&stderr_bytes);

    // Kill flowrex
    flowrex.kill().ok();
    flowrex.wait().ok();

    assert!(
        exit_status.success(),
        "Coordinator failed with status: {exit_status}\nstderr: {stderr}"
    );

    let stdout = String::from_utf8_lossy(&stdout_bytes);

    // Verify flowrex executor IDs appear in metrics — proves WASM jobs ran remotely
    let flowrex_prefix = format!("{flowrex_pid}-");
    assert!(
        stdout.contains(&flowrex_prefix),
        "Metrics should contain flowrex executor IDs (prefix '{flowrex_prefix}'), \
         proving WASM was fetched over HTTP and executed remotely.\n\
         stdout:\n{stdout}\nstderr: {stderr}"
    );

    // Verify no coordinator executors (--threads 0)
    let coordinator_prefix = format!("{coordinator_pid}-");
    assert!(
        !stdout.contains(&coordinator_prefix),
        "With --threads 0, no coordinator executor IDs should appear.\n\
         stdout:\n{stdout}\nstderr: {stderr}"
    );

    // Allow mDNS goodbye packets to propagate
    thread::sleep(Duration::from_secs(3));
}
