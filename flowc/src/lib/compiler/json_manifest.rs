use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use log::info;

use flowcore::lib_manifest::LibraryManifest;
use flowcore::lib_manifest::DEFAULT_LIB_JSON_MANIFEST_FILENAME;

use crate::errors::*;

/// Generate a manifest for the library in JSON that can be used to load it using 'flowr'
pub fn write(lib_manifest: &LibraryManifest, json_manifest_filename: &Path) -> Result<()> {
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

/// Given an output directory, return a PathBuf to the json format manifest that should be
/// generated inside it
pub fn manifest_filename(base_dir: &Path) -> PathBuf {
    let mut filename = base_dir.to_path_buf();
    filename.push(DEFAULT_LIB_JSON_MANIFEST_FILENAME.to_string());
    filename.set_extension("json");
    filename
}
