use std::sync::Arc;

use flowrlib::lib_manifest::{ImplementationLocator::Native, LibraryManifest};

pub fn get_manifest() -> LibraryManifest {
    let mut manifest = LibraryManifest::new();

    manifest.locators.insert("lib://runtime/args/get/Get".to_string(), Native(Arc::new(::args::get::Get{})));
    manifest.locators.insert("lib://runtime/file/file_write/FileWrite".to_string(), Native(Arc::new(::file::file_write::FileWrite{})));
    manifest.locators.insert("lib://runtime/stdio/readline/Readline".to_string(), Native(Arc::new(::stdio::readline::Readline{})));
    manifest.locators.insert("lib://runtime/stdio/stdin/Stdin".to_string(), Native(Arc::new(::stdio::stdin::Stdin{})));
    manifest.locators.insert("lib://runtime/stdio/stdout/Stdout".to_string(), Native(Arc::new(::stdio::stdout::Stdout{})));
    manifest.locators.insert("lib://runtime/stdio/stderr/Stderr".to_string(), Native(Arc::new(::stdio::stderr::Stderr{})));

    manifest
}