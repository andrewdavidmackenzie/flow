//! Integration test for the debug help command using flowrdb as a separate process
fn main() {}

#[cfg(test)]
#[cfg(feature = "debugger")]
mod test {
    use serial_test::serial;
    use utilities::DebugSession;

    #[test]
    #[serial]
    fn test_debug_help_string_example() {
        let example_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("examples")
            .join("debug-help-string");
        eprintln!(
            "[DEBUG] starting debug session for {}",
            example_dir.display()
        );
        let mut session = DebugSession::start(&example_dir, &[]);
        eprintln!("[DEBUG] session started, sleeping 3s before sending commands");
        std::thread::sleep(std::time::Duration::from_secs(3));
        eprintln!("[DEBUG] sending 'h'");
        session.send("h");
        eprintln!("[DEBUG] sending 'e'");
        session.send("e");
        eprintln!("[DEBUG] calling finish()");
        let stdout = session.finish();
        eprintln!(
            "[DEBUG] finish() returned, stdout length = {}",
            stdout.len()
        );

        assert!(
            stdout.contains("Debugger commands:"),
            "No help output:\n{stdout}"
        );
        assert!(
            stdout.contains("'b' | 'breakpoint'"),
            "No breakpoint help:\n{stdout}"
        );
        assert!(
            stdout.contains("Debugger is exiting"),
            "No exit message:\n{stdout}"
        );
    }
}
