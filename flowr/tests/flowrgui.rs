#![allow(missing_docs)]

use serial_test::serial;
use std::path::PathBuf;

fn check_stdout(example_path: &str) {
    let mut sample_dir = PathBuf::from(example_path);
    sample_dir.pop();
    utilities::compare_and_fail(
        sample_dir.join("expected.stdout"),
        sample_dir.join("test.stdout"),
    );
}

#[test]
#[serial]
fn test_fibonacci_flowrgui_example() {
    utilities::run_example("examples/fibonacci/main.rs", "flowrgui", false, true);
    check_stdout("examples/fibonacci/main.rs");
}

#[test]
#[serial]
fn test_line_echo_flowrgui_example() {
    utilities::run_example("examples/line-echo/main.rs", "flowrgui", false, true);
    check_stdout("examples/line-echo/main.rs");
}

#[test]
#[serial]
fn test_reverse_echo_flowrgui_example() {
    utilities::run_example("examples/reverse-echo/main.rs", "flowrgui", false, false);
    check_stdout("examples/reverse-echo/main.rs");
}
