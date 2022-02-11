use std::fs;
use std::path::Path;

use colored::*;
use glob::glob;
use log::{debug, info};
use simpath::Simpath;
use url::Url;

use flowclib::compiler::{compile_wasm, rust_manifest};
use flowclib::compiler::{json_manifest, loader};
use flowclib::compiler::loader::LibType::RustLib;
use flowclib::dumper::dump_flow;
use flowcore::model::lib_manifest::LibraryManifest;
use flowcore::lib_provider::{MetaProvider, Provider};
use flowcore::model::name::HasName;
use flowcore::model::process::Process::{FlowProcess, FunctionProcess};

use crate::errors::*;
use crate::Options;

/// Build a library from source and generate a manifest for it so it can be used at runtime when
/// a flow referencing it is loaded and ran
pub fn build_lib(options: &Options, provider: &dyn Provider) -> Result<String> {
    let (metadata, lib_type) = loader::load_metadata(&options.source_url, provider)?;

    let name = metadata.name.clone();
    println!(
        "   {} {} v{} ({}) with 'flowc'",
        "Compiling".green(),
        metadata.name,
        metadata.version,
        options.source_url
    );
    let lib_url = Url::parse(&format!("lib://{}", metadata.name))?;
    let mut lib_manifest = LibraryManifest::new(lib_url, metadata);

    let lib_root_path = options
        .source_url
        .to_file_path()
        .map_err(|_| "Could not convert Url to File path")?;

    let build_count = compile_implementations(
        &lib_root_path,
        &options.output_dir,
        options.dump,
        options.graphs,
        &mut lib_manifest,
        provider,
        options.native_only,
    )
    .chain_err(|| "Could not compile implementations in library")?;

    let manifest_json_file = json_manifest::manifest_filename(&options.output_dir);
    let json_manifest_exists = manifest_json_file.exists() && manifest_json_file.is_file();

    let manifest_rust_file = rust_manifest::manifest_filename(&options.output_dir);
    let rust_manifest_exists = if lib_type == RustLib {
        manifest_rust_file.exists() && manifest_rust_file.is_file()
    } else {
        true // we don't care if the rust manifest exists if the lib type is not a rust lib
    };

    let (message, write_manifests) = if json_manifest_exists && rust_manifest_exists {
        if build_count > 0 {
            ("Library manifest file(s) exists, but implementations were built, writing new file(s)", true)
        } else {
            let provider = MetaProvider::new(Simpath::new(""));
            let json_manifest_file_as_url =
                Url::from_file_path(&manifest_json_file).map_err(|_| {
                    format!(
                        "Could not parse Url from file path: {}",
                        manifest_json_file.display()
                    )
                })?;
            if let Ok((existing_json_manifest, _)) =
                LibraryManifest::load(&provider, &json_manifest_file_as_url)
            {
                if existing_json_manifest != lib_manifest {
                    ("Library manifest exists, but new manifest has changes, writing new manifest file(s)", true)
                } else {
                    ("Existing manifest files are up to date", false)
                }
            } else {
                ("Could not load existing Library manifest to compare, writing new manifest file(s)", true)
            }
        }
    } else {
        (
            "Library manifest file(s) missing, writing new manifest file(s)",
            true,
        )
    };

    info!("{}", message);

    if write_manifests {
        json_manifest::write(&lib_manifest, &manifest_json_file)?;
        if lib_type == RustLib {
            rust_manifest::write(&lib_root_path, &lib_manifest, &manifest_rust_file)?;
        }
    }

    Ok(format!("    {} {}", "Finished".green(), name))
}

/*
   Copy the source files for function or flow into the target directory
*/
fn copy_sources_to_target_dir(toml_path: &Path, target_dir: &Path, docs: &str) -> Result<()> {
    // copy the definition toml to target directory
    fs::copy(
        &toml_path,
        &target_dir.join(
            toml_path
                .file_name()
                .ok_or("Could not get Toml file filename")?,
        ),
    )?;

    // Copy any docs files to target directory
    if !docs.is_empty() {
        let docs_path = toml_path.with_file_name(docs);
        fs::copy(
            &docs_path,
            &target_dir.join(docs_path.file_name().ok_or("Could not get docs filename")?),
        )?;
    }

    Ok(())
}

/*
    Find all process definitions under the base_dir and if they provide an implementation, check if
    the wasm file is up-to-date with the source and if not compile it, and add them all to the
    manifest struct
*/
fn compile_implementations(
    lib_root_path: &Path,
    output_dir: &Path,
    dump: bool,
    graphs: bool,
    lib_manifest: &mut LibraryManifest,
    provider: &dyn Provider,
    native_only: bool,
) -> Result<i32> {
    let mut build_count = 0;
    // Function implementations are described in .toml format and can be at multiple levels in
    // a library's directory structure.
    let search_pattern = format!("{}/**/*.toml", &lib_root_path.display());

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

        // calculate the path of the file's directory, relative to lib_root
        let relative_dir = toml_path
            .parent()
            .ok_or("Could not get toml path parent dir")?
            .strip_prefix(&lib_root_path)
            .map_err(|_| "Could not calculate relative_dir")?;
        // calculate the target directory for generating output using the relative path from the
        // lib_root appended to the root of the output directory
        let target_dir = output_dir.join(relative_dir);
        if !target_dir.exists() {
            fs::create_dir_all(&target_dir)?;
        }

        // Load the `FunctionProcess` or `FlowProcess` definition from the found `.toml` file
        match loader::load(
            &url,
            provider,
            #[cfg(feature = "debugger")]
            &mut lib_manifest.source_urls,
        ) {
            Ok(FunctionProcess(ref mut function)) => {
                let (wasm_abs_path, built) = compile_wasm::compile_implementation(
                    &target_dir,
                    function,
                    native_only,
                    #[cfg(feature = "debugger")]
                    &mut lib_manifest.source_urls,
                )
                .chain_err(|| "Could not compile supplied implementation to wasm")?;

                let wasm_relative_path = wasm_abs_path
                    .strip_prefix(output_dir)
                    .map_err(|_| "Could not calculate wasm_relative_path")?;

                copy_sources_to_target_dir(&toml_path, &target_dir, function.get_docs())?;

                lib_manifest
                    .add_locator(
                        &wasm_relative_path.to_string_lossy(),
                        &relative_dir.to_string_lossy(),
                        function.name() as &str,
                    )
                    .chain_err(|| "Could not add entry to library manifest")?;
                if built {
                    build_count += 1;
                }
            }
            Ok(FlowProcess(ref mut flow)) => {
                if dump || graphs {
                    dump_flow::dump_flow(flow, &target_dir, provider, dump, graphs)
                        .chain_err(|| "Failed to dump flow's definition")?;

                    if graphs {
                        dump_flow::generate_svgs(output_dir)?;
                    }
                }

                copy_sources_to_target_dir(&toml_path, &target_dir, flow.get_docs())?;
            }
            Err(_) => debug!("Skipping file '{}'", url),
        }
    }

    if build_count > 0 {
        info!("Compiled {} functions to wasm", build_count);
    }

    Ok(build_count)
}
