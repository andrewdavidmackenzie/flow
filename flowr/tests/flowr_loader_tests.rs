use std::env;
use std::io::{self, Read};
use std::sync::Arc;

use flow_impl::{DONT_RUN_AGAIN, Implementation, RunAgain};
use serde_json::Value;
use url::Url;

use flowrlib::function::Function;
use flowrlib::lib_manifest::{ImplementationLocator::Native, LibraryManifest};
use flowrlib::loader::Loader;
use flowrlib::manifest::{Manifest, MetaData};
use provider::content::provider::MetaProvider;

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
fn url_from_rel_path(path: &str) -> String {
    let cwd = Url::from_file_path(env::current_dir().unwrap()).unwrap();
    let source_file = cwd.join(file!()).unwrap();
    let file = source_file.join(path).unwrap();
    file.to_string()
}

fn cwd_as_url() -> Result<Url, String> {
    Url::from_directory_path(
        env::current_dir()
            .map_err(|_| "Could not get the value of the current working directory".to_string())?)
        .map_err(|_| "Could not form a Url for the current working directory".into())
}

fn create_manifest(functions: Vec<Function>) -> Manifest {
    let metadata = MetaData {
        name: "test manifest".into(),
        description: "test manifest".into(),
        version: "0.0".into(),
        author_name: "me".into(),
        author_email: "me@a.com".into(),
    };

    let mut manifest = Manifest::new(metadata);

    for function in functions {
        manifest.add_function(function);
    }

    manifest
}

#[derive(Debug)]
struct Fake;

impl Implementation for Fake {
    fn run(&self, mut _inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
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

        (value, DONT_RUN_AGAIN)
    }
}

pub fn get_manifest() -> LibraryManifest {
    let metadata = MetaData {
        name: "".to_string(),
        description: "".into(),
        version: "0.1.0".into(),
        author_name: "".into(),
        author_email: "".into(),
    };
    let mut manifest = LibraryManifest::new(metadata);

    manifest.locators.insert("lib://runtime/args/get/Get".to_string(), Native(Arc::new(Fake {})));
    manifest.locators.insert("lib://runtime/file/file_write/FileWrite".to_string(), Native(Arc::new(Fake {})));
    manifest.locators.insert("lib://runtime/stdio/readline/Readline".to_string(), Native(Arc::new(Fake {})));
    manifest.locators.insert("lib://runtime/stdio/stdin/Stdin".to_string(), Native(Arc::new(Fake {})));
    manifest.locators.insert("lib://runtime/stdio/stdout/Stdout".to_string(), Native(Arc::new(Fake {})));
    manifest.locators.insert("lib://runtime/stdio/stderr/Stderr".to_string(), Native(Arc::new(Fake {})));

    manifest
}

#[test]
fn resolve_lib_implementation_test() {
    let f_a = Function::new("fA".to_string(), // name
                            "/context/fA".to_string(),
                            "lib://runtime/stdio/stdin/Stdin".to_string(),
                            vec!(),
                            0,
                            &vec!());
    let functions = vec!(f_a);
    let mut manifest = create_manifest(functions);
    let provider = MetaProvider {};
    let mut loader = Loader::new();
    let manifest_url = url_from_rel_path("manifest.json");

    // Load library functions provided
    loader.add_lib(&provider, get_manifest(), &cwd_as_url().unwrap().to_string()).unwrap();

    loader.resolve_implementations(&mut manifest, &provider, &manifest_url).unwrap();
}

#[test]
fn unresolved_lib_functions_test() {
    let f_a = Function::new("fA".to_string(), // name
                            "/context/fA".to_string(),
                            "lib://runtime/stdio/stdin/Foo".to_string(),
                            vec!(),
                            0,
                            &vec!());
    let functions = vec!(f_a);
    let mut manifest = create_manifest(functions);
    let provider = MetaProvider {};
    let mut loader = Loader::new();
    let manifest_url = url_from_rel_path("manifest.json");

    // Load library functions provided
    loader.add_lib(&provider, get_manifest(), &cwd_as_url().unwrap().to_string()).unwrap();

    assert!(loader.resolve_implementations(&mut manifest, &provider, &manifest_url).is_err());
}

// TODO add a wasm loading test
// check coverage of flowrlib/loader.rs and wasm.rs

// TODO add a wasm and native execution test
// check the coverage of execution.rs