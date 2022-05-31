use std::fs::File;
use std::io::{self, Read};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;
use simpath::Simpath;
use tempdir::TempDir;
use url::Url;

use flowcore::{DONT_RUN_AGAIN, Implementation, RunAgain};
use flowcore::errors::Result;
use flowcore::meta_provider::MetaProvider;
use flowcore::model::flow_manifest::FlowManifest;
use flowcore::model::lib_manifest::{ImplementationLocator::Native, LibraryManifest};
use flowcore::model::metadata::MetaData;
use flowcore::model::runtime_function::RuntimeFunction;
use flowrlib::loader::Loader;

/// flowrlib integration tests
///
/// These tests are in flowr and not flowrlib as we want to keep flowrlib free of anything that
/// prevents it from being compiled to wasm32 for use in a browser or other wasm environment.
///
/// These tests use pre-written files on the file system and use filesystem
/// io to read them, which cannot be compiled to wasm, as no such concept (as stdio) exists in a
/// generic wasm execution environment.
///
/// They could be written as code (not reading files) and hence in flowclib, but that's quite
/// a lot of work to construct each flow in code, and so for now I've taken the easy route to write
/// some test flow toml files and just compile those. Plus that also stresses the deserialization
/// and parsing.
///
/// An interim solution could be so have the files in the code as Strings and parse from there.
///

fn create_test_flow_manifest(functions: Vec<RuntimeFunction>) -> FlowManifest {
    let metadata = MetaData {
        name: "test manifest".into(),
        description: "test manifest".into(),
        version: "0.0".into(),
        authors: vec!["me".into()],
    };

    let mut manifest = FlowManifest::new(metadata);

    for function in functions {
        let location_url = &Url::parse(function.implementation_location())
            .expect("Could not create Url");
        match location_url.scheme() {
            "lib" => manifest.add_lib_reference(location_url),
            "context" => manifest.add_context_reference(location_url),
            _ => {}
        }
        manifest.add_function(function);
    }

    manifest
}

#[derive(Debug)]
struct Fake;

impl Implementation for Fake {
    fn run(&self, mut _inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let mut value = None;

        let mut buffer = String::new();
        if let Ok(size) = io::stdin().lock().read_to_string(&mut buffer) {
            if size > 0 {
                let input = Value::String(buffer.trim().to_string());
                value = Some(input);
            }
        }

        Ok((value, DONT_RUN_AGAIN))
    }
}

fn create_test_context_manifest() -> LibraryManifest {
    let metadata = MetaData {
        name: "context".to_string(),
        description: "".into(),
        version: "0.1.0".into(),
        authors: vec!["".into()],
    };
    let lib_url = Url::parse("context://").expect("Couldn't create lib url");
    let mut manifest = LibraryManifest::new(lib_url, metadata);

    manifest.locators.insert(
        Url::parse("context://args/get/get").expect("Could not create Url"),
        Native(Arc::new(Fake {})),
    );
    manifest.locators.insert(
        Url::parse("context://file/file_write/file_write").expect("Could not create Url"),
        Native(Arc::new(Fake {})),
    );
    manifest.locators.insert(
        Url::parse("context://stdio/readline/readline").expect("Could not create Url"),
        Native(Arc::new(Fake {})),
    );
    manifest.locators.insert(
        Url::parse("context://stdio/stdin/stdin").expect("Could not create Url"),
        Native(Arc::new(Fake {})),
    );
    manifest.locators.insert(
        Url::parse("context://stdio/stdout/stdout").expect("Could not create Url"),
        Native(Arc::new(Fake {})),
    );
    manifest.locators.insert(
        Url::parse("context://stdio/stderr/stderr").expect("Could not create Url"),
        Native(Arc::new(Fake {})),
    );

    manifest
}

fn write_manifest(manifest: &FlowManifest, filename: &Path) -> Result<()> {
    let mut manifest_file =
        File::create(&filename).map_err(|_| "Could not create lib manifest file")?;

    manifest_file
        .write_all(
            serde_json::to_string_pretty(manifest)
                .map_err(|_| "Could not pretty format the manifest JSON contents")?
                .as_bytes(),
        )
        .map_err(|_| "Could not write manifest data bytes to created manifest file")?;

    Ok(())
}

#[test]
fn load_manifest_from_file() {
    let f_a = RuntimeFunction::new(
        #[cfg(feature = "debugger")] "fA",
        #[cfg(feature = "debugger")] "/fA",
        "context://stdio/stdout/stdout",
        vec![],
        0,
        0,
        &[],
        false,
    );
    let functions = vec![f_a];
    let manifest = create_test_flow_manifest(functions);

    let temp_dir = TempDir::new("flow").expect("Could not get temp dir").into_path();
    let manifest_file = temp_dir.join("manifest.json");
    write_manifest(&manifest, &manifest_file).expect("Could not write manifest file");
    let manifest_url = Url::from_directory_path(manifest_file).expect("Could not create url from directory path");
    let provider = MetaProvider::new(Simpath::new("FLOW_LIB_PATH"),
                                     PathBuf::from("/"));

    let mut loader = Loader::new();
    loader
        .add_lib(
            &provider,
            create_test_context_manifest(),
            &Url::parse("context://").expect("Could not parse lib url"),
        )
        .expect("Could not add context library to loader");

    assert!(loader.load_flow(&provider, &manifest_url).is_ok());
}

#[test]
fn unresolved_references_test() {
    let f_a = RuntimeFunction::new(
        #[cfg(feature = "debugger")] "fA",
        #[cfg(feature = "debugger")] "/fA",
        "context://stdio/stdout/foo",
        vec![],
        0,
        0,
        &[],
        false,
    );
    let functions = vec![f_a];
    let manifest = create_test_flow_manifest(functions);

    let temp_dir = TempDir::new("flow").expect("Could not get temp dir").into_path();
    let manifest_file = temp_dir.join("manifest.json");
    write_manifest(&manifest, &manifest_file).expect("Could not write manifest file");
    let manifest_url = Url::from_directory_path(manifest_file).expect("Could not create url from directory path");
    let provider = MetaProvider::new(Simpath::new("FLOW_LIB_PATH"),
                                     PathBuf::from("/"));

    let mut loader = Loader::new();
    loader
        .add_lib(
            &provider,
            create_test_context_manifest(),
            &Url::parse("context://").expect("Could not parse lib url"),
        )
        .expect("Could not add context library to loader");

    assert!(loader
        .load_flow(&provider, &manifest_url).is_err());
}
