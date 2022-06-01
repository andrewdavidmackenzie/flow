use std::env;
use std::path::Path;
use std::path::PathBuf;

use simpath::Simpath;
use url::Url;

pub fn set_lib_search_path_to_project() -> Simpath {
    let mut lib_search_path = Simpath::new("lib_search_path");

    // Add the parent directory of 'context' which is in flowr/src so it can be found
    let root_str = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Could not get project root dir");
    let runtime_parent = root_str.join("flowr/src");
    lib_search_path.add_directory(runtime_parent.to_str().expect("Could not convert path to string"));

    lib_search_path
}

pub fn get_canonical_context_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let samples_dir = manifest_dir.parent().ok_or("Could not get parent dir")
        .expect("Could not get parent dir");
    samples_dir.join("flowr/src/cli").canonicalize().expect("Could not get absolute path")
}

pub fn absolute_file_url_from_relative_path(path: &str) -> Url {
    let flow_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("Could not get parent directory");
    Url::from_directory_path(flow_root)
        .expect("Could not create Url from directory path")
        .join(path)
        .expect("Could not jin path to Url")
}
