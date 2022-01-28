#![warn(clippy::unwrap_used)]

use std::{env, io};
use std::path::Path;

use simpath::Simpath;

pub fn main() -> io::Result<()>{
    let bin_path = env::current_exe()?;
    println!(
        "'{}' version {}",
        env!("CARGO_CRATE_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    println!("For more details see: {}", env!("CARGO_PKG_HOMEPAGE"));
    println!(
        "'{}' binary located at '{}'",
        env!("CARGO_CRATE_NAME"),
        bin_path.display()
    );

    let bin_directory = bin_path.parent().ok_or_else(||
        io::Error::new( io::ErrorKind::Other, "Could not get directory where 'flowstdlib' binary is located"))?;
    check_flow_lib_path(bin_directory);

    Ok(())
}

fn check_flow_lib_path(parent: &Path) {
    match std::env::var("FLOW_LIB_PATH") {
        Err(_) => {
            println!("'FLOW_LIB_PATH' is not set. \n\
                    For this 'flowstdlib' to be found by 'flowc' or 'flowr' the '-L {}' option must be used", parent.display());
        }
        Ok(value) => {
            let lib_path = Simpath::new_with_separator("FLOW_LIB_PATH", ',');
            if !lib_path.contains(&parent.display().to_string()) {
                println!("'FLOW_LIB_PATH' is set to '{}'. \nIt does not contain the parent directory of this 'flowstdlib' directory.\n\
                        For flowc or flowr to find 'flowstdlib' add '{}' to FLOW_LIB_PATH or use the '-L {}' option.",
                         value, parent.display(), parent.display());
            } else {
                println!("'FLOW_LIB_PATH' is set to '{}' and contains the parent directory of this 'flowstdlib' directory.\n\
                        This 'flowstdlib' should be found correctly by 'flowc' and 'flowr'",
                         value
                );
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use super::check_flow_lib_path;

    #[test]
    fn flow_lib_path() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        std::env::set_var("FLOW_LIB_PATH", manifest_dir);
        check_flow_lib_path(manifest_dir);
    }

    #[test]
    fn flow_lib_path_parent() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        std::env::set_var(
            "FLOW_LIB_PATH",
            manifest_dir
                .parent()
                .expect("Couldn't get parent dir"),
        );
        check_flow_lib_path(manifest_dir);
    }

    #[test]
    fn no_flow_lib_path() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        std::env::remove_var("FLOW_LIB_PATH");
        check_flow_lib_path(manifest_dir);
    }

    #[test]
    fn get_manifest_test() {
        let manifest = flowstdlib::get_manifest().expect("Could not get manifest");
        assert_eq!(manifest.locators.len(), 30);
    }
}
