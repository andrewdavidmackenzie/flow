extern crate flowclib;
extern crate flowrlib;
extern crate glob;
extern crate phf_codegen;
extern crate toml;

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

use flowclib::model::name::HasName;
use flowclib::model::process::Process;
use flowclib::model::process::Process::FunctionProcess;
use glob::glob;

fn main() -> Result<(), String> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let lib_name = env!("CARGO_PKG_NAME");

    let _manifest = build_manifest(lib_name, &out_dir);

    Ok(())
}

fn build_manifest(lib_name: &str, out_dir: &str) -> HashMap<String, String>{
    println!("Building manifest for '{}' in output directory: '{}'\n", lib_name, out_dir);
    let mut manifest = HashMap::<String, String>::new();

    let search_pattern = "src/**/*.toml";
    println!("Searching for process definitions using search pattern: '{}':\n",
             search_pattern);
    for entry in glob(search_pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(ref path) => {
                match load_process(path) {
                    Ok(process) => {
                        match process {
                            FunctionProcess(function) => {
                                let _ = add_to_manifest(lib_name, path,
                                                        function.name(),
                                                        &mut manifest);
                            }
                            _ => {
                                /* Ignore valid flow definitions */
                                println!("Skipping flow definition at '{:?}'", path);
                            }
                        }
                    }
                    Err(e) => {
                        /* Ignore problems from other .toml files */
                        println!("Found invalid process definition at '{:?}', skipping. Error = {}",
                                 path, e);
                    }
                }
            }
            Err(e) => println!("{:?}", e),
        }
    }

    manifest
}

fn load_process(path: &PathBuf) -> Result<Process, String> {
    let content = fs::read_to_string(&path).map_err(
        |e| format!("Could not load content from '{}', {}",
                    path.to_str().unwrap(), e))?;
    toml::from_str(&content).map_err(|e| e.to_string())
}

fn add_to_manifest(lib_name: &str, path: &PathBuf, function_name: &str,
                   manifest: &mut HashMap<String, String>)
                   -> Result<(), String> {
    let subpath = path.strip_prefix("src/")
        .expect("Could not strip off leading 'src/'");
    let subpath_str = subpath
        .to_str()
        .expect("Could not convert to str")
        .replace(".toml", "");

    let impl_reference = format!("//{}/{}/{}", lib_name, subpath_str, function_name);
    println!("Adding function to manifest: '{}'", impl_reference);
    let implementation = format!("");
    manifest.insert(impl_reference, implementation);

    return Ok(());
}


// use flowrlib::implementation_table::ImplementationLocatorTable;
// use flowrlib::implementation_table::ImplementationLocator;
//
// let mut impl_locator_table = ImplementationLocatorTable::new();
//
// for implementation in manifest {
// locator_str = "";
//    println!(
//    );
//      ImplementationLocator {
//          Native(&'a Implementation),
//      }
//
// }