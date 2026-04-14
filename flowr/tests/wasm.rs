#![allow(missing_docs)]

#[test]
#[cfg_attr(target_os = "macos", ignore = "flowc hangs on macOS Sequoia+ (#2303)")]
fn test_fibonacci_wasm_example() {
    utilities::test_example("flowr/examples/fibonacci/main.rs", "flowrcli", false, false);
}
