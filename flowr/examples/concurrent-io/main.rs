//! Test flow for concurrent IO — stdout should appear while readline is blocking
fn main() {
    utilities::run_example(file!(), "flowrgui", false, true);
}

#[cfg(test)]
mod test {
    use std::env;
    use std::path::PathBuf;

    #[ignore = "Blocked by #2918 — stdout cannot execute while readline is pending"]
    #[test]
    fn test_concurrent_io() {
        let _ = env::set_current_dir(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("Could not cd into flow directory"),
        );

        // TODO: When #2918 is fixed, this test should:
        // 1. Start the flow with delayed stdin (via FIFO or background thread)
        // 2. Verify stdout output appears BEFORE stdin is provided
        // 3. Then send stdin and verify stderr output appears
        //
        // Currently, stdout is blocked until readline completes because the
        // coordinator serializes all context function communication through
        // a single ZMQ socket.
        super::main();
        utilities::check_test_output(file!());
    }
}
