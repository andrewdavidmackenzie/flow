use std::path::Path;

use simpath::Simpath;

pub fn check_flow_lib_path() {
    if let Some(parent) = Path::new(env!("CARGO_MANIFEST_DIR")).parent() {
        match std::env::var("FLOW_LIB_PATH") {
            Err(_) => {
                println!("'FLOW_LIB_PATH' is not set. \n\
                        For this 'flowstdlib' to be found by 'flowc' or 'flowr' the '-L {}' option must be used", parent.display());
            }
            Ok(value) => {
                let lib_path = Simpath::new_with_separator("FLOW_LIB_PATH", ',');
                if !lib_path.contains(&parent.display().to_string()) {
                    println!("'FLOW_LIB_PATH' is set to '{}'. But it does not contain the parent directory of this 'flowstdlib' directory.\n\
                            For flowc or flowr to find this 'flowstdlib' the '-L {}' option must be used.",
                         value, parent.display());
                } else {
                    println!("'FLOW_LIB_PATH' is set to '{}' and contains the parent directory of this 'flowstdlib' directory.\n\
                            This 'flowstdlib' should be found correctly by 'flowc' and 'flowr'",
                         value
                );
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use super::check_flow_lib_path;

    #[test]
    fn flow_pib_path() {
        std::env::set_var("FLOW_LIB_PATH", Path::new(env!("CARGO_MANIFEST_DIR")));
        check_flow_lib_path();
    }

    #[test]
    fn flow_pib_path_parent() {
        std::env::set_var(
            "FLOW_LIB_PATH",
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("Couldn't get parent dir"),
        );
        check_flow_lib_path();
    }

    #[test]
    fn no_flow_pib_path() {
        std::env::remove_var("FLOW_LIB_PATH");
        check_flow_lib_path();
    }
}
