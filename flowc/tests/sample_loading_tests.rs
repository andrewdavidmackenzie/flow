extern crate flowclib;
extern crate flowrlib;
extern crate provider;
extern crate url;

use std::env;

use flowclib::compiler::loader;
use url::Url;

use provider::args::url_from_string;
use provider::content::provider::MetaProvider;

fn url_from_rel_path(path: &str) -> String {
    let cwd = Url::from_file_path(env::current_dir().unwrap()).unwrap();
    cwd.join(path).unwrap().to_string()
}

fn set_flow_lib_path() {
    let mut parent_dir = std::env::current_dir().unwrap();
    parent_dir.pop();
    println!("Set 'FLOW_LIB_PATH' to '{}'", parent_dir.to_string_lossy().to_string());
    env::set_var("FLOW_LIB_PATH", parent_dir.to_string_lossy().to_string());
}

#[test]
fn load_hello_world_from_context() {
    set_flow_lib_path();
    let meta_provider = MetaProvider {};
    loader::load_context(&url_from_rel_path("samples/hello-world/context.toml"),
                         &meta_provider).unwrap();
}

#[test]
fn load_hello_world_include() {
    set_flow_lib_path();
    let meta_provider = MetaProvider {};
    loader::load_context(&url_from_rel_path("samples/hello-world-include/context.toml"),
                         &meta_provider).unwrap();
}

#[test]
fn load_hello_world_flow1() {
    set_flow_lib_path();
    let meta_provider = MetaProvider {};
    loader::load_context(&url_from_rel_path("samples/hello-world/flow1.toml"),
                         &meta_provider).unwrap();
}

#[test]
fn load_reverse_echo_from_toml() {
    set_flow_lib_path();
    let meta_provider = MetaProvider {};
    loader::load_context(&url_from_rel_path("samples/reverse-echo/context.toml"),
                         &meta_provider).unwrap();
}

#[test]
fn load_fibonacci_from_file() {
    set_flow_lib_path();
    let meta_provider = MetaProvider {};
    loader::load_context(&url_from_rel_path("samples/fibonacci/context.toml"),
                         &meta_provider).unwrap();
}

#[test]
fn load_fibonacci_from_directory() {
    set_flow_lib_path();
    let meta_provider = MetaProvider {};
    let url = url_from_string(Some("../samples/fibonacci")).unwrap();
    loader::load_context(&url.into_string(), &meta_provider).unwrap();
}