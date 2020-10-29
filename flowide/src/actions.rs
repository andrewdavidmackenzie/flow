use gtk::{TextBufferExt, WidgetExt};
use url::Url;

use flowclib::compiler::compile;
use flowclib::compiler::loader;
use flowclib::generator::generate;
use flowclib::model::flow::Flow;
use flowclib::model::process::Process::FlowProcess;
use flowrlib::coordinator::Submission;
use flowrstructs::manifest::{DEFAULT_MANIFEST_FILENAME, Manifest};
use provider::content::provider::MetaProvider;

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
                                        context.manifest_url = Some(Url::parse(&manifest_url_str).unwrap());
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
            Err(e) => message(&e)
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
            Ok((manifest, _)) => {
                set_manifest(&manifest);

                match UICONTEXT.try_lock() {
                    Ok(mut context) => {
                        context.manifest = Some(manifest);
                        context.manifest_url = Some(Url::parse(&url).unwrap());
                    }
                    Err(_) => message("Could not lock UI Context")
                }
            }
            Err(e) => message(&e.to_string())
        }
    });
}

fn set_args(arg: Vec<String>) {
    match UICONTEXT.try_lock() {
        Ok(ref mut context) => {
            let mut guard = context.client.lock().unwrap();
            guard.set_args(arg);
        }
        _ => message("Could not get access to uicontext and client")
    }
}

pub fn run_manifest(args: Vec<String>) {
    std::thread::spawn(move || {
        match UICONTEXT.try_lock() {
            Ok(ref mut context) => {
                match &context.manifest_url {
                    Some(manifest_url) => {
                        set_args(args);
                        // let debug_client = CLI_DEBUG_CLIENT;
                        let _submission = Submission::new(&manifest_url.to_string(),
                                                          1,
                                                          false);
                        // let mut coordinator = Coordinator::new(1);
                        // coordinator.init();
                        //
                        // coordinator.submit(submission);
                        message("Submitted flow for execution");
                    }
                    _ => message("No manifest loaded to run")
                }
            }
            _ => message("Could not get access to uicontext and client")
        }
    });
}