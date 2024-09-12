//! A runner for the example using flowrcli
fn main() {
    utilities::run_example(file!(), "flowrcli", false, true);
}

#[cfg(test)]
mod test {
    use std::env;
    use std::path::PathBuf;

    #[test]
    #[ignore]
    fn test_reverse_echo_example() {
        let _ = env::set_current_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().expect("Could not cd into flow directory"));

        super::main();
        utilities::check_test_output(file!());
    }
}
