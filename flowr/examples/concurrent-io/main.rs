//! Test flow for concurrent IO — stdout should appear while readline is blocking.
//!
//! This flow has two parallel paths from args:
//! - Path 1: prints the first arg to stdout (fast, non-blocking)
//! - Path 2: waits for readline input, then prints it to stderr
//!
//! The test verifies that stdout output appears BEFORE stdin input is provided.
//! Currently this fails because the coordinator serializes context function
//! communication through a single ZMQ socket.
fn main() {
    utilities::run_example(file!(), "flowrgui", false, true);
}

#[cfg(test)]
mod test {
    use std::io::{BufRead, BufReader, Write};
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::sync::mpsc;
    use std::time::Duration;

    #[ignore = "Blocked by #2918 — stdout blocked while readline is pending"]
    #[test]
    fn test_concurrent_stdout_while_readline_pending() {
        let example_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("examples")
            .join("concurrent-io");

        utilities::compile_example(&example_dir, "flowrcli");

        let mut child = Command::new("flowrcli")
            .args(["--native", "manifest.json"])
            .current_dir(
                example_dir
                    .canonicalize()
                    .expect("Could not canonicalize path"),
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not spawn flowrcli");

        let stdout = child.stdout.take().expect("Could not get stdout");

        // Read stdout on a background thread with a channel to report results
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) > 0 {
                let _ = tx.send(line);
            }
        });

        // Wait up to 5 seconds for stdout — it should appear within ~1s
        // because printing args is non-blocking and has no dependency on readline
        let stdout_result = rx.recv_timeout(Duration::from_secs(5));

        assert!(
            stdout_result.is_ok(),
            "Stdout should have received output while readline is pending, \
             but it was blocked. This confirms the concurrent IO bug (#2918)."
        );

        // Send stdin to unblock readline and let the flow complete
        if let Some(mut stdin) = child.stdin.take() {
            let _ = writeln!(stdin, "test input");
        }

        let _ = child.wait();
    }
}
