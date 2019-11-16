use flowrlib::lib_manifest::{ImplementationLocator::Native, LibraryManifest};
use flowrlib::manifest::MetaData;
use gtk::TextBufferExt;
use std::sync::Arc;

use crate::RuntimeContext;

pub fn create_runtime(runtime_context: &RuntimeContext) -> LibraryManifest {
    let metadata = MetaData {
        name: "flowide-runtime".into(),
        version: "0.1.0".into(),
        description: "Runtime provided by flowide".into(),
        author_name: "Andrew Mackenzie".into(),
        author_email: "andrew@mackenzie-serres.net".into(),

    };
    let mut manifest = LibraryManifest::new(metadata);

    let (start, end) = runtime_context.args.get_bounds();
    let args_string = runtime_context.args.get_text(&start, &end, false).unwrap().to_string();
    let arg_values: Vec<String> = args_string.split(' ').map(|s| s.to_string()).collect();
    manifest.locators.insert("lib://runtime/args/get/Get".to_string(), Native(Arc::new(super::args::get::Get::new(arg_values))));
    manifest.locators.insert("lib://runtime/file/file_write/FileWrite".to_string(), Native(Arc::new(super::file::file_write::FileWrite{})));
    manifest.locators.insert("lib://runtime/stdio/readline/Readline".to_string(), Native(Arc::new(super::stdio::readline::Readline{})));
    manifest.locators.insert("lib://runtime/stdio/stdin/Stdin".to_string(), Native(Arc::new(super::stdio::stdin::Stdin{})));
    manifest.locators.insert("lib://runtime/stdio/stdout/Stdout".to_string(), Native(Arc::new(super::stdio::stdout::Stdout{})));
    manifest.locators.insert("lib://runtime/stdio/stderr/Stderr".to_string(), Native(Arc::new(super::stdio::stderr::Stderr{})));

    manifest
}