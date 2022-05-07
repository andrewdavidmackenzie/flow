use std::env;
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

// Helper function for tests
fn url_from_rel_path(path: &str) -> Url {
    let cwd = cwd_as_url();
    let source_file = cwd.join(file!()).expect("Could not join paths");
    source_file.join(path).expect("Could not join paths")
}

fn cwd_as_url() -> Url {
    Url::from_directory_path(env::current_dir().expect("Could not get CWD"))
        .expect("Could not create Url from path")
}

fn create_manifest(functions: Vec<RuntimeFunction>) -> FlowManifest {
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
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        if let Ok(size) = handle.read_to_string(&mut buffer) {
            if size > 0 {
                let input = Value::String(buffer.trim().to_string());
                value = Some(input);
            }
        }

        Ok((value, DONT_RUN_AGAIN))
    }
}

fn get_manifest() -> LibraryManifest {
    let metadata = MetaData {
        name: "context".to_string(),
        description: "".into(),
        version: "0.1.0".into(),
        authors: vec!["".into()],
    };
    let lib_url = Url::parse("lib://context").expect("Couldn't create lib url");
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

// Setup a lib search path so that they can find the context library that is in
// flowr/src/bin/flowr/context
fn set_lib_search_path() -> Simpath {
    let mut lib_search_path = Simpath::new("lib_search_path");
    let flowr_path_str = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Could not get project root dir")
        .join("flowr/src/bin/flowr");
    println!("flowr_path_str: {:?}", flowr_path_str);
    lib_search_path.add_directory(
        flowr_path_str
            .to_str()
            .expect("Could not get context parent directory path as string"),
    );
    lib_search_path
}

#[test]
fn load_manifest_from_file() {
    let f_a = RuntimeFunction::new(
        "fA",
        "/fA",
        "context://stdio/stdout/stdout",
        vec![],
        0,
        0,
        &[],
        false,
    );
    let functions = vec![f_a];

    let manifest = create_manifest(functions);

    let temp_dir = TempDir::new("flow").expect("Could not get temp dir").into_path();
    let manifest_file = temp_dir.join("manifest.json");
    write_manifest(&manifest, &manifest_file).expect("Could not write manifest file");
    let manifest_url = Url::from_directory_path(manifest_file).expect("Could not create url from directory path");
    let provider = MetaProvider::new(set_lib_search_path(),
                                     PathBuf::from("/"));

    let mut loader = Loader::new();
    loader
        .add_lib(
            &provider,
            get_manifest(),
            &Url::parse("lib://context").expect("Could not parse lib url"),
        )
        .expect("Could not add context library to loader");

    let _ = loader
        .load_flow(&provider, &manifest_url)
        .expect("Loader could not load flow");

    assert!(!loader.get_lib_implementations().is_empty());
}

#[test]
fn resolve_lib_implementation_test() {
    let f_a = RuntimeFunction::new(
        "fA",
        "/fA",
        "context://stdio/stdin/stdin",
        vec![],
        0,
        0,
        &[],
        false,
    );
    let functions = vec![f_a];
    let mut manifest = create_manifest(functions);
    let provider = MetaProvider::new(set_lib_search_path(),
                                     PathBuf::from("/"));
    let mut loader = Loader::new();
    let manifest_url = url_from_rel_path("manifest.json");

    // Load library functions provided
    loader
        .add_lib(&provider, get_manifest(), &cwd_as_url())
        .expect("Could not add library");

    loader
        .resolve_implementations(&provider, &mut manifest, &manifest_url)
        .expect("Could not add library");
}

#[test]
fn unresolved_lib_functions_test() {
    let f_a = RuntimeFunction::new(
        "fA",
        "/fA",
        "context://stdio/stdin/foo",
        vec![],
        0,
        0,
        &[],
        false,
    );
    let functions = vec![f_a];
    let mut manifest = create_manifest(functions);
    let provider = MetaProvider::new(set_lib_search_path(),
                                     PathBuf::from("/"));
    let mut loader = Loader::new();
    let manifest_url = url_from_rel_path("manifest.json");

    // Load library functions provided
    loader
        .add_lib(&provider, get_manifest(), &cwd_as_url())
        .expect("Could not add context library to loader");

    assert!(loader
        .resolve_implementations(&provider, &mut manifest, &manifest_url)
        .is_err());
}
