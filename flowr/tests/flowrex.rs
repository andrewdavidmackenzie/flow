#![allow(missing_docs)]

#[test]
#[ignore = "Hangs on macOS Sequoia+ CI runners"]
fn test_fibonacci_flowrex_example() {
    utilities::test_example("flowr/examples/hello-world/main.rs", "flowrcli", true, true);
}
