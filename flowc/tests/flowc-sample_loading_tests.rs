use flowclib::compiler::loader;
use flowclib::model::process::Process::{FlowProcess, FunctionProcess};
use provider::content::provider::MetaProvider;

#[path="helper.rs"] mod helper;

#[test]
fn load_hello_world_from_context() {
    let meta_provider = MetaProvider {};
    loader::load(&helper::absolute_file_url_from_relative_path("samples/hello-world/context.toml"),
                 &meta_provider).expect("Could not load hello-world sample flow");
}

#[test]
fn load_reverse_echo_from_toml() {
    let meta_provider = MetaProvider {};
    loader::load(&helper::absolute_file_url_from_relative_path("samples/reverse-echo/context.toml"),
                 &meta_provider).expect("Could not load reverse-echo sample flow");
}

#[test]
fn load_fibonacci_from_file() {
    let meta_provider = MetaProvider {};
    loader::load(&helper::absolute_file_url_from_relative_path("samples/fibonacci/context.toml"),
                 &meta_provider).expect("Could not load fibonacci sample flow");
}

#[test]
fn load_range_of_ranges_from_file() {
    let meta_provider = MetaProvider {};
    let process = loader::load(&helper::absolute_file_url_from_relative_path("samples/range-of-ranges/context.toml"),
                 &meta_provider).expect("Could not load range-of-ranges sample flow");

    match process {
        FlowProcess(flow) => {
            match serde_json::to_string(&flow) {
                Ok(contents) => assert!(!contents.is_empty(), "Serialized flow was empty"),
                Err(e) => panic!("Could not serialize flow: {}", e)
            }
        },
        FunctionProcess(_) => panic!("Process deserialized was not a flow")
    }
}

#[test]
fn load_fibonacci_from_directory() {
    let meta_provider = MetaProvider {};
    let url = helper::absolute_file_url_from_relative_path("samples/fibonacci");
    loader::load(&url, &meta_provider).expect("Could not load fibonacci sample flow from directory");
}