#![allow(missing_docs)]

use serial_test::serial;

#[test]
#[serial]
fn test_fibonacci_flowrgui_example() {
    utilities::test_example("flowr/examples/fibonacci/main.rs", "flowrgui", false, true);
}

#[test]
#[serial]
fn test_line_echo_flowrgui_example() {
    utilities::test_example("flowr/examples/line-echo/main.rs", "flowrgui", false, true);
}

#[test]
#[serial]
fn test_reverse_echo_flowrgui_example() {
    utilities::test_example(
        "flowr/examples/reverse-echo/main.rs",
        "flowrgui",
        false,
        false,
    );
}
