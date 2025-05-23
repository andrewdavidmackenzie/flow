use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use colored::Colorize;
use log::{debug, info};
use simpath::Simpath;
use url::Url;
use wax::Glob;

use flowcore::meta_provider::MetaProvider;
use flowcore::model::lib_manifest::LibraryManifest;
use flowcore::model::process::Process::{FlowProcess, FunctionProcess};
use flowcore::provider::Provider;
use flowrclib::compiler::{compile, compile_wasm};
use flowrclib::compiler::parser;
use flowrclib::dumper::flow_to_dot;

use crate::errors::{Result, ResultExt, bail};
use crate::Options;

/// Build a library from source and generate a manifest for it so it can be used at runtime when
/// a flow referencing it is loaded and ran
///
/// # Errors
///
/// Returns an error if:
/// - Library metadata cannot be parsed correctly
/// - A valid Url cannot be formed from the library name (from the meta-data)
/// - The library's source path cannot be converted to a Url
/// - The library cannot be compiled
/// - The library's manifest cannot be generated in the output folder
/// - The documentation files cannot be copied to the output folder
///
pub fn build_lib(options: &Options, provider: &dyn Provider, output_dir: &PathBuf) -> Result<()> {
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
        .map_err(|()| "Could not convert Url to File path")?;

    prepare_lib_workspace(&lib_root_path)?;

    // compile all functions to the output directory first, as they maybe referenced later in flows
    let mut file_count = compile_functions(
        &lib_root_path.join("src"),
        options,
        &mut lib_manifest,
        provider,
        output_dir,
    )?;

    file_count += compile_flows(
        &lib_root_path.join("src"),
        options,
        &mut lib_manifest,
        provider,
        output_dir,
    )?;

    file_count += copy_docs(&lib_root_path.join("src"), output_dir)?;

    let manifest_json_file = LibraryManifest::manifest_filename(output_dir);

    let (message, write_manifest) = check_manifest_status(&manifest_json_file, file_count,
                                                          &lib_manifest)?;

    info!("{message}");

    if write_manifest {
        lib_manifest.write_json(&manifest_json_file)?;
    }

    teardown_lib_workspace(&lib_root_path)?;

    println!("    {} {name}", "Finished".green());
    Ok(())
}


/// Build a runner into the `output_dir`
///
/// # Errors
///
/// Returns an error if:
/// - the `Url` constructed form the input parameter path for the flow (or the default)
///   cannot have "context" added to it to find the "context" dir
/// - the docs for the runner cannot be read or copied to the specified output dir
pub fn build_runner(options: &Options, output_dir: &Path) -> Result<()> {
    println!(
        "   {} runner ({}) with 'flowc'",
        "Compiling".green(),
        options.source_url
    );

    let runner_context_path = options
        .source_url
        .to_file_path()
        .map_err(|()| "Could not convert Url to File path")?
        .join("context");

    // compile all functions to the output directory first, as they maybe referenced later in flows
    copy_definitions(
        &runner_context_path,
        output_dir,
    )?;

    let _ = copy_docs(&runner_context_path, output_dir)?;

    println!("    {}", "Finished".green());
    Ok(())
}

// prepare the library's internal virtual workspace for building under 'src' directory,
// as this allows all functions being built to share the same target directory and built
// dependencies, greatly speeding builds
fn prepare_lib_workspace(lib_root_path: &Path) -> Result<()> {
    // ensure lib.toml exists in the root and if so copy it to src/Cargo.toml for building
    let lib_toml_path = lib_root_path.join("lib.toml");
    if !lib_toml_path.exists() {
        bail!("Flow libraries must have a valid 'lib.toml' file in the library's root directory");
    }
    let lib_src_path = lib_root_path.join("src");
    let cargo_toml = lib_root_path.join("src/Cargo.toml");
    fs::copy(lib_toml_path, cargo_toml)?;

    // copy all function.toml files to Cargo.toml files in same directory so the
    // workspace members references from lib.toml can be found

    let glob = Glob::new("**/function.toml").map_err(|_| "Globbing error")?;
    for entry in glob.walk(lib_src_path).flatten() {
        let mut cargo_toml = entry.path().to_path_buf();
        cargo_toml.set_file_name("Cargo.toml");
        fs::copy(entry.path(), cargo_toml)?;
    }

    Ok(())
}

