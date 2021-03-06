use std::collections::HashSet;

use simpath::Simpath;
use url::Url;

use flowclib::compiler::loader;
use flowcore::lib_provider::MetaProvider;

#[path = "helper.rs"]
mod helper;

#[test]
fn load_hello_world_from_context() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project());
    loader::load(
        &helper::absolute_file_url_from_relative_path("samples/hello-world/context.toml"),
        &meta_provider,
        &mut HashSet::<(Url, Url)>::new(),
    )
    .expect("Could not load sample");
}

#[test]
fn load_reverse_echo_from_toml() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project());
    loader::load(
        &helper::absolute_file_url_from_relative_path("samples/reverse-echo/context.toml"),
        &meta_provider,
        &mut HashSet::<(Url, Url)>::new(),
    )
    .expect("Could not load sample");
}

#[test]
fn load_fibonacci_from_file() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project());
    loader::load(
        &helper::absolute_file_url_from_relative_path("samples/fibonacci/context.toml"),
        &meta_provider,
        &mut HashSet::<(Url, Url)>::new(),
    )
    .expect("Could not load sample");
}

#[test]
fn load_fibonacci_from_directory() {
    let meta_provider = MetaProvider::new(helper::set_lib_search_path_to_project());
    let url = helper::absolute_file_url_from_relative_path("samples/fibonacci");
    loader::load(&url, &meta_provider, &mut HashSet::<(Url, Url)>::new())
        .expect("Could not load sample");
}

pub fn set_lib_search_path_flowstdlib_on_web() -> Simpath {
    let mut lib_search_path = Simpath::new("lib_search_path");

    // Add the parent directory of 'flowruntime' which is in flowr/src/lib so `lib://flowruntime/*` references
    // can be found
    // let root_str = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("Could not get project root dir");
    // let runtime_parent = root_str.join("flowr/src/lib");
    // lib_search_path.add_directory(runtime_parent.to_str().unwrap());
    lib_search_path.add_url(&Url::parse("https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/flowr/src/lib/flowruntime")
        .expect("Could not parse the url for Simpath"));

    // Add the url of 'flowstdlib' on the web, so `lib://flowstdlib/*` references can be found
    lib_search_path.add_url(
        &Url::parse(
            "https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/flowstdlib",
        )
        .expect("Could not parse the url for Simpath"),
    );

    lib_search_path
}

#[test]
fn load_fibonacci_libs_on_the_web() {
    let meta_provider = MetaProvider::new(set_lib_search_path_flowstdlib_on_web());
    let url = helper::absolute_file_url_from_relative_path("samples/fibonacci");
    loader::load(&url, &meta_provider, &mut HashSet::<(Url, Url)>::new())
        .expect("Could not load sample");
}
