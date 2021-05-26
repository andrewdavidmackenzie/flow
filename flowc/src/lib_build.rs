use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use glob::glob;
use log::{debug, info};
use simpath::Simpath;
use url::Url;

use flowclib::compiler::compile_wasm;
use flowclib::compiler::loader;
use flowclib::compiler::loader::load;
use flowclib::dumper::dump_flow;
use flowclib::model::name::HasName;
use flowclib::model::process::Process::{FlowProcess, FunctionProcess};
use flowcore::lib_manifest::LibraryManifest;
use flowcore::lib_manifest::{
    DEFAULT_LIB_JSON_MANIFEST_FILENAME, DEFAULT_LIB_RUST_MANIFEST_FILENAME,
};
use flowcore::lib_provider::{LibProvider, MetaProvider};

use crate::errors::*;
use crate::Options;

/// Build a library from source and generate a manifest for it so it can be used at runtime when
/// a flow referencing it is loaded and ran
pub fn build_lib(options: &Options, provider: &dyn LibProvider) -> Result<String> {
    let metadata = loader::load_metadata(&options.url, provider).chain_err(|| {
        format!(
            "Could not load Library metadata from '{}'",
            options.output_dir.display()
        )
    })?;

    info!("Building '{}' library", metadata.name);
    let mut lib_manifest = LibraryManifest::new(metadata);

    let mut base_dir = options.output_dir.display().to_string();
    // ensure basedir always ends in '/'
    if !base_dir.ends_with('/') {
        base_dir = format!("{}/", base_dir);
    }

    let build_count =
        compile_implementations(options, &mut lib_manifest, &base_dir, provider, false)
            .chain_err(|| "Could not build library")?;

    let manifest_json_file = json_manifest_file(&options.output_dir);
    let manifest_rust_file = rust_manifest_file(&options.output_dir);
    let manifest_exists = manifest_json_file.exists() && manifest_json_file.is_file();

    if manifest_exists {
        if build_count > 0 {
            info!("Library manifest exists, but implementations were built, so updating manifest file");
            write_lib_json_manifest(&lib_manifest, &manifest_json_file)?;
            write_lib_rust_manifest(&lib_manifest, &manifest_rust_file)?;
        } else {
            let provider = MetaProvider::new(Simpath::new(""));
            let manifest_file_as_url = Url::from_file_path(&manifest_json_file)
                .map_err(|_| "Could not parse Url from file path")?;
            if let Ok((existing_manifest, _)) =
                LibraryManifest::load(&provider, &manifest_file_as_url)
            {
                if existing_manifest != lib_manifest {
                    info!("Library manifest exists, but new manifest has changes, so updating manifest file");
                    write_lib_json_manifest(&lib_manifest, &manifest_json_file)?;
                    write_lib_rust_manifest(&lib_manifest, &manifest_rust_file)?;
                } else {
                    info!(
                        "Existing manifest at '{}' is up to date",
                        manifest_file_as_url
                    );
                }
            } else {
                info!("Could not load existing Library manifest to compare, so writing new manifest file");
                write_lib_json_manifest(&lib_manifest, &manifest_json_file)?;
                write_lib_rust_manifest(&lib_manifest, &manifest_rust_file)?;
            }
        }
    } else {
        // no existing manifest, so just write the one we've built
        info!("No existing library manifest, so writing one");
        write_lib_json_manifest(&lib_manifest, &manifest_json_file)?;
        write_lib_rust_manifest(&lib_manifest, &manifest_rust_file)?;
    }

    Ok(format!(
        "Library '{}' built successfully",
        options.url.to_string()
    ))
}

fn json_manifest_file(base_dir: &Path) -> PathBuf {
    let mut filename = base_dir.to_path_buf();
    filename.push(DEFAULT_LIB_JSON_MANIFEST_FILENAME.to_string());
    filename.set_extension("json");
    filename
}

fn rust_manifest_file(base_dir: &Path) -> PathBuf {
    let mut filename = base_dir.to_path_buf();
    filename.push(DEFAULT_LIB_RUST_MANIFEST_FILENAME.to_string());
    filename
}

