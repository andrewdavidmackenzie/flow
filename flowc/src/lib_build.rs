use std::path::PathBuf;

use url::Url;

use flowclib::compiler::loader;
use flowclib::deserializers::deserializer_helper::get_deserializer;
use flowclib::model::library::Library;
use flowclib::model::name::HasName;
use flowclib::model::name::Name;
use flowclib::model::process::Process::FunctionProcess;
use flowrlib::lib_manifest::ImplementationLocator::Wasm;
use flowrlib::lib_manifest::LibraryManifest;
use flowrlib::manifest::MetaData;
use flowrlib::provider::Provider;
use glob::glob;

use crate::errors::*;

/*
    Compile a Library
*/
pub fn build_lib(url: Url, _provided_implementation: bool, out_dir: PathBuf, provider: &dyn Provider) -> Result<String> {
    let library = loader::load_library(&url.to_string(), provider).expect("Could not load Library");
    build_manifest(&library, &out_dir.to_str().unwrap(), provider).expect("Could not build library manifest");
    Ok("ok".into())
}

fn build_manifest(library: &Library, out_dir: &str, provider: &dyn Provider) -> Result<LibraryManifest> {
    info!("Building manifest for '{}' in output directory: '{}'\n", library.name, out_dir);
    let mut lib_manifest = LibraryManifest::new(MetaData::from(library));

    // TODO generalize to all valid extensions
    let search_pattern = "**/*.toml";
    debug!("Searching for process definitions using search pattern: '{}':\n", search_pattern);
    for entry in glob(search_pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(ref path) => {
                let resolved_url = Url::from_file_path(&path)
                    .map_err(|_| format!("Could not create url from file path '{}'",
                                         path.to_str().unwrap()))?.to_string();
                let contents = provider.get(&resolved_url)
                    .chain_err(|| format!("Could not get contents of resolved url: '{}'", resolved_url))?;
                let deserializer = get_deserializer(&resolved_url)?;
                match deserializer.deserialize(&String::from_utf8(contents).unwrap(), Some(&resolved_url)) {
                    Ok(process) => {
                        match process {
                            FunctionProcess(function) => {
                                add_to_manifest(path, function.name(), &mut lib_manifest);
                            }
                            _ => { /* Ignore valid flow definitions */ }
                        }
                    }
                    Err(e) => Err(e).chain_err(|| format!("Could not deserialize from file {}", resolved_url))?
                }
            }
            Err(_) => {/* Skipping unreadable files */}
        }
    }

    Ok(lib_manifest)
}

fn add_to_manifest(path: &PathBuf, function_name: &Name, manifest: &mut LibraryManifest) {
    let subpath_str = path
        .to_str()
        .expect("Could not convert to str")
        .replace(".toml", "");

    let impl_reference = format!("//{}/{}/{}", manifest.metadata.name, subpath_str, function_name);
    debug!("Adding function to manifest: '{}'", impl_reference);
    let implementation_location = format!("");
    manifest.locators.insert(impl_reference, Wasm(implementation_location));
}