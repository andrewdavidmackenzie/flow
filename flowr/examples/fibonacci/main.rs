extern crate serial_test;

fn main() {
    utilities::run_example(file!(), "flowrcli",
                           false, true);
}

#[cfg(test)]
mod test {
    use std::env;
    use std::path::PathBuf;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_fibonacci_example() {
        let _ = env::set_current_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().expect("Could not cd into flow directory"));

        super::main();
        utilities::check_test_output(file!());
    }
}
