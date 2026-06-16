#![allow(missing_docs)]

#[test]
fn test_fibonacci_flowrex_example() {
    let source = std::path::PathBuf::from("flowr")
        .join("examples")
        .join("hello-world")
        .join("main.rs");
    utilities::test_example(source.to_str().expect("path"), "flowrcli", true, true);
}
