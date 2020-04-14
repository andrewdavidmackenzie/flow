use flowclib::compiler::loader;
use provider::content::provider::MetaProvider;

#[path="helper.rs"] mod helper;

#[test]
fn load_hello_world_from_context() {
    let meta_provider = MetaProvider {};
    loader::load(&helper::absolute_file_url_from_relative_path("samples/hello-world/context.toml"),
                 &meta_provider).unwrap();
}

#[test]
fn load_reverse_echo_from_toml() {
    let meta_provider = MetaProvider {};
    loader::load(&helper::absolute_file_url_from_relative_path("samples/reverse-echo/context.toml"),
                 &meta_provider).unwrap();
}

#[test]
fn load_fibonacci_from_file() {
    let meta_provider = MetaProvider {};
    loader::load(&helper::absolute_file_url_from_relative_path("samples/fibonacci/context.toml"),
                 &meta_provider).unwrap();
}

#[test]
fn load_fibonacci_from_directory() {
    let meta_provider = MetaProvider {};
    let url = helper::absolute_file_url_from_relative_path("samples/fibonacci");
    loader::load(&url, &meta_provider).unwrap();
}