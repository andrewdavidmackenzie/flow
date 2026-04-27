#![allow(missing_docs)]

use serial_test::serial;

#[test]
#[serial]
fn test_fibonacci_flowrgui_example() {
    utilities::run_example("examples/fibonacci/main.rs", "flowrgui", false, true);
    utilities::check_test_output("examples/fibonacci/main.rs");
}

#[test]
#[serial]
fn test_line_echo_flowrgui_example() {
    utilities::run_example("examples/line-echo/main.rs", "flowrgui", false, true);
    utilities::check_test_output("examples/line-echo/main.rs");
}

#[test]
#[serial]
fn test_reverse_echo_flowrgui_example() {
    utilities::run_example("examples/reverse-echo/main.rs", "flowrgui", false, false);
    utilities::check_test_output("examples/reverse-echo/main.rs");
}
