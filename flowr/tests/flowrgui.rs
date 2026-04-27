#![allow(missing_docs)]

#[test]
fn test_fibonacci_flowrgui_example() {
    utilities::run_example("examples/fibonacci/main.rs", "flowrgui", false, true);
}

#[test]
fn test_line_echo_flowrgui_example() {
    utilities::run_example("examples/line-echo/main.rs", "flowrgui", false, true);
}

#[test]
fn test_reverse_echo_flowrgui_example() {
    utilities::run_example("examples/reverse-echo/main.rs", "flowrgui", false, false);
}
