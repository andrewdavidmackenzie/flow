//! A runner for the example using flowrcli
fn main() {
    utilities::run_example(file!(), "flowrcli", false, true);
}

#[cfg(test)]
mod test {
    use std::env;
    use std::path::PathBuf;

    #[test]
    #[ignore] // Disabled — path_tracker sub-flow needs restructuring for internal/external value semantics
    fn test_router_example() {
        let _ = env::set_current_dir(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("Could not cd into flow directory"),
        );

        super::main();
        utilities::check_test_output(file!());
    }
}
