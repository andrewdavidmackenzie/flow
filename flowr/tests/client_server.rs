#![allow(missing_docs)]

use std::path::PathBuf;

#[test]
fn hello_world_client_server() {
    let example_dir = PathBuf::from("examples/hello-world");
    utilities::compile_example(&example_dir, "flowrcli");
    utilities::execute_flow_client_server("hello-world", example_dir.join("manifest.json"));
}