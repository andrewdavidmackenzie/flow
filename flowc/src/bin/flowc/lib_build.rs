use std::fs;
use std::path::Path;
use std::path::PathBuf;

use colored::*;
use log::{debug, info};
use simpath::Simpath;
use url::Url;
use wax::Glob;

use flowclib::compiler::{compile, compile_wasm};
use flowclib::compiler::parser;
use flowclib::dumper::flow_to_dot;
use flowcore::meta_provider::MetaProvider;
use flowcore::model::lib_manifest::LibraryManifest;
use flowcore::model::process::Process::{FlowProcess, FunctionProcess};
use flowcore::provider::Provider;

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

    // compile all functions to the output directory first, as they maybe referenced later in flows
    let mut file_count = compile_functions(
        lib_root_path.join("src"),
        options,
        &mut lib_manifest,
        provider,
    )?;

    file_count += compile_flows(
        lib_root_path.join("src"),
        options,
        &mut lib_manifest,
        provider,
    )?;

    file_count += copy_docs(
        lib_root_path.join("src"),
        options,
    )?;

    let manifest_json_file = LibraryManifest::manifest_filename(&options.output_dir);

    let (message, write_manifest) = check_manifest_status(&manifest_json_file, file_count,
                                                          &lib_manifest)?;

    info!("{}", message);

    if write_manifest {
        lib_manifest.write_json(&manifest_json_file)?;
    }

    println!("    {} {name}", "Finished".green());
    Ok(())
}

