use std::fs;
use std::path::Path;
use std::path::PathBuf;

use colored::*;
use log::{debug, info};
use simpath::Simpath;
use url::Url;
use wax::Glob;

use flowclib::compiler::{json_manifest, parser};
use flowclib::compiler::compile_wasm;
use flowclib::dumper::{dump, dump_dot};
use flowcore::meta_provider::{MetaProvider, Provider};
use flowcore::model::lib_manifest::LibraryManifest;
use flowcore::model::process::Process::{FlowProcess, FunctionProcess};

use crate::errors::*;
use crate::Options;

/// Build a library from source and generate a manifest for it so it can be used at runtime when
/// a flow referencing it is loaded and ran
pub fn build_lib(options: &Options, provider: &dyn Provider) -> Result<()> {
    let (metadata, _) = parser::parse_metadata(&options.source_url, provider)?;

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
        options,
        &mut lib_manifest,
        provider,
    )
    .chain_err(|| "Could not compile implementations in library")?;

    let manifest_json_file = json_manifest::manifest_filename(&options.output_dir);

    let (message, write_manifest) = check_manifest_status(&manifest_json_file, build_count,
        &lib_manifest)?;

    info!("{}", message);

    if write_manifest {
        json_manifest::write(&lib_manifest, &manifest_json_file)?;
    }

    println!("    {} {}", "Finished".green(), name);
    Ok(())
}

/*
    Check if a new manifest needs to be generated on disk based on timestamps and changed contents
*/
fn check_manifest_status(manifest_json_file: &PathBuf, build_count: i32,
                         lib_manifest: &LibraryManifest) -> Result<(&'static str, bool)> {
    let json_manifest_exists = manifest_json_file.exists() && manifest_json_file.is_file();
    if json_manifest_exists {
        if build_count > 0 {
            Ok(("Library manifest file(s) exists, but implementations were built, writing new file(s)", true))
        } else {
            let provider = MetaProvider::new(Simpath::new(""),
                                             PathBuf::from("/")
            );
            let json_manifest_file_as_url =
                Url::from_file_path(manifest_json_file).map_err(|_| {
                    format!(
                        "Could not parse Url from file path: {}",
                        manifest_json_file.display()
                    )
                })?;
            if let Ok((existing_json_manifest, _)) =
            LibraryManifest::load(&provider, &json_manifest_file_as_url)
            {
                if &existing_json_manifest != lib_manifest {
                    Ok(("Library manifest exists, but new manifest has changes, writing new manifest file(s)", true))
                } else {
                    Ok(("Existing manifest files are up to date", false))
                }
            } else {
                Ok(("Could not load existing Library manifest to compare, writing new manifest file(s)", true))
            }
        }
    } else {
        Ok(("Library manifest file(s) missing, writing new manifest file(s)", true))
    }
}

