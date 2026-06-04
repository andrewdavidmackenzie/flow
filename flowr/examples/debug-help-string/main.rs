//! Integration test for the debug help command using flowdb as a separate process
fn main() {}

#[cfg(test)]
#[cfg(feature = "debugger")]
mod test {
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_debug_help_string_example() {
        let crate_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let example_dir = crate_dir.join("examples/debug-help-string");

        let mut server = Command::new("flowrcli")
            .args(["--debugger", "--native", "manifest.json"])
            .current_dir(&example_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not spawn flowrcli");

        let stderr = server.stderr.as_mut().expect("Could not get stderr");
        let mut reader = BufReader::new(stderr);
        let mut port_line = String::new();
        reader
            .read_line(&mut port_line)
            .expect("Could not read port line");

        let port = port_line
            .split("localhost:")
            .nth(1)
            .and_then(|s| s.trim().parse::<u16>().ok())
            .unwrap_or_else(|| panic!("Could not parse debug port from: {port_line}"));

        let mut flowdb = Command::new("flowdb")
            .args(["--address", &format!("localhost:{port}")])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not spawn flowdb");

        let mut stdin = flowdb.stdin.take().expect("Could not get flowdb stdin");

        thread::sleep(Duration::from_secs(2));
        writeln!(stdin, "h").expect("Could not write h");
        thread::sleep(Duration::from_millis(500));
        writeln!(stdin, "e").expect("Could not write e");
        drop(stdin);

        let output = flowdb
            .wait_with_output()
            .expect("Could not wait for flowdb");

        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(
            stdout.contains("Debugger commands:"),
            "Help output not found in flowdb stdout:\n{stdout}"
        );
        assert!(
            stdout.contains("'b' | 'breakpoint'"),
            "Breakpoint help not found in flowdb stdout"
        );
        assert!(
            stdout.contains("Debugger is exiting"),
            "Exit message not found in flowdb stdout:\n{stdout}"
        );

        server.kill().expect("Could not kill flowrcli");
        server.wait().expect("Could not wait for flowrcli");
    }
}
