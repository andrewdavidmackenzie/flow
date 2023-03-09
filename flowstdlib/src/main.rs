#![warn(clippy::unwrap_used)]
//! This `flowstdlib` binary checks that the installed flowstdlib library can be found

use std::{env, io};

use simpath::{FileType, FoundType, Simpath};

/// Check the `FLOW_LIB_PATH` environment variable is set and that we can find the 'flowstdlib'
/// lib directory using it.
pub fn main() -> io::Result<()>{
    match env::var("FLOW_LIB_PATH") {
        Err(_) => {
            println!("'FLOW_LIB_PATH' is not set. It must be set in order for the 'flowstdlib' lib directory to be found");
        }
        Ok(value) => {
                println!("'FLOW_LIB_PATH' is set to '{value}'");
            let lib_path = Simpath::new_with_separator("FLOW_LIB_PATH", ',');
            if let Ok(FoundType::Directory(found_at)) = lib_path.find_type("flowstdlib", FileType::Directory){
                println!("The 'flowstdlib' lib directory was found at: '{}'", found_at.to_string_lossy());
            } else {
                println!("The 'flowstdlib' lib directory could not be found");
            }
        }
    }

    Ok(())
}