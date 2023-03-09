#![warn(clippy::unwrap_used)]
//! This `flowstdlib` binary checks that the installed flowstdlib library can be found

use std::{env, io};

use simpath::Simpath;

/// Check the `FLOW_LIB_PATH` environment variable is set and that we can find the 'flowstdlib'
/// lib directory using it.
pub fn main() -> io::Result<()>{
    match env::var("FLOW_LIB_PATH") {
        Err(_) => {
            println!("'FLOW_LIB_PATH' is not set. It must be set in order for the 'flowstdlib' directory to be found");
        }
        Ok(value) => {
            let lib_path = Simpath::new_with_separator("FLOW_LIB_PATH", ',');
            if !lib_path.contains("flowstdlib") {
                println!("'FLOW_LIB_PATH' is set to '{value}'. The 'flowstdlib' directory could not be found");
            } else {
                println!("'FLOW_LIB_PATH' is set to '{value}'. The 'flowstdlib' lib directory was found"
                );
            }
        }
    }

    Ok(())
}