/*
    Generate a manifest for the library in JSON that can be used to load it using 'flowr'
*/
fn write_lib_json_manifest(
    lib_manifest: &LibraryManifest,
    json_manifest_filename: &Path,
) -> Result<()> {
    let mut manifest_file = File::create(&json_manifest_filename)
        .chain_err(|| "Could not create lib json manifest file")?;

    manifest_file
        .write_all(
            serde_json::to_string_pretty(lib_manifest)
                .chain_err(|| "Could not pretty format the library manifest JSON contents")?
                .as_bytes(),
        )
        .chain_err(|| "Could not write library manifest data bytes to created manifest file")?;

    info!(
        "Generated library JSON manifest at '{}'",
        json_manifest_filename.display()
    );

    Ok(())
}

/*
    Generate a manifest for the library in rust for static linking

    TODO: Implement library rust manifest generation
*/
#[allow(clippy::unnecessary_wraps)]
fn write_lib_rust_manifest(
    _lib_manifest: &LibraryManifest,
    _rust_manifest_filename: &Path,
) -> Result<()> {
    // let mut manifest_file = File::create(&rust_manifest_filename).chain_err(|| "Could not create lib rust manifest file")?;
    //
    // manifest_file.write_all(serde_json::to_string_pretty(lib_manifest)
    //     .chain_err(|| "Could not pretty format the library manifest JSON contents")?
    //     .as_bytes()).chain_err(|| "Could not write library manifest data bytes to created manifest file")?;
    //
    // info!("Generated library JSON manifest at '{}'", rust_manifest_filename.display());

    Ok(())
}

/*
    Find all process definitions under the base_dir and if they provide an implementation, check if
    the wasm file is up-to-date with the source and if not compile it, and add them all to the
    manifest struct
*/
fn compile_implementations(
    options: &Options,
    lib_manifest: &mut LibraryManifest,
    base_dir: &str,
    provider: &dyn LibProvider,
    skip_building: bool,
) -> Result<i32> {
    let mut build_count = 0;
    let search_pattern = format!("{}**/*.toml", base_dir);

    debug!(
        "Searching for process definitions using search pattern: '{}'",
        search_pattern
    );

    for toml_path in (glob(&search_pattern).chain_err(|| "Failed to read glob pattern")?).flatten()
    {
        let url = Url::from_file_path(&toml_path).map_err(|_| {
            format!(
                "Could not create url from file path '{}'",
                toml_path.display()
            )
        })?;
        debug!("Trying to load library process from '{}'", url);

        match load(&url, provider, &mut lib_manifest.source_urls) {
            Ok(FunctionProcess(ref mut function)) => {
                let (wasm_abs_path, built) = compile_wasm::compile_implementation(
                    function,
                    skip_building,
                    &mut lib_manifest.source_urls,
                )
                .chain_err(|| "Could not compile supplied implementation to wasm")?;
                let wasm_dir = wasm_abs_path
                    .parent()
                    .chain_err(|| "Could not get parent directory of wasm path")?;
                lib_manifest
                    .add_locator(
                        base_dir,
                        wasm_abs_path
                            .to_str()
                            .chain_err(|| "Could not convert wasm_path to str")?,
                        wasm_dir
                            .to_str()
                            .chain_err(|| "Could not convert wasm_dir to str")?,
                        function.name() as &str,
                    )
                    .chain_err(|| "Could not add entry to library manifest")?;
                if built {
                    build_count += 1;
                }
            }
            Ok(FlowProcess(ref mut flow)) => {
                if options.dump || options.graphs {
                    // Dump the dot file alongside the definition file
                    let source_path = flow.source_url.to_file_path().map_err(|_| {
                        "Could not convert flow's source_url Url to a Path".to_string()
                    })?;
                    let output_dir = source_path
                        .parent()
                        .chain_err(|| "Could not get parent directory of flow's source_url")?;

                    dump_flow::dump_flow(
                        &flow,
                        &output_dir.to_path_buf(),
                        provider,
                        options.dump,
                        options.graphs,
                    )
                    .chain_err(|| "Failed to dump flow's definition")?;
                }
            }
            Err(_) => debug!("Skipping file '{}'", url),
        }
    }

    if build_count > 0 {
        info!("Compiled {} functions to wasm", build_count);
    }

    Ok(build_count)
}
