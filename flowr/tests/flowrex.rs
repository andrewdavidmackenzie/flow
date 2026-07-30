#![allow(missing_docs)]

#[cfg_attr(target_os = "windows", ignore)]
#[test]
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

    // Start coordinator with 0 threads — no local executors
    let coordinator = Command::new("flowrcli")
        .args(["--threads", "0", "manifest.json"])
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

    // Wait a moment for the coordinator to start and advertise services
    thread::sleep(Duration::from_secs(2));

    // Start flowrex — it should discover the coordinator and start executing jobs
    let mut flowrex = Command::new("flowrex")
        .args(["--threads", "2"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Could not spawn flowrex");

    // Wait for coordinator to finish (fibonacci is fast)
    let output = coordinator
        .wait_with_output()
        .expect("Could not wait for coordinator");

    // Kill flowrex
    flowrex.kill().ok();
    flowrex.wait().ok();

    // Verify the coordinator ran successfully
    assert!(
        output.status.success(),
        "Coordinator failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify fibonacci output contains expected values
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains('1') && stdout.contains('2'),
        "Expected fibonacci output, got: {stdout}"
    );
}
