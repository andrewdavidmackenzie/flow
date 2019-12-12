use std::sync::{Arc, Mutex};

use log::info;

use flowclib::compiler::compile;
use flowclib::compiler::loader;
use flowclib::generator::generate;
use flowclib::model::flow::Flow;
use flowclib::model::process::Process::FlowProcess;
use flowrlib::lib_manifest::LibraryManifest;
use flowrlib::loader::Loader;
use flowrlib::manifest::Manifest;
use flowrlib::provider::Provider;
use provider::content::provider::MetaProvider;
use runtime::runtime_client::RuntimeClient;

/*
    manifest_dir is used as a reference directory for relative paths to project files
*/
pub fn compile(flow: &Flow, debug_symbols: bool, manifest_dir: &str) -> Result<Manifest, String> {
    info!("Compiling Flow to Manifest");
    let tables = compile::compile(flow)
        .map_err(|e| format!("Could not compile flow: '{}'", e.to_string()))?;

    generate::create_manifest(&flow, debug_symbols, &manifest_dir, &tables)
        .map_err(|e| format!("Could create flow manifest: '{}'", e.to_string()))
}

pub fn load_flow(url: &str) -> Result<Flow, String> {
    let provider = MetaProvider {};

    match loader::load_context(url, &provider)
        .map_err(|e| format!("Could not load flow context: '{}'", e.to_string()))? {
        FlowProcess(flow) => Ok(flow),
        _ => Err("Process loaded was not of type 'Flow'".into())
    }
}

fn load_libs<'a>(loader: &'a mut Loader, provider: &dyn Provider, runtime_manifest: LibraryManifest) -> Result<String, String> {
    // Load this runtime's library of native (statically linked) implementations
    loader.add_lib(provider, runtime_manifest, "runtime").map_err(|e| e.to_string())?;

    // Load the native flowstdlib - before it maybe loaded from WASM
    loader.add_lib(provider, flowstdlib::get_manifest(), "flowstdlib").map_err(|e| e.to_string())?;

    Ok("Added the 'runtime' and 'flowstdlibs'".to_string())
}

pub fn load_from_uri(uri: &str, runtime_client: Arc<Mutex<dyn RuntimeClient>>) -> Result<(Loader, Manifest), String> {
    let mut loader = Loader::new();
    let provider = MetaProvider {};
    let runtime_manifest = runtime::manifest::create_runtime(runtime_client);

    load_libs(&mut loader, &provider, runtime_manifest).map_err(|e| e.to_string())?;
    let manifest = loader.load_manifest(&provider, uri).unwrap(); // TODO

    Ok((loader, manifest))
}