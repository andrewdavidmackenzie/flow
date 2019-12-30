use std::sync::{Arc, Mutex};

use gtk::{TextBufferExt, WidgetExt};
use toml;

use flowclib::compiler::compile;
use flowclib::compiler::loader;
use flowclib::generator::generate;
use flowclib::generator::generate::GenerationTables;
use flowclib::model::flow::Flow;
use flowclib::model::process::Process::FlowProcess;
use flowrlib::coordinator::{Coordinator, Submission};
use flowrlib::lib_manifest::LibraryManifest;
use flowrlib::loader::Loader;
use flowrlib::manifest::Manifest;
use flowrlib::provider::Provider;
use provider::content::provider::MetaProvider;
use runtime::runtime_client::RuntimeClient;

use crate::ide_runtime_client::IDERuntimeClient;
use crate::UICONTEXT;
use crate::widgets;

pub fn compile_flow() -> Result<String, String> {
    match UICONTEXT.lock() {
        Ok(ref mut context) => {
            match &context.flow {
                Some(ref flow) => {
                    let flow_clone = flow.clone();
                    std::thread::spawn(move || {
                            let tables = compile::compile(&flow_clone).expect("Could not compile flow");

    //                        info!("==== Compiler phase: Compiling provided implementations");
    //                        compile_supplied_implementations(&mut tables, provided_implementations, release)?;

                            let manifest_dir = std::env::current_dir().unwrap(); // TODO
                            let manifet_dir_string = manifest_dir.as_path().to_string_lossy();
                            let manifest = create_manifest(&flow_clone, true, &manifet_dir_string, &tables)
                                .unwrap(); // TODO
                            set_manifest_content(&manifest);
                        });
                    Ok("Compiling flow".to_string())
                }
                _ => Err("No flow loaded to compile".into())
            }
        }
        _ => Err("Could not access ui context".into())
    }
}

/*
    Generate a manifest for the flow in JSON that can be used to run it using 'flowr'
*/
fn create_manifest(flow: &Flow, debug_symbols: bool, manifest_dir: &str, tables: &GenerationTables) -> Result<String, String> {
    let manifest = generate::create_manifest(&flow, debug_symbols, manifest_dir, tables)
        .map_err(|e| e.to_string())?;

    serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())
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

pub fn run_manifest() -> Result<String, String> {
    match UICONTEXT.lock() {
        Ok(ref mut context) => {
            match &context.manifest {
                Some(manifest) => {
                    let manifest_clone: Manifest = manifest.clone();
                    std::thread::spawn(move || {
                        let submission = Submission::new(manifest_clone, 1, false, None);
                        let mut coordinator = Coordinator::new(1);
                        coordinator.submit(submission);
                    });
                    Ok("Submitting flow for execution".to_string())
                }
                _ => Err("No manifest loaded to run".into())
            }
        }
        _ => Err("Could not access ui context".into())
    }
}

pub fn open_flow(uri: String) {
    std::thread::spawn(move || {
        let flow = load_flow(&uri).unwrap(); // TODO
        let flow_content = toml::Value::try_from(&flow).unwrap().to_string(); // TODO

        match UICONTEXT.lock() {
            Ok(mut context) => context.flow = Some(flow),
            Err(_) => { /* TODO */ }
        }

        widgets::do_in_gtk_eventloop(|refs| {
            refs.compile_flow_menu().set_sensitive(true);
            refs.flow_buffer().set_text(&flow_content);
        });
    });
}

fn set_manifest_content(manifest_content: &str) {
    widgets::do_in_gtk_eventloop(|refs| {
        refs.run_manifest_menu().set_sensitive(true);
        refs.manifest_buffer().set_text(&manifest_content);
    });
}

pub fn open_manifest(uri: String) {
    std::thread::spawn(move || {
        let runtime_client = Arc::new(Mutex::new(IDERuntimeClient));
        let (loader, manifest) = load_from_uri(&uri, runtime_client).unwrap(); // TODO

        let manifest_content = serde_json::to_string_pretty(&manifest).unwrap(); // TODO

        match UICONTEXT.lock() {
            Ok(mut context) => {
                context.loader = Some(loader);
                context.manifest = Some(manifest);
                // TODO enable run action
            }
            Err(_) => {}
        }

        set_manifest_content(&manifest_content);
    });
}