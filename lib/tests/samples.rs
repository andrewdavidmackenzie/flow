use std::path::PathBuf;

extern crate flowlib;
use flowlib::loader::loader;

#[test]
#[ignore]
fn sample_hello_world_simple_yaml() {
    let path = PathBuf::from("../samples/hello-world-simple-yaml/context.yaml");
    loader::load(path, false).unwrap();
}

#[test]
#[ignore]
fn sample_hello_world_yaml() {
    let path = PathBuf::from("../samples/hello-world-yaml/context.yaml");
    loader::load(path, false).unwrap();
}

#[test]
fn dump_hello_world_simple() {
    let path = PathBuf::from("../samples/hello-world-simple/context.toml");
    loader::load(path, true).unwrap();
}

#[test]
fn dump_hello_world_context() {
    let path = PathBuf::from("../samples/hello-world/context.toml");
    loader::load(path, true).unwrap();
}

#[test]
fn dump_hello_world_flow1() {
    let path = PathBuf::from("../samples/hello-world/flow1.toml");
    loader::load(path, true).unwrap();
}

#[test]
fn dump_complex1() {
    let path = PathBuf::from("../samples/complex1/context.toml");
    loader::load(path, true).unwrap();
}

#[test]
fn dump_fibonacci() {
    let path = PathBuf::from("../samples/fibonacci/context.toml");
    loader::load(path, true).unwrap();
}