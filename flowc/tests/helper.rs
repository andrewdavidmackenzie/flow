use std::env;
use std::path::PathBuf;

use url::Url;

pub fn absolute_file_url_from_relative_path(path: &str) -> String {
    let mut flow_root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    flow_root.pop();
    let abs_url = Url::from_directory_path(flow_root).unwrap().join(path).unwrap().to_string();
    abs_url
}

pub fn set_flow_lib_path() {
    let cargo_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut flow_root = PathBuf::from(cargo_dir);
    flow_root.pop();
    env::set_var("FLOW_LIB_PATH", flow_root.to_string_lossy().to_string());
}