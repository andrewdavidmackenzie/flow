//! A runner for the example using flowrcli
fn main() {
    utilities::run_example(file!(), "flowrcli", false, true);
}

#[cfg(test)]
mod test {
    use std::env;
    use std::path::PathBuf;

    #[test]
    fn test_hello_world_example() {
        let _ = env::set_current_dir(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("Could not cd into flow directory"),
        );

        eprintln!("[FLOW_TEST] test_hello_world_example: calling super::main()");
        super::main();
        eprintln!("[FLOW_TEST] test_hello_world_example: super::main() returned, checking output");
        utilities::check_test_output(file!());
        eprintln!("[FLOW_TEST] test_hello_world_example: PASSED");
    }
}
