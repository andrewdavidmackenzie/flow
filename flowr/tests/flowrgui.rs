#![allow(missing_docs)]

use serial_test::serial;

#[test]
#[serial]
fn test_fibonacci_flowrgui_example() {
    utilities::test_example("flowr/examples/fibonacci/main.rs", "flowrgui", false, true);
}
