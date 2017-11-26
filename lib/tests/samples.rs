use std::path::PathBuf;

extern crate flowlib;
use flowlib::loader::loader;
use flowlib::dumper::dump;

#[test]
#[ignore]
fn sample_hello_world_simple() {
    let path = PathBuf::from("../samples/hello-world-simple/context.yaml");
    loader::load_flow("", path).unwrap();
}

#[test]
#[ignore]
fn sample_hello_world() {
    let path = PathBuf::from("../samples/hello-world/context.yaml");
    loader::load_flow("", path).unwrap();
}

#[test]
fn dump_hello_simple_toml() {
    let path = PathBuf::from("../samples/hello-world-simple-toml/context.toml");
    dump(&loader::load_flow("", path).unwrap(), 0);
}

#[test]
fn dump_hello_world_toml_context() {
    let path = PathBuf::from("../samples/hello-world-toml/context.toml");
    dump(&loader::load_flow("", path).unwrap(), 0);
}

#[test]
fn dump_hello_world_flow1_toml() {
    let path = PathBuf::from("../samples/hello-world-toml/flow1.toml");
    dump( &loader::load_flow("", path).unwrap(), 0);
}

#[test]
fn dump_complex1() {
    let path = PathBuf::from("../samples/complex1/context.toml");
    dump(&loader::load_flow("", path).unwrap(), 0);
}