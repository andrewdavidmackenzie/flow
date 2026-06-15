#![allow(missing_docs)]

#[test]
fn test_fibonacci_wasm_example() {
    let source = std::path::PathBuf::from("flowr")
        .join("examples")
        .join("fibonacci")
        .join("main.rs");
    utilities::test_example(source.to_str().expect("path"), "flowrcli", false, false);
}
