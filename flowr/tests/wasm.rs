#![allow(missing_docs)]

#[test]
#[ignore]
fn test_fibonacci_wasm_example() {
    utilities::test_example("flowr/examples/fibonacci/main.rs",
                            "flowrcli", false, false);
}