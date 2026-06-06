//! Integration tests for debugger commands using flowrdb as a separate process
fn main() {}

#[cfg(test)]
#[cfg(feature = "debugger")]
mod test {
    use serial_test::serial;
    use utilities::DebugSession;

    fn example_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/debug-print-args")
    }

    #[test]
    #[serial]
    fn test_debug_continue_and_exit() {
        let mut session = DebugSession::start(&example_dir(), &["test_arg1"]);
        session.send("c");
        std::thread::sleep(std::time::Duration::from_secs(2));
        session.send("e");
        let stdout = session.finish();

        assert!(
            stdout.contains("Flow has completed"),
            "No completion:\n{stdout}"
        );
        assert!(stdout.contains("Debugger is exiting"), "No exit:\n{stdout}");
    }

    #[test]
    #[serial]
    fn test_debug_step() {
        let mut session = DebugSession::start(&example_dir(), &["test_arg1"]);
        session.send("s");
        session.send("e");
        let stdout = session.finish();

        assert!(
            stdout.contains("About to send Job #"),
            "No job info after step:\n{stdout}"
        );
        assert!(
            stdout.contains("Inputs:"),
            "No inputs after step:\n{stdout}"
        );
        assert!(stdout.contains("Debugger is exiting"), "No exit:\n{stdout}");
    }

    #[test]
    #[serial]
    fn test_debug_functions() {
        let mut session = DebugSession::start(&example_dir(), &["test_arg1"]);
        session.send("f");
        session.send("e");
        let stdout = session.finish();

        assert!(
            stdout.contains("Functions List"),
            "No function list:\n{stdout}"
        );
        assert!(stdout.contains("inspect"), "No inspect hint:\n{stdout}");
        assert!(stdout.contains("Debugger is exiting"), "No exit:\n{stdout}");
    }

    #[test]
    #[serial]
    fn test_debug_inspect() {
        let mut session = DebugSession::start(&example_dir(), &["test_arg1"]);
        session.send("i");
        session.send("e");
        let stdout = session.finish();

        assert!(stdout.contains("States:"), "No state info:\n{stdout}");
        assert!(stdout.contains("Debugger is exiting"), "No exit:\n{stdout}");
    }

    #[test]
    #[serial]
    fn test_debug_list_empty() {
        let mut session = DebugSession::start(&example_dir(), &["test_arg1"]);
        session.send("l");
        session.send("e");
        let stdout = session.finish();

        assert!(stdout.contains("Debugger is exiting"), "No exit:\n{stdout}");
    }

    #[test]
    #[serial]
    fn test_debug_breakpoint_and_delete() {
        let mut session = DebugSession::start(&example_dir(), &["test_arg1"]);
        session.send("b 0");
        session.send("l");
        session.send("d 0");
        session.send("e");
        let stdout = session.finish();

        assert!(stdout.contains("Debugger is exiting"), "No exit:\n{stdout}");
    }

    #[test]
    #[serial]
    fn test_debug_validate() {
        let mut session = DebugSession::start(&example_dir(), &["test_arg1"]);
        session.send("v");
        session.send("e");
        let stdout = session.finish();

        assert!(stdout.contains("Debugger is exiting"), "No exit:\n{stdout}");
    }

    #[test]
    #[serial]
    fn test_debug_step_n() {
        let mut session = DebugSession::start(&example_dir(), &["test_arg1"]);
        session.send("s 2");
        session.send("e");
        let stdout = session.finish();

        assert!(
            stdout.contains("About to send Job #") || stdout.contains("Flow has completed"),
            "No job info or completion after step 2:\n{stdout}"
        );
        assert!(stdout.contains("Debugger is exiting"), "No exit:\n{stdout}");
    }

    #[test]
    #[serial]
    fn test_debug_inspect_function() {
        let mut session = DebugSession::start(&example_dir(), &["test_arg1"]);
        session.send("f");
        session.send("i 1");
        session.send("e");
        let stdout = session.finish();

        assert!(
            stdout.contains("Functions List"),
            "No function list:\n{stdout}"
        );
        assert!(stdout.contains("Debugger is exiting"), "No exit:\n{stdout}");
    }

    #[test]
    #[serial]
    fn test_debug_inspect_input() {
        let mut session = DebugSession::start(&example_dir(), &["test_arg1"]);
        session.send("i 0:0");
        session.send("e");
        let stdout = session.finish();

        assert!(stdout.contains("Debugger is exiting"), "No exit:\n{stdout}");
    }

    #[test]
    #[serial]
    fn test_debug_inspect_output() {
        let mut session = DebugSession::start(&example_dir(), &["test_arg1"]);
        session.send("i 0/");
        session.send("e");
        let stdout = session.finish();

        assert!(stdout.contains("Debugger is exiting"), "No exit:\n{stdout}");
    }

    #[test]
    #[serial]
    fn test_debug_inspect_block() {
        let mut session = DebugSession::start(&example_dir(), &["test_arg1"]);
        session.send("i 0->1");
        session.send("e");
        let stdout = session.finish();

        assert!(stdout.contains("Debugger is exiting"), "No exit:\n{stdout}");
    }

    #[test]
    #[serial]
    fn test_debug_breakpoint_output_spec() {
        let mut session = DebugSession::start(&example_dir(), &["test_arg1"]);
        session.send("b 0/");
        session.send("l");
        session.send("d 0/");
        session.send("e");
        let stdout = session.finish();

        assert!(stdout.contains("Debugger is exiting"), "No exit:\n{stdout}");
    }

    #[test]
    #[serial]
    fn test_debug_breakpoint_input_spec() {
        let mut session = DebugSession::start(&example_dir(), &["test_arg1"]);
        session.send("b 0:0");
        session.send("l");
        session.send("d 0:0");
        session.send("e");
        let stdout = session.finish();

        assert!(stdout.contains("Debugger is exiting"), "No exit:\n{stdout}");
    }

    #[test]
    #[serial]
    fn test_debug_breakpoint_block_spec() {
        let mut session = DebugSession::start(&example_dir(), &["test_arg1"]);
        session.send("b 0->1");
        session.send("l");
        session.send("d 0->1");
        session.send("e");
        let stdout = session.finish();

        assert!(stdout.contains("Debugger is exiting"), "No exit:\n{stdout}");
    }

    #[test]
    #[serial]
    fn test_debug_modify() {
        let mut session = DebugSession::start(&example_dir(), &["test_arg1"]);
        session.send("m max_parallel_jobs=2");
        session.send("e");
        let stdout = session.finish();

        assert!(stdout.contains("Debugger is exiting"), "No exit:\n{stdout}");
    }

    #[test]
    #[serial]
    fn test_debug_reset() {
        let mut session = DebugSession::start(&example_dir(), &["test_arg1"]);
        session.send("c");
        std::thread::sleep(std::time::Duration::from_secs(2));
        session.send("r");
        std::thread::sleep(std::time::Duration::from_secs(1));
        session.send("e");
        let stdout = session.finish();

        assert!(
            stdout.contains("Resetting state"),
            "No reset message:\n{stdout}"
        );
        assert!(stdout.contains("Debugger is exiting"), "No exit:\n{stdout}");
    }

    #[test]
    #[serial]
    fn test_debug_delete_all() {
        let mut session = DebugSession::start(&example_dir(), &["test_arg1"]);
        session.send("b 0");
        session.send("b 1");
        session.send("d *");
        session.send("e");
        let stdout = session.finish();

        assert!(stdout.contains("Debugger is exiting"), "No exit:\n{stdout}");
    }

    #[test]
    #[serial]
    fn test_debug_completion_breakpoint() {
        let mut session = DebugSession::start(&example_dir(), &["test_arg1"]);
        session.send("b 0+");
        session.send("l");
        session.send("d 0+");
        session.send("e");
        let stdout = session.finish();

        assert!(
            stdout.contains("Completion breakpoint set on Function #0"),
            "No completion breakpoint confirmation:\n{stdout}"
        );
        assert!(
            stdout.contains("Completion Breakpoints"),
            "No completion breakpoints in list:\n{stdout}"
        );
        assert!(
            stdout.contains("Function #0+"),
            "No Function #0+ in list:\n{stdout}"
        );
        assert!(
            stdout.contains("Completion breakpoint on Function #0 was deleted"),
            "No deletion confirmation:\n{stdout}"
        );
    }

    #[test]
    #[serial]
    fn test_debug_processes() {
        let mut session = DebugSession::start(&example_dir(), &["test_arg1"]);
        session.send("p");
        session.send("e");
        let stdout = session.finish();

        assert!(
            stdout.contains("Process Tree"),
            "No process tree header:\n{stdout}"
        );
        assert!(stdout.contains("Debugger is exiting"), "No exit:\n{stdout}");
    }
}
