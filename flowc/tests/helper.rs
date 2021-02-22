use std::env;
use std::path::Path;

use url::Url;

pub fn set_lib_search_path() {
    let root_str = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("Could not get project root dir");
    let runtime_parent = root_str.join("flowr/src/lib");
    let lib_search_path = format!("{}:{}", root_str.display(), runtime_parent.display());
    println!("FLOW_LIB_PATH set to '{}'", lib_search_path);
    env::set_var("FLOW_LIB_PATH", lib_search_path);
}

pub fn absolute_file_url_from_relative_path(path: &str) -> String {
    let flow_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    Url::from_directory_path(flow_root).unwrap().join(path).unwrap().to_string()
}