// Delete any temporary Cargo.toml files that were created under 'src' in prepare_workspace()
// as these will prevent the directory containing them from being included in the crate when
// we attempt to publish it
fn teardown_lib_workspace(lib_root_path: &PathBuf) -> Result<()> {
    let glob = Glob::new("src/**/Cargo.toml").map_err(|_| "Globbing error")?;
    for entry in glob.walk(lib_root_path).flatten() {
        fs::remove_file(entry.path())?;
    }

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
            let provider = Arc::new(MetaProvider::new(Simpath::new(""),
                                             PathBuf::from("/"))) as Arc<dyn Provider>;
            let json_manifest_file_as_url =
                Url::from_file_path(manifest_json_file).map_err(|()| {
                    format!(
                        "Could not parse Url from file path: {}",
                        manifest_json_file.display()
                    )
                })?;
            if let Ok((existing_json_manifest, _)) =
            LibraryManifest::load(&provider, &json_manifest_file_as_url)
            {
                if &existing_json_manifest == lib_manifest {
                    Ok(("Existing manifest files are up to date", false))
                } else {
                    Ok(("Library manifest exists, but new manifest has changes, writing new manifest file(s)", true))
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
    let output_file = output_dir.join(toml_path.file_name()
                                          .ok_or("Could not get Toml file filename")?);

    println!("   {} {} to {}", "Copying".green(),
        toml_path.file_name().ok_or("Could not get file name")?.to_string_lossy(),
             output_file.display());

    fs::copy(toml_path, &output_file)?;

    Ok(1)
}

/*
    Find all function definitions under the base_dir and if they provide an implementation, check if
    the wasm file is up-to-date with the source and if not compile it, and add them all to the
    manifest struct
*/
fn compile_functions(
    lib_root_path: &PathBuf,
    options: &Options,
    lib_manifest: &mut LibraryManifest,
    provider: &dyn Provider,
    output_dir: &PathBuf,
) -> Result<i32> {
    let mut file_count = 0;
    // Function implementations are described in .toml format and can be at multiple levels in
    // a library's directory structure.

    debug!(
        "Searching for function definitions using search pattern: '{}/**/*.toml'",
        lib_root_path.display(),
    );

    let glob = Glob::new("**/*.toml").map_err(|_| "Globbing error")?;
    for entry in glob.walk(lib_root_path) {
        match &entry {
            Ok(walk_entry) => {
                let toml_path = walk_entry.path();
                let toml_filename = toml_path.file_name()
                    .ok_or("Could not get toml file name")?.to_string_lossy();
                if toml_filename == "function.toml" {
                    continue;
                }

                let url = Url::from_file_path(toml_path).map_err(|()| {
                    format!(
                        "Could not create url from file path '{}'",
                        toml_path.display()
                    )
                })?;

                debug!("Trying to load library FunctionProcess from '{url}'");
                match parser::parse(
                    &url,
                    provider,
                ) {
                    Ok(FunctionProcess(ref mut function)) => {
                        // calculate the path of the file's directory, relative to lib_root
                        let relative_dir = toml_path
                            .parent()
                            .ok_or("Could not get toml path parent dir")?
                            .strip_prefix(lib_root_path)
                            .map_err(|_| "Could not calculate relative_dir")?;
                        // calculate the target directory for generating output using the relative path from the
                        // lib_root appended to the root of the output directory
                        let out_dir = output_dir.join(relative_dir);
                        if !out_dir.exists() {
                            fs::create_dir_all(&out_dir)?;
                        }

                        let (source_path, wasm_destination) = compile::get_paths(&out_dir, function)?;

                        // here we assume that the library has a workspace at lib_root_path
                        let mut target_dir = lib_root_path.clone();

                        if options.optimize {
                            target_dir.push("target/wasm32-unknown-unknown/release/");
                        } else {
                            target_dir.push("target/wasm32-unknown-unknown/debug/");
                        }

                        let wasm_relative_path = wasm_destination
                            .strip_prefix(output_dir)
                            .map_err(|_| "Could not calculate wasm_relative_path")?;

                        let built = compile_wasm::compile_implementation(
                            out_dir.as_path(),
                            target_dir,
                            &wasm_destination,
                            &source_path,
                            function,
                            options.native_only,
                            options.optimize,
                            #[cfg(feature = "debugger")]
                            &mut lib_manifest.source_urls,
                        ).chain_err(|| format!("Could not compile implementation '{}' to wasm",
                                        source_path.display()))?;

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

                        file_count += copy_definition_to_output_dir(toml_path, &out_dir)?;
                    }
                    Ok(FlowProcess(_)) => debug!("Skipping file '{url}'. Reason: 'It is a Flow'"),
                    Err(err) => debug!("Skipping file '{url}'. Reason: '{err}'"),
                }
            },
            Err(e) => bail!("Error walking glob entries: {}", e.to_string())
        }
    }

    if file_count > 0 {
        info!("Compiled {file_count} functions to wasm");
    }

    Ok(file_count)
}

// Find all function definitions under the base_dir copy them to output dir
fn copy_definitions(
    root_path: &PathBuf,
    output_dir: &Path,
) -> Result<()> {
    // Function implementations are described in .toml format and can be at multiple levels in

    debug!(
        "Searching for function definitions using search pattern: '{}/**/*.toml'",
        root_path.display(),
    );

    let glob = Glob::new("**/*.toml").map_err(|_| "Globbing error")?;
    for entry in glob.walk(root_path) {
        match &entry {
            Ok(walk_entry) => {
                let toml_path = walk_entry.path();

                // calculate the path of the file's directory, relative to root
                let relative_dir = toml_path
                    .parent()
                    .ok_or("Could not get toml path parent dir")?
                    .strip_prefix(root_path)
                    .map_err(|_| "Could not calculate relative_dir")?;
                // calculate the output directory relative path to the root
                let out_dir = output_dir.join(relative_dir);
                if !out_dir.exists() {
                    fs::create_dir_all(&out_dir)?;
                }

                let _ = copy_definition_to_output_dir(toml_path, &out_dir)?;
            },
            Err(e) => bail!("Error walking glob entries: {}", e.to_string())
        }
    }

    Ok(())
}

/*
    Find all library flow definitions under `lib_root_path`
      - copy to target and add to the manifest

    Flow definitions are described in .toml format and can be at multiple levels in
    a library's directory structure.
*/
fn compile_flows(
    lib_root_path: &PathBuf,
    options: &Options,
    lib_manifest: &mut LibraryManifest,
    provider: &dyn Provider,
    output_dir: &Path
) -> Result<i32> {
    let mut file_count = 0;
    debug!(
        "Searching for flow definitions using search pattern: '{}/**/*.toml'",
        lib_root_path.display(),
    );

    let glob = Glob::new("**/*.toml").map_err(|_| "Globbing error")?;
    for entry in glob.walk(lib_root_path) {
        match &entry {
            Ok(walk_entry) => {
                if walk_entry.path().file_name() == Some(OsStr::new("function.toml")) ||
                   walk_entry.path().file_name() == Some(OsStr::new("Cargo.toml")) {
                    continue;
                }

                let toml_path = walk_entry.path();

                let url = Url::from_file_path(toml_path).map_err(|()| {
                    format!(
                        "Could not create url from file path '{}'",
                        toml_path.display()
                    )
                })?;

                debug!("Trying to load library FlowProcess from '{url}'");
                match parser::parse(
                    &url,
                    provider,
                ) {
                    Ok(FunctionProcess(_)) => debug!("Skipping file '{url}'. Reason: 'It is a Function'"),
                    Ok(FlowProcess(ref mut flow)) => {
                        // calculate the path of the file's directory, relative to lib_root
                        let relative_dir = toml_path
                            .parent()
                            .ok_or("Could not get toml path parent dir")?
                            .strip_prefix(lib_root_path)
                            .map_err(|_| "Could not calculate relative_dir")?;
                        // calculate the target directory for generating output using the relative path from the
                        // lib_root appended to the root of the output directory
                        let out_dir = output_dir.join(relative_dir);
                        if !out_dir.exists() {
                            fs::create_dir_all(&out_dir)?;
                        }

                        if options.graphs {
                            flow_to_dot::dump_flow(flow, &out_dir, provider)?;
                            flow_to_dot::generate_svgs(&out_dir, true)?;
                        }

                        file_count += copy_definition_to_output_dir(toml_path, &out_dir)?;

                        let flow_relative_path = toml_path
                            .strip_prefix(lib_root_path)
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
                    Err(err) => bail!("Error parsing '{}'. Reason: '{}'", url, err),
                }
            },
            Err(e) => bail!("Error walking glob entries: {}", e.to_string())
        }
    }

    if file_count > 0 {
        info!("Compiled {file_count} flows");
    }

    Ok(file_count)
}


/*
    Find all document files not already copied and copy them to the destination folder tree
*/
fn copy_docs(
    lib_root_path: &PathBuf,
    output_dir: &Path,
) -> Result<i32> {
    let mut file_count = 0;
    debug!(
        "Searching for additional docs files using search pattern: '{}/**/*.md'",
        lib_root_path.display(),
    );

    let glob = Glob::new("**/*.md").map_err(|_| "Globbing error")?;
    for entry in glob.walk(lib_root_path) {
        match &entry {
            Ok(walk_entry) => {
                let md_path = walk_entry.path();

                // calculate the path of the file, relative to lib_root
                let relative_file_path = md_path
                    .strip_prefix(lib_root_path)
                    .map_err(|_| "Could not calculate relative path")?;
                // calculate the target file for copying to using the relative path from the
                // lib_root appended to the output directory
                let target_file = output_dir.join(relative_file_path);

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
        info!("Copied {file_count} doc files");
    }

    Ok(file_count)
}

#[cfg(test)]
mod test {
    use std::io::prelude::*;

    use tempfile::tempdir;
    use url::Url;

    use flowcore::model::lib_manifest::LibraryManifest;
    use flowcore::model::metadata::MetaData;

    fn test_manifest() -> Url {
        let dir = tempdir().expect("Could not create temp dir");
        let url = Url::from_directory_path(dir.keep()).expect("Could not create Url");
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
