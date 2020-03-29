use std::sync::{Arc, Mutex};

use gtk::{TextBufferExt, WidgetExt};
use toml;
use url::Url;

use flowclib::compiler::compile;
use flowclib::compiler::loader;
use flowclib::generator::generate;
use flowclib::model::flow::Flow;
use flowclib::model::process::Process::FlowProcess;
use flowrlib::coordinator::{Coordinator, Submission};
use flowrlib::lib_manifest::LibraryManifest;
use flowrlib::loader::Loader;
use flowrlib::manifest::{DEFAULT_MANIFEST_FILENAME, Manifest};
use flowrlib::provider::Provider;
use provider::content::provider::MetaProvider;

use crate::ide_runtime_client::IDERuntimeClient;
use crate::message;
use crate::UICONTEXT;
use crate::widgets;

fn manifest_url(flow_url_str: &str) -> String {
    let flow_url = Url::parse(&flow_url_str).unwrap();
    flow_url.join(DEFAULT_MANIFEST_FILENAME).unwrap().to_string()
}

pub fn compile_flow() {
    std::thread::spawn(move || {
        match UICONTEXT.try_lock() {
            Ok(ref mut context) => {
                match (&context.flow, &context.flow_url) {
                    (Some(ref flow), Some(ref flow_url_str)) => {
                        let flow_clone = flow.clone();
                        let flow_url_clone = flow_url_str.clone();
                        message("Compiling flow");
                        match compile::compile(&flow_clone) {
                            Ok(tables) => {
                                //                        info!("==== Compiler phase: Compiling provided implementations");
                                //                        compile_supplied_implementations(&mut tables, provided_implementations, release)?;
                                match generate::create_manifest(&flow, true, &flow_url_clone, &tables) {
                                    Ok(manifest) => {
                                        set_manifest(&manifest);
                                        context.manifest = Some(manifest);
                                        let manifest_url_str = manifest_url(&flow_url_clone);
                                        message(&format!("Manifest url set to '{}'", manifest_url_str));
                                        context.manifest_url = Some(manifest_url_str);
                                    }
                                    Err(e) => message(&e.to_string())
                                }
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

    match loader::load(url, &provider)
        .map_err(|e| format!("Could not load flow context: '{}'", e.to_string()))? {
        FlowProcess(flow) => Ok(flow),
        _ => Err("Process loaded was not of type 'Flow'".into())
    }
}

pub fn open_flow(url: String) {
    std::thread::spawn(move || {
        match load_flow_from_url(&url) {
            Ok(flow) => {
                match toml::Value::try_from(&flow) {
                    Ok(flow_content) => {
                        match UICONTEXT.try_lock() {
                            Ok(mut context) => {
                                context.flow = Some(flow);
                                context.flow_url = Some(url);
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

pub fn open_manifest(url: String) {
    std::thread::spawn(move || {
        let provider = MetaProvider {};
        match Manifest::load(&provider, &url) {
            Ok(manifest) => {
                set_manifest(&manifest);

                match UICONTEXT.try_lock() {
                    Ok(mut context) => {
                        context.manifest = Some(manifest);
                        context.manifest_url = Some(url);
                    }
                    Err(_) => message("Could not lock UI Context")
                }
            }
            Err(e) => message(&e.to_string())
        }
    });
}

fn load_libs(loader: &mut Loader, provider: &dyn Provider, flowruntime_manifest: LibraryManifest) -> Result<String, String> {
    // Load this run-time's library of function implementations
    loader.add_lib(provider, "lib://flowruntime", flowruntime_manifest, "flowruntime").map_err(|e| e.to_string())?;

    // Load the statically linked flowstdlib - before it maybe loaded from WASM
    loader.add_lib(provider, "lib://flowstdlib", flowstdlib::get_manifest(), "flowstdlib").map_err(|e| e.to_string())?;

    Ok("Added the 'flowruntime' and 'flowstdlibs' static libraries".to_string())
}

fn load_manifest(manifest: &mut Manifest, manifest_url: &str, arg: Vec<String>) {
    let mut loader = Loader::new();
    let provider = MetaProvider {};

    let mut ide_runtime_client = IDERuntimeClient::new();
    ide_runtime_client.set_args(arg);
    let runtime_client = Arc::new(Mutex::new(ide_runtime_client));
    let runtime_manifest = flowruntime::get_manifest(runtime_client);

    // Load the 'run-time' library provided by the IDE and the 'flowstdlib' libraries
    match load_libs(&mut loader, &provider, runtime_manifest) {
        Ok(s) => message(&s),
        Err(e) => message(&e)
    }

    // load any other libraries the flow references - these will be loaded as WASM
    loader.load_libraries(&provider, &manifest).unwrap(); // TODO

    // Find the implementations for all functions in this flow
    loader.resolve_implementations(manifest, &provider, manifest_url).unwrap();
    // TODO
}

pub fn run_manifest(args: Vec<String>) {
    std::thread::spawn(move || {
        match UICONTEXT.try_lock() {
            Ok(ref mut context) => {
                match (&context.manifest, &context.manifest_url) {
                    (Some(manifest), Some(manifest_url)) => {
                        let mut manifest_clone: Manifest = manifest.clone();
                        load_manifest(&mut manifest_clone, manifest_url, args);
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