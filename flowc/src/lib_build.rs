use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

use log::{debug, info};
use url::Url;

use flowclib::compiler::loader;
use flowclib::deserializers::deserializer_helper::get_deserializer;
use flowclib::model::name::HasName;
use flowclib::model::process::Process::{FunctionProcess, FlowProcess};
use flowrlib::lib_manifest::DEFAULT_LIB_MANIFEST_FILENAME;
use flowrlib::lib_manifest::LibraryManifest;
use flowrlib::provider::Provider;
use glob::glob;
use provider::content::file_provider::FileProvider;

use crate::compile_wasm;
use crate::errors::*;
use crate::Options;

/// Build a library from source and generate a manifest for it so it can be used at runtime when
/// a flow referencing it is loaded and ran
pub fn build_lib(options: &Options, provider: &dyn Provider) -> Result<String> {
    let metadata = loader::load_metadata(&options.url.to_string(), provider)
        .chain_err(|| format!("Could not load Library from '{}'", options.output_dir.display()))?;

    info!("Building manifest for '{}' library", metadata.library_name);
    let mut lib_manifest = LibraryManifest::new(metadata);

    let mut base_dir = options.output_dir.display().to_string();
    // ensure basedir always ends in '/'
    if !base_dir.ends_with('/') {
        base_dir = format!("{}/", base_dir);
    }

    let build_count = compile_implementations(&mut lib_manifest, &base_dir, provider,
                                              options.skip_generation, options.release)
        .chain_err(|| "Could not build library")?;

    let manifest_file = manifest_file(&options.output_dir);
    let manifest_exists = manifest_file.exists() && manifest_file.is_file();

    if manifest_exists {
        if build_count > 0 {
            info!("Library manifest exists, but implementations were built, so updating manifest file");
            write_lib_manifest(&lib_manifest, &manifest_file)?;
        } else {
            let provider = &FileProvider {} as &dyn Provider;
            let manifest_file_as_url = Url::from_file_path(&manifest_file).unwrap().to_string();
            if let Ok((existing_manifest, _)) = LibraryManifest::load(provider, &manifest_file_as_url) {
                if existing_manifest != lib_manifest {
                    info!("Library manifest exists, but new manifest has changes, so updating manifest file");
                    write_lib_manifest(&lib_manifest, &manifest_file)?;
                } else {
                    info!("Existing manifest at '{}' is up to date", manifest_file_as_url);
                }
            } else {
                info!("Could not load existing Library manifest to compare, so writing new manifest file");
                write_lib_manifest(&lib_manifest, &manifest_file)?;
            }
        }
    } else {
        // no existing manifest, so just write the one we've built
        info!("No existing library manifest, so writing one");
        write_lib_manifest(&lib_manifest, &manifest_file)?;
    }

    Ok(format!("Library '{}' built successfully", options.url.to_string()))
}

fn manifest_file(base_dir: &PathBuf) -> PathBuf {
    let mut filename = base_dir.clone();
    filename.push(DEFAULT_LIB_MANIFEST_FILENAME.to_string());
    filename.set_extension("json");
    filename
}

/*
    Generate a manifest for the library in JSON that can be used to load it using 'flowr'
*/
fn write_lib_manifest(lib_manifest: &LibraryManifest, filename: &PathBuf) -> Result<()> {
    let mut manifest_file = File::create(&filename).chain_err(|| "Could not create lib manifest file")?;

    manifest_file.write_all(serde_json::to_string_pretty(lib_manifest)
        .chain_err(|| "Could not pretty format the library manifest JSON contents")?
        .as_bytes()).chain_err(|| "Could not write library smanifest data bytes to created manifest file")?;

    info!("Generated library manifest at '{}'", filename.display());

    Ok(())
}

/*
    Find all process definitions under the base_dir and if they provide an implementation, check if
    the wasm file is up-to-date with the source and if not compile it, and add them all to the
    manifest struct
*/
fn compile_implementations(lib_manifest: &mut LibraryManifest, base_dir: &str, provider: &dyn Provider,
                           skip_building: bool, release: bool) -> Result<i32> {
    let mut build_count = 0;
    let search_pattern = format!("{}**/*.toml", base_dir);

    debug!("Searching for process definitions using search pattern: '{}'", search_pattern);
    for entry in glob(&search_pattern).chain_err(|| "Failed to read glob pattern")? {
        if let Ok(ref toml_path) = entry {
            let resolved_url = Url::from_file_path(&toml_path)
                .map_err(|_| format!("Could not create url from file path '{}'",
                                     toml_path.to_str().unwrap()))?.to_string();
            debug!("Inspecting '{}' for function definition", resolved_url);
            let contents = provider.get_contents(&resolved_url)
                .chain_err(|| format!("Could not get contents of resolved url: '{}'", resolved_url))?;
            let deserializer = get_deserializer(&resolved_url)?;

            match deserializer.deserialize(&String::from_utf8(contents).unwrap(), Some(&resolved_url)) {
                Ok(FunctionProcess(ref mut function)) => {
                    function.set_source_url(&resolved_url);
                    let (wasm_abs_path, built) = compile_wasm::compile_implementation(function,
                                                                                      skip_building, release)?;
                    let wasm_dir = wasm_abs_path.parent()
                        .chain_err(|| "Could not get parent directory of wasm path")?;
                    lib_manifest.add_to_manifest(base_dir,
                                                 wasm_abs_path.to_str()
                                                     .chain_err(|| "Could not convert wasm_path to str")?,
                                                 wasm_dir.to_str()
                                                     .chain_err(|| "Could not convert wasm_dir to str")?,
                                                 function.name() as &str);
                    if built {
                        build_count += 1;
                    }
                },
                Ok(FlowProcess(ref _flow)) => {},
                Err(_) => debug!("Skipping file '{}'", resolved_url)
            }
        }
    }

    if build_count > 0 {
        info!("Compiled {} functions to wasm", build_count);
    }

    Ok(build_count)
}