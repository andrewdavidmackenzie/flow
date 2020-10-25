use std::env;
use std::path::PathBuf;

use url::Url;

pub fn absolute_file_url_from_relative_path(path: &str) -> String {
    if env::var("FLOW_ROOT").is_err() {
        eprint!("Environment Variable 'FLOW_ROOT' is not set.\n");
        eprint!("In development you can set 'FLOW_ROOT' to the root directory of the project and that should work.\n");
        let cwd = env::current_dir().unwrap();
        let project_root = cwd.parent().unwrap();
        eprint!("Setting 'FLOW_ROOT' to '{}'\n", project_root.display());
        env::set_var("FLOW_ROOT", project_root)
    }

    let value = env::var("FLOW_ROOT").unwrap();
    let flow_root = PathBuf::from(value);
    Url::from_directory_path(flow_root).unwrap().join(path).unwrap().to_string()
}