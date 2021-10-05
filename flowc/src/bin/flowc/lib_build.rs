use std::collections::HashSet;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use colored::*;
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
use flowcore::lib_provider::{MetaProvider, Provider};

use crate::errors::*;
use crate::Options;

const GET_MANIFEST_HEADER: &str = "
/// Return the LibraryManifest for this library
pub fn get_manifest() -> Result<LibraryManifest> {
    let metadata = MetaData {
        name: env!(\"CARGO_PKG_NAME\").into(),
        version: env!(\"CARGO_PKG_VERSION\").into(),
        description: env!(\"CARGO_PKG_DESCRIPTION\").into(),
        authors: env!(\"CARGO_PKG_AUTHORS\")
            .split(':')
            .map(|s| s.to_string())
            .collect(),
    };
    let lib_url = Url::parse(&format!(\"lib://{}\", metadata.name))?;
    let mut manifest = LibraryManifest::new(lib_url, metadata);\n
";

/// Build a library from source and generate a manifest for it so it can be used at runtime when
/// a flow referencing it is loaded and ran
pub fn build_lib(options: &Options, provider: &dyn Provider) -> Result<String> {
    let metadata = loader::load_metadata(&options.url, provider).chain_err(|| {
        format!(
            "Could not load Library metadata from '{}'",
            options.output_dir.display()
        )
    })?;

    let name = metadata.name.clone();
    println!(
        "   {} {} v{} ({})",
        "Compiling".green(),
        metadata.name,
        metadata.version,
        options.url
    );
    let lib_url = Url::parse(&format!("lib://{}", metadata.name))?;
    let mut lib_manifest = LibraryManifest::new(lib_url, metadata);

    let mut base_dir = options.output_dir.display().to_string();
    // ensure basedir always ends in '/'
    if !base_dir.ends_with('/') {
        base_dir = format!("{}/", base_dir);
    }

    let build_count =
        compile_implementations(options, &mut lib_manifest, &base_dir, provider, false)
            .chain_err(|| "Could not compile implementations in library")?;

    let manifest_json_file = json_manifest_file(&options.output_dir);
    let manifest_rust_file = rust_manifest_file(&options.output_dir);
    let json_manifest_exists = manifest_json_file.exists() && manifest_json_file.is_file();
    let rust_manifest_exists = manifest_rust_file.exists() && manifest_rust_file.is_file();

    if json_manifest_exists && rust_manifest_exists {
        if build_count > 0 {
            info!("Library manifest file(s) exists, but implementations were built, updating manifest file(s)");
            write_lib_json_manifest(&lib_manifest, &manifest_json_file)?;
            write_lib_rust_manifest(&lib_manifest, &manifest_rust_file)?;
        } else {
            let provider = MetaProvider::new(Simpath::new(""));
            let json_manifest_file_as_url = Url::from_file_path(&manifest_json_file)
                .map_err(|_| "Could not parse Url from file path")?;
            if let Ok((existing_json_manifest, _)) =
                LibraryManifest::load(&provider, &json_manifest_file_as_url)
            {
                if existing_json_manifest != lib_manifest {
                    info!("Library manifest exists, but new manifest has changes, updating manifest file(s)");
                    write_lib_json_manifest(&lib_manifest, &manifest_json_file)?;
                    write_lib_rust_manifest(&lib_manifest, &manifest_rust_file)?;
                } else {
                    info!(
                        "Existing manifest files at '{}' are up to date",
                        json_manifest_file_as_url
                    );
                }
            } else {
                info!("Could not load existing Library manifest to compare, writing new manifest file(s)");
                write_lib_json_manifest(&lib_manifest, &manifest_json_file)?;
                write_lib_rust_manifest(&lib_manifest, &manifest_rust_file)?;
            }
        }
    } else {
        // no existing manifest, so just write the one we've built
        info!("Library manifest file(s) missing, writing new manifest file(s)");
        write_lib_json_manifest(&lib_manifest, &manifest_json_file)?;
        write_lib_rust_manifest(&lib_manifest, &manifest_rust_file)?;
    }

    Ok(format!("    {} {}", "Finished".green(), name))
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
    let mut manifest_file = File::create(&json_manifest_filename)?;

    manifest_file.write_all(
        serde_json::to_string_pretty(lib_manifest)
            .chain_err(|| "Could not pretty format the library manifest JSON contents")?
            .as_bytes(),
    )?;

    info!(
        "Generated library JSON manifest at '{}'",
        json_manifest_filename.display()
    );

    Ok(())
}

// take a name like 'duplicate_rows' and remove underscores and camel case it to 'DuplicateRows'
fn camel_case(original: &str) -> String {
    // split into parts by '_' and Uppercase the first character of the (ASCII) Struct name
    let words: Vec<String> = original
        .split('_')
        .map(|w| format!("{}{}", (&w[..1].to_string()).to_uppercase(), &w[1..]))
        .collect();
    // recombine
    words.join("")
}

/*
    Generate a manifest for the library in rust for static linking
*/
#[allow(clippy::unnecessary_wraps)]
fn write_lib_rust_manifest(
    lib_manifest: &LibraryManifest,
    rust_manifest_filename: &Path,
) -> Result<()> {
    // Create the file we will be writing to
    let mut manifest_file = File::create(&rust_manifest_filename)?;

    // Create the list of top level modules
    let mut modules = HashSet::<&str>::new();
    for module_url in lib_manifest.locators.keys() {
        let module_name = module_url
            .path_segments()
            .chain_err(|| "Could not get path segments")?
            .into_iter()
            .next()
            .chain_err(|| "Could not get first path segment")?;

        modules.insert(module_name);
    }

    // generate their pub mod statements
    for module in modules {
        manifest_file.write_all(format!("\n/// functions from module '{}'", module).as_bytes())?;

        manifest_file.write_all(format!("\npub mod {};\n", module).as_bytes())?;
    }

    // generate the get_manifest() function header
    manifest_file.write_all(GET_MANIFEST_HEADER.as_bytes())?;

    // generate all the manifest entries
    for reference in lib_manifest.locators.keys() {
        let parts: Vec<&str> = reference
            .path_segments()
            .chain_err(|| "Could not get Location segments")?
            .collect::<Vec<&str>>();

        let implementation_struct = format!(
            "{}::{}",
            parts[0..parts.len() - 1].join("::"),
            self::camel_case(parts[2])
        );

        let manifest_entry = format!(
            "    manifest.locators.insert(
            Url::parse(\"{}\")?,
            Native(Arc::new({})),
        );\n\n",
            reference, implementation_struct
        );

        manifest_file.write_all(manifest_entry.as_bytes())?;
    }

    // close the get_manifest() function
    manifest_file.write_all("    Ok(manifest)\n}".as_bytes())?;

    info!(
        "Generated library Rust manifest at '{}'",
        rust_manifest_filename.display()
    );

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
    provider: &dyn Provider,
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

        match load(
            &url,
            provider,
            #[cfg(feature = "debugger")]
            &mut lib_manifest.source_urls,
        ) {
            Ok(FunctionProcess(ref mut function)) => {
                let (wasm_abs_path, built) = compile_wasm::compile_implementation(
                    function,
                    skip_building,
                    #[cfg(feature = "debugger")]
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
                        flow,
                        &output_dir.to_path_buf(),
                        provider,
                        options.dump,
                        options.graphs,
                    )
                    .chain_err(|| "Failed to dump flow's definition")?;

                    if options.graphs {
                        dump_flow::generate_svgs(&options.output_dir.to_string_lossy())?;
                    }
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
