//! Minimal reproducer for loopback-clearing race condition (#2887)
fn main() {
    utilities::run_example(file!(), "flowrcli", false, true);
}

#[cfg(test)]
mod test {
    use std::env;
    use std::path::PathBuf;

    #[test]
    fn test_loopback_race_example() {
        let _ = env::set_current_dir(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("Could not cd into flow directory"),
        );

        super::main();
        utilities::check_test_output(file!());
    }
}
