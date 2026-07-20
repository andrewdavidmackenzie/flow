#![allow(missing_docs)]

#[cfg_attr(target_os = "windows", ignore)]
#[test]
fn test_hello_world_flowrex_example() {
    let source = std::path::PathBuf::from("flowr")
        .join("examples")
        .join("hello-world")
        .join("main.rs");
    utilities::test_example(source.to_str().expect("path"), "flowrcli", true, true);
}
