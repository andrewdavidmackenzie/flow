use std::env;
use std::path::Path;

use url::Url;

pub fn absolute_file_url_from_relative_path(path: &str) -> String {
    let flow_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    Url::from_directory_path(flow_root).unwrap().join(path).unwrap().to_string()
}