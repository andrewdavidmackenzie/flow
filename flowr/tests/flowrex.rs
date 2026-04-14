#![allow(missing_docs)]

#[test]
#[cfg_attr(target_os = "macos", ignore = "flowc hangs on macOS Sequoia+ (#2303)")]
fn test_fibonacci_flowrex_example() {
    utilities::test_example("flowr/examples/hello-world/main.rs", "flowrcli", true, true);
}
