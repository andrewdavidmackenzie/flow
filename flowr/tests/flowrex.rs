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

/// End-to-end test with a real flowrex process as peer coordinator.
/// Starts flowrex, discovers its peer-coordinator service via mDNS,
/// submits a sub-flow, and verifies boundary outputs.
#[cfg_attr(target_os = "windows", ignore)]
#[test]
#[serial]
#[allow(clippy::too_many_lines)]
fn flowrex_peer_coordinator_end_to_end() {
    use flowcore::model::flow_manifest::{FlowInfo, FlowManifest};
    use flowcore::model::input::{Input, InputInitializer};
    use flowcore::model::metadata::MetaData;
    use flowcore::model::output_connection::{OutputConnection, Source};
    use flowcore::model::runtime_function::RuntimeFunction;
    use flowrlib::peer_client::PeerClient;
    use flowrlib::peer_discovery::discover_peer_coordinators;
    use std::process::{Command as ProcessCommand, Stdio};
    use std::thread;
    use std::time::Duration;

    // Start flowrex as a peer coordinator
    let mut flowrex = ProcessCommand::new("flowrex")
        .args(["--threads", "0", "-v", "info"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Could not spawn flowrex");

    // Wait for flowrex to start and advertise its peer-coordinator service
    thread::sleep(Duration::from_secs(3));

    // Discover the peer coordinator
    let peers = discover_peer_coordinators(Duration::from_secs(5), None).expect("discovery failed");

    if peers.is_empty() {
        // Clean up and skip — mDNS may not be working in this environment
        flowrex.kill().ok();
        flowrex.wait().ok();
        thread::sleep(Duration::from_secs(3));
        eprintln!("No peer coordinators discovered — skipping test");
        return;
    }

    let peer_address = peers.first().expect("no peers").clone();

    // Build a sub-flow: add(7,3)=10 with boundary output to #10:0
    let mut manifest = FlowManifest::new(MetaData::default());
    manifest.add_flow_info(FlowInfo {
        process_id: 0,
        parent_id: None,
        sub_flow_ids: vec![],
        #[cfg(feature = "debugger")]
        name: "subflow".into(),
        #[cfg(feature = "debugger")]
        route: "/subflow".into(),
    });

    let mut func = RuntimeFunction::new(
        #[cfg(feature = "debugger")]
        "add",
        #[cfg(feature = "debugger")]
        "/subflow/add",
        "lib://flowstdlib/math/add",
        vec![
            Input::new(
                #[cfg(feature = "debugger")]
                "i1",
                0,
                false,
                Some(InputInitializer::Once(serde_json::json!(7))),
                None,
            ),
            Input::new(
                #[cfg(feature = "debugger")]
                "i2",
                0,
                false,
                Some(InputInitializer::Once(serde_json::json!(3))),
                None,
            ),
        ],
        1,
        0,
        &[OutputConnection::new(
            Source::default(),
            10,
            0,
            0,
            false,
            String::new(),
            #[cfg(feature = "debugger")]
            String::new(),
        )],
        false,
    );
    let dummy_url = url::Url::parse("file:///dummy/manifest.json").expect("URL");
    func.set_implementation_url(&dummy_url).expect("set URL");
    manifest.add_function(func);

    // Connect to the peer and submit the sub-flow
    let zmq_context = zmq::Context::new();
    let client = PeerClient::connect(&zmq_context, &peer_address).expect("connect failed");

    let outputs = client
        .submit_subflow(manifest, vec![])
        .expect("submit failed");

    // Verify boundary output: add(7,3) = 10
    assert!(
        !outputs.is_empty(),
        "Should have boundary outputs from flowrex peer"
    );
    assert_eq!(
        outputs.first().map(|o| &o.value),
        Some(&serde_json::json!(10))
    );

    // Clean up
    drop(client);
    flowrex.kill().ok();
    flowrex.wait().ok();

    // Allow mDNS goodbye packets to propagate
    thread::sleep(Duration::from_secs(3));
}

/// Test `delegate_subflow` with a real flowrex peer coordinator.
#[cfg_attr(target_os = "windows", ignore)]
#[test]
#[serial]
#[allow(clippy::too_many_lines)]
fn delegate_subflow_to_peer() {
    use flowcore::model::flow_manifest::{FlowInfo, FlowManifest};
    use flowcore::model::input::{Input, InputInitializer};
    use flowcore::model::metadata::MetaData;
    use flowcore::model::output_connection::{OutputConnection, Source};
    use flowcore::model::runtime_function::RuntimeFunction;
    use flowrlib::delegation::delegate_subflow;
    use std::process::{Command as ProcessCommand, Stdio};
    use std::thread;
    use std::time::Duration;

    // Start flowrex as peer coordinator
    let mut flowrex = ProcessCommand::new("flowrex")
        .args(["--threads", "0", "-v", "info"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Could not spawn flowrex");

    thread::sleep(Duration::from_secs(3));

    // Build manifest with root flow containing a child sub-flow
    let mut manifest = FlowManifest::new(MetaData::default());
    manifest.add_flow_info(FlowInfo {
        process_id: 0,
        parent_id: None,
        sub_flow_ids: vec![1],
        #[cfg(feature = "debugger")]
        name: "root".into(),
        #[cfg(feature = "debugger")]
        route: "/root".into(),
    });
    manifest.add_flow_info(FlowInfo {
        process_id: 1,
        parent_id: Some(0),
        sub_flow_ids: vec![],
        #[cfg(feature = "debugger")]
        name: "child".into(),
        #[cfg(feature = "debugger")]
        route: "/root/child".into(),
    });

    // Function #10 in root
    manifest.add_function(RuntimeFunction::new(
        #[cfg(feature = "debugger")]
        "target",
        #[cfg(feature = "debugger")]
        "/root/target",
        "lib://flowstdlib/math/add",
        vec![Input::new(
            #[cfg(feature = "debugger")]
            "in",
            0,
            false,
            None,
            None,
        )],
        10,
        0,
        &[],
        false,
    ));

    // Function #20 in child — add(7,3)=10, outputs to #10:0
    let mut func20 = RuntimeFunction::new(
        #[cfg(feature = "debugger")]
        "add",
        #[cfg(feature = "debugger")]
        "/root/child/add",
        "lib://flowstdlib/math/add",
        vec![
            Input::new(
                #[cfg(feature = "debugger")]
                "i1",
                0,
                false,
                Some(InputInitializer::Once(serde_json::json!(7))),
                None,
            ),
            Input::new(
                #[cfg(feature = "debugger")]
                "i2",
                0,
                false,
                Some(InputInitializer::Once(serde_json::json!(3))),
                None,
            ),
        ],
        20,
        1,
        &[OutputConnection::new(
            Source::default(),
            10,
            0,
            0,
            false,
            String::new(),
            #[cfg(feature = "debugger")]
            String::new(),
        )],
        false,
    );
    let dummy_url = url::Url::parse("file:///dummy/manifest.json").expect("URL");
    func20.set_implementation_url(&dummy_url).expect("set URL");
    manifest.add_function(func20);

    // Delegate child flow #1 to the peer
    let result = delegate_subflow(&manifest, 1, None, vec![]);

    // Clean up flowrex
    flowrex.kill().ok();
    flowrex.wait().ok();

    let result = result.expect("delegate_subflow failed");
    let delegation = result.expect("Should have delegated to a peer");
    assert_eq!(delegation.flow_id, 1);
    assert!(
        !delegation.boundary_outputs.is_empty(),
        "Should have boundary outputs"
    );
    assert_eq!(
        delegation.boundary_outputs.first().map(|o| &o.value),
        Some(&serde_json::json!(10)),
        "Boundary output should be 10 (7+3)"
    );

    // Allow mDNS goodbye packets to propagate
    thread::sleep(Duration::from_secs(3));
}

/// End-to-end test: `flowrcli --delegate` delegates a sub-flow to a running
/// flowrex peer coordinator, producing correct output.
///
/// 1. Compiles the mandlebrot example
/// 2. Starts flowrex as a peer coordinator (--threads 0)
/// 3. Runs flowrcli --delegate with the mandlebrot manifest
/// 4. Verifies the output PNG matches the expected file
/// 5. Confirms delegation happened remotely (log mentions "remote peer")
///
/// NOTE: This test is ignored because `flowrcli --delegate` discovers peers
/// via mDNS which can find stale entries from previous tests, causing hangs.
/// Run manually with: `cargo test --test flowrex test_delegate_to_remote -- --ignored`
#[cfg_attr(target_os = "windows", ignore)]
#[test]
#[serial]
#[ignore = "mDNS stale entries from prior tests cause hangs — run manually"]
#[allow(clippy::too_many_lines)]
fn test_delegate_to_remote_flowrex() {
    use std::io::Read;
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

    // Start flowrex as peer coordinator (no executor threads)
    let mut flowrex = Command::new("flowrex")
        .args(["--threads", "0", "-v", "info"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Could not spawn flowrex");

    // Wait for mDNS from previous tests to clear and for
    // this flowrex to advertise its peer-coordinator service.
    thread::sleep(Duration::from_secs(8));

    // Create a temp file for the output
    let output_file = std::env::temp_dir().join("flowrex_delegate_test.png");

    // Run flowrcli --delegate
    let mut coordinator = Command::new("flowrcli")
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
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Could not spawn flowrcli");

    // Wait with timeout
    let deadline = std::time::Instant::now() + Duration::from_mins(2);
    let exit_status = loop {
        if std::time::Instant::now() > deadline {
            // Read stderr before killing for debugging
            let mut stderr_output = String::new();
            if let Some(mut err) = coordinator.stderr.take() {
                err.read_to_string(&mut stderr_output).ok();
            }
            coordinator.kill().ok();
            flowrex.kill().ok();
            coordinator.wait().ok();
            flowrex.wait().ok();
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
    let mut stderr_output = String::new();
    if let Some(mut err) = coordinator.stderr.take() {
        err.read_to_string(&mut stderr_output).ok();
    }

    // Clean up flowrex
    flowrex.kill().ok();
    flowrex.wait().ok();

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

    // Allow mDNS goodbye packets to propagate
    thread::sleep(Duration::from_secs(3));
}
