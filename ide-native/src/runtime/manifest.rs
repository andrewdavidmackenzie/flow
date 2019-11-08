use std::sync::Arc;

use flowrlib::lib_manifest::{ImplementationLocator::Native, LibraryManifest};
use flowrlib::manifest::MetaData;

pub fn get_manifest() -> LibraryManifest {
    let metadata = MetaData {
        name: "flowr-runtime".into(),
        version: "0.1.0".into(),
        description: "Runtime library provided by flowr binary".into(),
        author_name: "Andrew Mackenzie".into(),
        author_email: "andrew@mackenzie-serres.net".into(),

    };
    let mut manifest = LibraryManifest::new(metadata);

    manifest.locators.insert("lib://runtime/args/get/Get".to_string(), Native(Arc::new(super::args::get::Get)));
    manifest.locators.insert("lib://runtime/file/file_write/FileWrite".to_string(), Native(Arc::new(super::file::file_write::FileWrite{})));
    manifest.locators.insert("lib://runtime/stdio/readline/Readline".to_string(), Native(Arc::new(super::stdio::readline::Readline{})));
    manifest.locators.insert("lib://runtime/stdio/stdin/Stdin".to_string(), Native(Arc::new(super::stdio::stdin::Stdin{})));
    manifest.locators.insert("lib://runtime/stdio/stdout/Stdout".to_string(), Native(Arc::new(super::stdio::stdout::Stdout{})));
    manifest.locators.insert("lib://runtime/stdio/stderr/Stderr".to_string(), Native(Arc::new(super::stdio::stderr::Stderr{})));

    manifest
}