//! A runner for the Game of Life example using flowrcli
fn main() {
    utilities::run_example(file!(), "flowrcli", false, true);
}

#[cfg(test)]
mod test {
    use std::env;
    use std::path::PathBuf;

    #[cfg_attr(target_os = "windows", ignore)]
    #[test]
    fn test_game_of_life_example() {
        let _ = env::set_current_dir(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("Could not cd into flow directory"),
        );

        super::main();
        utilities::check_test_output(file!());
    }
}