/*
    Check if a new manifest needs to be generated on disk based on timestamps and changed contents
*/
fn check_manifest_status(manifest_json_file: &PathBuf, file_count: i32,
                         lib_manifest: &LibraryManifest) -> Result<(&'static str, bool)> {
    let json_manifest_exists = manifest_json_file.exists() && manifest_json_file.is_file();
    if json_manifest_exists {
        if file_count > 0 {
            Ok(("Library manifest file(s) exists, but files were modified", true))
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
   Copy definition toml file for function or flow into the output dir
*/
fn copy_definition_to_output_dir(toml_path: &Path, output_dir: &Path) -> Result<i32> {
    let mut file_count = 0;

    // copy the definition toml to output directory
    fs::copy(
        toml_path,
        output_dir.join(
            toml_path
                .file_name()
                .ok_or("Could not get Toml file filename")?,
        ),
    )?;
    file_count += 1;

    Ok(file_count)
}

/*
    Find all function definitions under the base_dir and if they provide an implementation, check if
    the wasm file is up-to-date with the source and if not compile it, and add them all to the
    manifest struct
*/
fn compile_functions(
    lib_root_path: PathBuf,
    options: &Options,
    lib_manifest: &mut LibraryManifest,
    provider: &dyn Provider,
) -> Result<i32> {
    let mut file_count = 0;
    // Function implementations are described in .toml format and can be at multiple levels in
    // a library's directory structure.

    debug!(
        "Searching for function definitions using search pattern: '{}/**/*.toml'",
        lib_root_path.display(),
    );

    let glob = Glob::new("**/*.toml").map_err(|_| "Globbing error")?;
    for entry in glob.walk(&lib_root_path) {
        match &entry {
            Ok(walk_entry) => {
                let toml_path = walk_entry.path();

                let url = Url::from_file_path(toml_path).map_err(|_| {
                    format!(
                        "Could not create url from file path '{}'",
                        toml_path.display()
                    )
                })?;

                debug!("Trying to load library FunctionProcess from '{}'", url);
                match parser::parse(
                    &url,
                    provider,
                ) {
                    Ok(FunctionProcess(ref mut function)) => {
                        // calculate the path of the file's directory, relative to lib_root
                        let relative_dir = toml_path
                            .parent()
                            .ok_or("Could not get toml path parent dir")?
                            .strip_prefix(&lib_root_path)
                            .map_err(|_| "Could not calculate relative_dir")?;
                        // calculate the target directory for generating output using the relative path from the
                        // lib_root appended to the root of the output directory
                        let output_dir = options.output_dir.join(relative_dir);
                        if !output_dir.exists() {
                            fs::create_dir_all(&output_dir)?;
                        }

                        let (source_path, wasm_destination) = compile::get_paths(&output_dir, function)?;

                        // here we assume that the library has a workspace at lib_root_path
                        let mut target_dir = lib_root_path.clone();

                        if options.optimize {
                            target_dir.push("target/wasm32-unknown-unknown/release/");
                        } else {
                            target_dir.push("target/wasm32-unknown-unknown/debug/");
                        }

                        let wasm_relative_path = wasm_destination
                            .strip_prefix(&options.output_dir)
                            .map_err(|_| "Could not calculate wasm_relative_path")?;

                        let built = compile_wasm::compile_implementation(
                            output_dir.as_path(),
                            target_dir,
                            &wasm_destination,
                            &source_path,
                            function,
                            options.native_only,
                            options.optimize,
                            #[cfg(feature = "debugger")]
                            &mut lib_manifest.source_urls,
                        ).chain_err(|| "Could not compile implementation to wasm")?;

                        if built {
                            file_count += 1;
                        }

                        lib_manifest
                            .add_locator(
                                &wasm_relative_path.to_string_lossy(),
                                &relative_dir.to_string_lossy(),
                                #[cfg(feature = "debugger")]
                                &source_path.to_string_lossy(),
                            )
                            .chain_err(|| "Could not add entry to library manifest")?;

                        file_count += copy_definition_to_output_dir(toml_path, &output_dir)?;
                    }
                    Ok(FlowProcess(_)) => {},
                    Err(err) => debug!("Skipping file '{}'. Reason: '{}'", url, err),
                }
            },
            Err(e) => bail!("Error walking glob entries: {}", e.to_string())
        }
    }

    if file_count > 0 {
        info!("Compiled {} functions to wasm", file_count);
    }

    Ok(file_count)
}

/*
    Find all flow definitions under the base_dir, copy to target and add them all to the manifest
*/
fn compile_flows(
    lib_root_path: PathBuf,
    options: &Options,
    lib_manifest: &mut LibraryManifest,
    provider: &dyn Provider,
) -> Result<i32> {
    let mut file_count = 0;
    // Flow implementations are described in .toml format and can be at multiple levels in
    // a library's directory structure.

    debug!(
        "Searching for flow definitions using search pattern: '{}/**/*.toml'",
        lib_root_path.display(),
    );

    let glob = Glob::new("**/*.toml").map_err(|_| "Globbing error")?;
    for entry in glob.walk(&lib_root_path) {
        match &entry {
            Ok(walk_entry) => {
                let toml_path = walk_entry.path();

                let url = Url::from_file_path(toml_path).map_err(|_| {
                    format!(
                        "Could not create url from file path '{}'",
                        toml_path.display()
                    )
                })?;

                debug!("Trying to load library FlowProcess from '{}'", url);
                match parser::parse(
                    &url,
                    provider,
                ) {
                    Ok(FunctionProcess(_)) => {}
                    Ok(FlowProcess(ref mut flow)) => {
                        // calculate the path of the file's directory, relative to lib_root
                        let relative_dir = toml_path
                            .parent()
                            .ok_or("Could not get toml path parent dir")?
                            .strip_prefix(&lib_root_path)
                            .map_err(|_| "Could not calculate relative_dir")?;
                        // calculate the target directory for generating output using the relative path from the
                        // lib_root appended to the root of the output directory
                        let output_dir = options.output_dir.join(relative_dir);
                        if !output_dir.exists() {
                            fs::create_dir_all(&output_dir)?;
                        }

                        if options.graphs {
                            flow_to_dot::dump_flow(flow, &output_dir, provider)?;
                            flow_to_dot::generate_svgs(&output_dir, true)?;
                        }

                        file_count += copy_definition_to_output_dir(toml_path, &output_dir)?;

                        let flow_relative_path = toml_path
                            .strip_prefix(&lib_root_path)
                            .map_err(|_| "Could not calculate relative_path")?;
                        let flow_lib_reference = flow_relative_path.file_stem()
                            .ok_or("Could not remove extension from flow file path")?
                            .to_string_lossy();

                        lib_manifest
                            .add_locator(
                                &flow_relative_path.to_string_lossy(),
                                &flow_lib_reference,
                                #[cfg(feature = "debugger")]
                                &toml_path.to_string_lossy()
                            )
                            .chain_err(|| "Could not add entry to library manifest")?;
                    }
                    Err(err) => debug!("Skipping file '{}'. Reason: '{}'", url, err),
                }
            },
            Err(e) => bail!("Error walking glob entries: {}", e.to_string())
        }
    }

    if file_count > 0 {
        info!("Compiled {} flows", file_count);
    }

    Ok(file_count)
}


/*
    Find all document files not already copied and copy them to the destination folder tree
*/
fn copy_docs(
    lib_root_path: PathBuf,
    options: &Options,
) -> Result<i32> {
    let mut file_count = 0;
    debug!(
        "Searching for additional docs files using search pattern: '{}/**/*.md'",
        lib_root_path.display(),
    );

    let glob = Glob::new("**/*.md").map_err(|_| "Globbing error")?;
    for entry in glob.walk(&lib_root_path) {
        match &entry {
            Ok(walk_entry) => {
                let md_path = walk_entry.path();

                // calculate the path of the file, relative to lib_root
                let relative_file_path = md_path
                    .strip_prefix(&lib_root_path)
                    .map_err(|_| "Could not calculate relative path")?;
                // calculate the target file for copying to using the relative path from the
                // lib_root appended to the output directory
                let target_file = options.output_dir.join(relative_file_path);

                if !target_file.exists() {
                    // copy the md file from the source tree to the target tree if not already there
                    fs::copy(md_path, target_file).map_err(|_| "Could not copy docs file")?;
                    file_count += 1;
                }

            },
            Err(e) => bail!("Error walking glob entries: {}", e.to_string())
        }
    }

    if file_count > 0 {
        info!("Copied {} doc files", file_count);
    }

    Ok(file_count)
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