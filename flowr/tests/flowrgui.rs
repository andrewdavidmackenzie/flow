#![allow(missing_docs)]

#[test]
#[cfg_attr(target_os = "macos", ignore = "flowc hangs on macOS Sequoia+ (#2303)")]
fn test_fibonacci_flowrgui_example() {
    utilities::run_example("examples/fibonacci/main.rs", "flowrgui", false, true);
}
