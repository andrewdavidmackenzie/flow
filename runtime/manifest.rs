use std::sync::{Arc, Mutex};

use flowrlib::lib_manifest::{ImplementationLocator::Native, LibraryManifest};
use flowrlib::manifest::MetaData;

use crate::runtime_client::RuntimeClient;

/// Return the `LibraryManifest` for the runtime functions to be added to `flowr`
pub fn create_runtime(client: &'static dyn RuntimeClient) -> LibraryManifest {
    let metadata = MetaData {
        name: "flowr-runtime".into(),
        version: "0.1.0".into(),
        description: "Runtime library provided by flowr binary".into(),
        author_name: "Andrew Mackenzie".into(),
        author_email: "andrew@mackenzie-serres.net".into(),

    };
    let mut manifest = LibraryManifest::new(metadata);

    manifest.locators.insert("lib://runtime/args/get/Get".to_string(),
                             Native(Arc::new(super::args::get::Get { client: Arc::new(Mutex::new(client)) })));
    manifest.locators.insert("lib://runtime/file/file_write/FileWrite".to_string(),
                             Native(Arc::new(super::file::file_write::FileWrite { client: Arc::new(Mutex::new(client)) })));
    manifest.locators.insert("lib://runtime/stdio/readline/Readline".to_string(),
                             Native(Arc::new(super::stdio::readline::Readline { client: Arc::new(Mutex::new(client)) })));
    manifest.locators.insert("lib://runtime/stdio/stdin/Stdin".to_string(),
                             Native(Arc::new(super::stdio::stdin::Stdin { client: Arc::new(Mutex::new(client)) })));
    manifest.locators.insert("lib://runtime/stdio/stdout/Stdout".to_string(),
                             Native(Arc::new(super::stdio::stdout::Stdout { client: Arc::new(Mutex::new(client)) })));
    manifest.locators.insert("lib://runtime/stdio/stderr/Stderr".to_string(),
                             Native(Arc::new(super::stdio::stderr::Stderr { client: Arc::new(Mutex::new(client)) })));

    manifest
}