use std::path::Path;

use simpath::Simpath;

pub fn main() {
    println!("'{}' version {} installed", env!("CARGO_CRATE_NAME"), env!("CARGO_PKG_VERSION"));
    println!("For more details see: {}", env!("CARGO_PKG_HOMEPAGE"));
    check_flow_lib_path();
}

fn check_flow_lib_path() {
    let parent = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().display().to_string();
    match std::env::var("FLOW_LIB_PATH") {
        Err(_) => {
            println!("'FLOW_LIB_PATH' is not set, so 'flowstdlib' will not be found by 'flowc' or 'flowr'.\n\
             Set it to an appropriate value thus: export FLOW_LIB_PATH=\"{}\"", parent);
        }
        Ok(value) => {
            let lib_path = Simpath::new("FLOW_LIB_PATH");
            if !lib_path.contains(&parent) {
                println!("'FLOW_LIB_PATH' is set to '{}'. But it does not contain the directory where 'flowstdlib' is\n\
                            so 'flowstdlib' will not be found by 'flowc' or 'flowr'. \n\
                            Add an entry for this directory thus: export FLOW_LIB_PATH=\"{}:$FLOW_LIB_PATH\"",
                         value, parent);
            } else {
                println!("'FLOW_LIB_PATH' is set to '{}' and correctly contains directory '{}'",
                         value, parent);
            }
        }
    }
}