use flowrlib::lib_manifest::{ImplementationLocator::Native, LibraryManifest};
use flowrlib::manifest::MetaData;
use gtk::TextBufferExt;
use std::sync::Arc;

use crate::runtime_context::RuntimeContext;

pub fn create_runtime() -> (LibraryManifest, RuntimeContext<'static>) {
    let metadata = MetaData {
        name: "flowide-runtime".into(),
        version: "0.1.0".into(),
        description: "Runtime provided by flowide".into(),
        author_name: "Andrew Mackenzie".into(),
        author_email: "andrew@mackenzie-serres.net".into(),

    };
    let mut manifest = LibraryManifest::new(metadata);

    let get = super::args::get::Get::new();
    let stdout = super::stdio::stdout::Stdout::new();
    let stderr = super::stdio::stderr::Stderr::new();

    manifest.locators.insert("lib://runtime/args/get/Get".to_string(), Native(Arc::new(get)));
    manifest.locators.insert("lib://runtime/file/file_write/FileWrite".to_string(), Native(Arc::new(super::file::file_write::FileWrite {})));
    manifest.locators.insert("lib://runtime/stdio/readline/Readline".to_string(), Native(Arc::new(super::stdio::readline::Readline {})));
    manifest.locators.insert("lib://runtime/stdio/stdin/Stdin".to_string(), Native(Arc::new(super::stdio::stdin::Stdin {})));
    manifest.locators.insert("lib://runtime/stdio/stdout/Stdout".to_string(), Native(Arc::new(stdout)));
    manifest.locators.insert("lib://runtime/stdio/stderr/Stderr".to_string(), Native(Arc::new(stderr)));

    let runtime_context = RuntimeContext::new(get.get_text_buffer(),
                                              stdout.get_text_buffer(),
                                              stderr.get_text_buffer());

    (manifest, runtime_context)
}