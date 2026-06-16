//! A runner for the Running Average example using flowrcli
fn main() {
    utilities::run_example(file!(), "flowrcli", false, true);
}

#[cfg(test)]
mod test {
    use std::env;
    use std::path::PathBuf;

    #[test]
    #[cfg_attr(
        target_os = "windows",
        ignore = "Non-deterministic intermediate output order on Windows"
    )]
    fn test_running_average_example() {
        let _ = env::set_current_dir(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("Could not cd into flow directory"),
        );

        super::main();
        utilities::check_test_output(file!());
    }
}
