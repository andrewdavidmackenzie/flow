use std::env;
use std::path::PathBuf;

use url::Url;

pub fn absolute_file_url_from_relative_path(path: &str) -> String {
    if env::var("FLOW_LIB_PATH").is_err() {
        eprint!("Environment Variable 'FLOW_LIB_PATH' is not set, so cannot find libraries for testing\n");
        eprint!("In development you can set 'FLOW_LIB_PATH' to the root directory of the project and that should work.\n");
        let cwd = env::current_dir().unwrap();
        let project_root = cwd.parent().unwrap();
        eprint!("Setting 'FLOW_LIB_PATH' to '{}'\n", project_root.display());
        env::set_var("FLOW_LIB_PATH", project_root)
    }

    let value = env::var("FLOW_LIB_PATH").unwrap();
    let flow_root = PathBuf::from(value);
    Url::from_directory_path(flow_root).unwrap().join(path).unwrap().to_string()
}