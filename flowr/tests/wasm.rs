#![allow(missing_docs)]

#[test]
#[cfg_attr(
    target_os = "windows",
    ignore = "Windows drive letter parsed as URL scheme (#2815)"
)]
fn test_fibonacci_wasm_example() {
    utilities::test_example("flowr/examples/fibonacci/main.rs", "flowrcli", false, false);
}
