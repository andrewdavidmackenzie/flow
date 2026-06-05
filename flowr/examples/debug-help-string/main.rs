//! Integration test for the debug help command using flowdb as a separate process
fn main() {}

#[cfg(test)]
#[cfg(feature = "debugger")]
mod test {
    use serial_test::serial;
    use utilities::DebugSession;

    #[test]
    #[serial]
    fn test_debug_help_string_example() {
        let example_dir =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/debug-help-string");
        let mut session = DebugSession::start(&example_dir, &[]);
        session.send("h");
        session.send("e");
        let stdout = session.finish();

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
