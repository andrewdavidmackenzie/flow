use log::info;

use flowcore::model::function_definition::FunctionDefinition;
use flowcore::model::lib_manifest::DEFAULT_LIB_RUST_MANIFEST_FILENAME;
use flowcore::model::lib_manifest::LibraryManifest;
use std::collections::HashSet;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use crate::errors::*;

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

/// Generate a manifest for the library in rust format for static linking into a runtime binary
#[allow(clippy::unnecessary_wraps)]
pub fn write(lib_root: &Path, lib_manifest: &LibraryManifest, filename: &Path) -> Result<()> {
    // Create the file we will be writing to
    let mut manifest_file = File::create(filename)?;

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

    // generate their pub mod statements, specifying a path in the original source directory
    for module in modules {
        manifest_file.write_all(format!("\n/// functions from module '{module}'").as_bytes())?;
        manifest_file.write_all(
            format!("\n#[path=\"{}/{module}/context\"]", lib_root.display()).as_bytes(),
        )?;
        manifest_file.write_all(format!("\npub mod {module};\n").as_bytes())?;
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
            FunctionDefinition::camel_case(parts[2])
        );

        let manifest_entry = format!(
            "    manifest.locators.insert(
            Url::parse(\"{reference}\")?,
            Native(Arc::new({implementation_struct})),
        );\n\n"
        );

        manifest_file.write_all(manifest_entry.as_bytes())?;
    }

    // close the get_manifest() function
    manifest_file.write_all("    Ok(manifest)\n}".as_bytes())?;

    info!(
        "Generated library Rust manifest at '{}'",
        filename.display()
    );

    Ok(())
}

/// Given an output directory, return a PathBuf to the rust format manifest that should be
/// generated inside it
pub fn manifest_filename(base_dir: &Path) -> PathBuf {
    let mut filename = base_dir.to_path_buf();
    filename.push(DEFAULT_LIB_RUST_MANIFEST_FILENAME);
    filename
}
