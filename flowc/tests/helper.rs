use std::env;
use std::path::PathBuf;

use url::Url;

pub fn absolute_file_url_from_relative_path(path: &str) -> String {
    let flow_root = PathBuf::from(env::var("FLOW_LIB_PATH").unwrap());
    let abs_url = Url::from_directory_path(flow_root).unwrap().join(path).unwrap().to_string();
    abs_url
}