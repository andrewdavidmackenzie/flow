#![allow(missing_docs)]

#[test]
#[ignore = "Problem with nested / recursive flows still!"]
fn test_fibonacci_wasm_example() {
    utilities::test_example("flowr/examples/fibonacci/main.rs",
                            "flowrcli", false, false);
}