/*
   Copy the source files for function or flow into the target directory
*/
fn copy_sources_to_target_dir(toml_path: &Path, target_dir: &Path, docs: &str) -> Result<()> {
    // copy the definition toml to target directory
    fs::copy(
        toml_path,
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
    options: &Options,
    lib_manifest: &mut LibraryManifest,
    provider: &dyn Provider,
) -> Result<i32> {
    let mut build_count = 0;
    // Function implementations are described in .toml format and can be at multiple levels in
    // a library's directory structure.

    debug!(
        "Searching for process definitions using search pattern: '{}/**/*.toml'",
        lib_root_path.display(),
    );

    let glob = Glob::new("**/*.toml").map_err(|_| "Globbing error")?;
    for entry in glob.walk(lib_root_path) {
        match &entry {
            Ok(walk_entry) => {
                let toml_path = walk_entry.path();

                let url = Url::from_file_path(toml_path).map_err(|_| {
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
                    .strip_prefix(lib_root_path)
                    .map_err(|_| "Could not calculate relative_dir")?;
                // calculate the target directory for generating output using the relative path from the
                // lib_root appended to the root of the output directory
                let target_dir = options.output_dir.join(relative_dir);
                if !target_dir.exists() {
                    fs::create_dir_all(&target_dir)?;
                }

                // Load the `FunctionProcess` or `FlowProcess` definition from the found `.toml` file
                match parser::parse(
                    &url,
                    provider,
                    #[cfg(feature = "debugger")]
                        &mut lib_manifest.source_urls,
                ) {
                    Ok(FunctionProcess(ref mut function)) => {
                        let (wasm_abs_path, built) = compile_wasm::compile_implementation(
                            &target_dir,
                            function,
                            options.native_only,
                            options.optimize,
                            #[cfg(feature = "debugger")]
                                &mut lib_manifest.source_urls,
                        )
                            .chain_err(|| "Could not compile supplied implementation to wasm")?;

                        let wasm_relative_path = wasm_abs_path
                            .strip_prefix(&options.output_dir)
                            .map_err(|_| "Could not calculate wasm_relative_path")?;

                        copy_sources_to_target_dir(toml_path, &target_dir, function.get_docs())?;

                        lib_manifest
                            .add_locator(
                                &wasm_relative_path.to_string_lossy(),
                                &relative_dir.to_string_lossy(),
                            )
                            .chain_err(|| "Could not add entry to library manifest")?;
                        if built {
                            build_count += 1;
                        }
                    }
                    Ok(FlowProcess(ref mut flow)) => {
                        if options.tables_dump {
                            dump::dump_flow(flow, &target_dir, provider)
                                .chain_err(|| "Failed to dump flow's definition")?;
                        }

                        if options.graphs {
                            dump_dot::dump_flow(flow, &options.output_dir, provider)?;
                            dump_dot::generate_svgs(&options.output_dir, true)?;
                        }

                        copy_sources_to_target_dir(toml_path, &target_dir, flow.get_docs())?;
                    }
                    Err(_) => debug!("Skipping file '{}'", url),
                }
            },
            Err(e) => bail!("Error walking glob entries: {}", e.to_string())
        }
    }

    if build_count > 0 {
        info!("Compiled {} functions to wasm", build_count);
    }

    Ok(build_count)
}

#[cfg(test)]
mod test {
    use std::io::prelude::*;

    use tempdir::TempDir;
    use url::Url;

    use flowcore::model::lib_manifest::LibraryManifest;
    use flowcore::model::metadata::MetaData;

    fn test_manifest() -> Url {
        let dir = TempDir::new("flow").expect("Could not create temp dir");
        let url = Url::from_directory_path(dir.into_path()).expect("Could not create Url");
        url.join("manifest.json").expect("Could not join filename to Url")
    }

    #[test]
    fn manifest_does_not_exist() {
        let lib_metadata = MetaData::default();
        let manifest_url = test_manifest();
        let lib_manifest = LibraryManifest::new(manifest_url.clone(), lib_metadata);

        let (_, generate) = super::check_manifest_status(
            &manifest_url.to_file_path().expect("Could not get back to path"),
        0, &lib_manifest).expect("Could not check manifest_status");

        assert!(generate);
    }

    #[test]
    fn manifest_exist_builds_done() {
        let lib_metadata = MetaData::default();
        let manifest_url = test_manifest();
        let lib_manifest = LibraryManifest::new(manifest_url.clone(), lib_metadata);
        let manifest_path = manifest_url.to_file_path().expect("Could not get back to path");
        std::fs::File::create(&manifest_path).expect("Could not create file");
        let (_, generate) = super::check_manifest_status(
            &manifest_path,
            1, &lib_manifest).expect("Could not check manifest_status");

        assert!(generate);
    }

    #[test]
    fn manifest_exist_builds_not_done_different_content() {
        let lib_metadata = MetaData::default();
        let manifest_url = test_manifest();
        let lib_manifest = LibraryManifest::new(manifest_url.clone(), lib_metadata);
        let manifest_path = manifest_url.to_file_path().expect("Could not get back to path");
        std::fs::File::create(&manifest_path).expect("Could not create file");
        let (_, generate) = super::check_manifest_status(
            &manifest_path,
            0, &lib_manifest).expect("Could not check manifest_status");

        assert!(generate);
    }

    #[test]
    fn manifest_exist_builds_not_done_same_content() {
        let lib_metadata = MetaData::default();
        let manifest_url = test_manifest();
        let lib_manifest = LibraryManifest::new(manifest_url.clone(), lib_metadata);
        let manifest_path = manifest_url.to_file_path().expect("Could not get back to path");
        let mut file = std::fs::File::create(&manifest_path).expect("Could not create file");
        file.write_all(
            serde_json::to_string_pretty(&lib_manifest)
                .expect("Could not pretty format the library manifest JSON contents")
                .as_bytes(),
        ).expect("Could not write to file");
        let (_, generate) = super::check_manifest_status(
            &manifest_path,
            0, &lib_manifest).expect("Could not check manifest_status");

        assert!(!generate); // No need to generate the manifest again then!
    }
}