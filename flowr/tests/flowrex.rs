#![allow(missing_docs)]

#[test]
#[cfg_attr(
    target_os = "windows",
    ignore = "Windows drive letter parsed as URL scheme (#2815)"
)]
fn test_fibonacci_flowrex_example() {
    utilities::test_example("flowr/examples/hello-world/main.rs", "flowrcli", true, true);
}
