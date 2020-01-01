use std::sync::{Arc, Mutex};

use gtk::{TextBufferExt, WidgetExt};
use toml;

use flowclib::compiler::compile;
use flowclib::compiler::loader;
use flowclib::generator::generate;
use flowclib::model::flow::Flow;
use flowclib::model::process::Process::FlowProcess;
use flowrlib::coordinator::{Coordinator, Submission};
use flowrlib::lib_manifest::LibraryManifest;
use flowrlib::loader::Loader;
use flowrlib::manifest::Manifest;
use flowrlib::provider::Provider;
use provider::content::provider::MetaProvider;

use crate::ide_runtime_client::IDERuntimeClient;
use crate::message;
use crate::UICONTEXT;
use crate::widgets;

pub fn compile_flow() {
    std::thread::spawn(move || {
        match UICONTEXT.try_lock() {
            Ok(ref mut context) => {
                match &context.flow {
                    Some(ref flow) => {
                        let flow_clone = flow.clone();
                        let tables = compile::compile(&flow_clone).expect("Could not compile flow");

                        //                        info!("==== Compiler phase: Compiling provided implementations");
                        //                        compile_supplied_implementations(&mut tables, provided_implementations, release)?;

                        let manifest_dir = std::env::current_dir().unwrap(); // TODO
                        let manifest_dir_string = manifest_dir.as_path().to_string_lossy();
                        match generate::create_manifest(&flow, true, &manifest_dir_string, &tables) {
                            Ok(manifest) => {
                                set_manifest(&manifest);
                                context.manifest = Some(manifest);
                            }
                            Err(e) => message(&e.to_string())
                        }
                    }
                    _ => message("No flow loaded to compile")
                }
            }
            _ => message("Could not access ui context")
        }
    });
}

fn load_flow_from_url(url: &str) -> Result<Flow, String> {
    let provider = MetaProvider {};

    match loader::load_context(url, &provider)
        .map_err(|e| format!("Could not load flow context: '{}'", e.to_string()))? {
        FlowProcess(flow) => Ok(flow),
        _ => Err("Process loaded was not of type 'Flow'".into())
    }
}

pub fn open_flow(uri: String) {
    std::thread::spawn(move || {
        match load_flow_from_url(&uri) {
            Ok(flow) => {
                match toml::Value::try_from(&flow) {
                    Ok(flow_content) => {
                        match UICONTEXT.try_lock() {
                            Ok(mut context) => {
                                context.flow = Some(flow);

                                widgets::do_in_gtk_eventloop(|refs| {
                                    refs.compile_flow_menu().set_sensitive(true);
                                    refs.flow_buffer().set_text(&flow_content.to_string());
                                });
                            }
                            _ => message("Could not get access to uicontext")
                        }
                    }
                    Err(e) => message(&e.to_string())
                }
            }
            Err(e) => message(&e.to_string())
        }
    });
}

fn set_manifest(manifest: &Manifest) {
    let manifest_content = serde_json::to_string_pretty(manifest).unwrap(); // TODO
    widgets::do_in_gtk_eventloop(|refs| {
        refs.run_manifest_menu().set_sensitive(true);
        refs.manifest_buffer().set_text(&manifest_content);
    });
}

pub fn open_manifest(uri: String) {
    std::thread::spawn(move || {
        let provider = MetaProvider {};
        match Manifest::load(&provider, &uri) {
            Ok(manifest) => {
                set_manifest(&manifest);

                match UICONTEXT.try_lock() {
                    Ok(mut context) => {
                        context.manifest = Some(manifest);
                    }
                    Err(_) => message("Could not lock UI Context")
                }
            }
            Err(e) => message(&e.to_string())
        }
    });
}

fn load_libs<'a>(loader: &'a mut Loader, provider: &dyn Provider, runtime_manifest: LibraryManifest) -> Result<String, String> {
    // Load this runtime's library of native (statically linked) implementations
    loader.add_lib(provider, runtime_manifest, "runtime").map_err(|e| e.to_string())?;

    // Load the native flowstdlib - before it maybe loaded from WASM
    loader.add_lib(provider, flowstdlib::get_manifest(), "flowstdlib").map_err(|e| e.to_string())?;

    Ok("Added the 'runtime' and 'flowstdlibs'".to_string())
}

fn load_manifest(manifest: &mut Manifest) {
    let mut loader = Loader::new();
    let provider = MetaProvider {};

    let runtime_client = Arc::new(Mutex::new(IDERuntimeClient));
    let runtime_manifest = runtime::manifest::create_runtime(runtime_client);

    // Load the 'runtime' library provided by the IDE and the 'flowstdlib' libraries
    match load_libs(&mut loader, &provider, runtime_manifest) {
        Ok(s) => message(&s),
        Err(e) => message(&e)
    }

    // load any other libraries the flow references - these will be loaded as WASM
    loader.load_libraries(&provider, &manifest).unwrap(); // TODO

    // 'root_dir' is where any relative file paths will be anchored
    let root_dir = std::env::current_dir().unwrap(); // TODO
    let root_dir_string = root_dir.as_path().to_string_lossy();

    // Find the implementations for all functions in this flow
    loader.resolve_implementations(manifest, &provider, &root_dir_string).unwrap();
    // TODO
}

pub fn run_manifest() {
    std::thread::spawn(move || {
        match UICONTEXT.try_lock() {
            Ok(ref mut context) => {
                match &context.manifest {
                    Some(manifest) => {
                        let mut manifest_clone: Manifest = manifest.clone();
                        load_manifest(&mut manifest_clone); // TODO
                        let submission = Submission::new(manifest_clone, 1, false, None);
                        let mut coordinator = Coordinator::new(1);
                        coordinator.submit(submission);
                        message("Submitted flow for execution");
                    }
                    _ => message("No manifest loaded to run")
                }
            }
            _ => message("Could not get access to uicontext")
        }
    });
}