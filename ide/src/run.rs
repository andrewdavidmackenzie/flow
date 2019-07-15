use flowclib::compiler::compile;
use flowclib::compiler::loader;
use flowclib::generator::generate;
use flowclib::model::flow::Flow;
use flowclib::model::process::Process::FlowProcess;
use flowrlib::coordinator::{Coordinator, Submission};
use flowrlib::loader::Loader;
use flowrlib::manifest::Manifest;
use flowrlib::provider::Provider;
use log;
use log::Level;
use nodeprovider::content::provider::MetaProvider;
use serde_json;
use std::fmt::Debug;
use std::str::FromStr;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_logger;
use web_sys::Document;
use web_sys::HtmlButtonElement;

use crate::runtime::ilt;

const DEFAULT_LOG_LEVEL: Level = Level::Error;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn info(document: &Document) -> Result<(), JsValue> {
    let flowide_el = document.get_element_by_id("flowide").expect("could not find 'flowide' element");
    flowide_el.set_inner_html(&format!("flowide: version = {}", env!("CARGO_PKG_VERSION")));

    let flowstdlib_el = document.get_element_by_id("flowstdlib").expect("could not find 'flowstdlib' element");
    flowstdlib_el.set_inner_html(&format!("flowstdlib: version = {}", flowstdlib::info::version()));

    let flowrlib_el = document.get_element_by_id("flowrlib").expect("could not find 'flowrlib' element");
    flowrlib_el.set_inner_html(&format!("flowrlib: version = {}", flowrlib::info::version()));

    let flowclib_el = document.get_element_by_id("flowclib").expect("could not find 'flowclib' element");
    flowclib_el.set_inner_html(&format!("flowclib: version = {}", flowclib::info::version()));

    Ok(())
}

fn get_flow_lib_path(document: &Document) -> Result<String, String> {
    let flow_lib_path_el = document.get_element_by_id("flow_lib_path").expect("could not find 'flow_lib_path' element");
    flow_lib_path_el.text_content().ok_or("Flow Lib Path not set".into())
}

fn pretty_print_json_for_html<S: Into<String> + Debug>(string: &S) -> String {
    format!("{:?}", string)
        .replace("\\\"", "\"")
        .replace("\\n", "<br/>")
        .replace("\"{", "{")
        .replace("}\"", "}")
}

fn load_flow(provider: &Provider, url: &str) -> Result<Flow, String> {
    info!("Loading flow");

    match loader::load_context(url, provider)? {
        FlowProcess(flow) => Ok(flow),
        _ => Err("Process loaded was not of type 'Flow'".into())
    }
}

fn set_flow_contents(document: &Document, content: &str) {
    let flow_el = document.get_element_by_id("flow").expect("could not find 'flow' element");
    flow_el.set_inner_html(&content);
}

/*
    manifest_dir is used as a reference directory for relative paths to project files
*/
fn compile(flow: &Flow, debug_symbols: bool, manifest_dir: &str) -> Result<Manifest, String> {
    info!("Compiling Flow to Manifest");
    let tables = compile::compile(flow)?;

    generate::create_manifest(&flow, debug_symbols, &manifest_dir, &tables)
        .chain_err(|| "Could not create the manifest from flow and compiled tables")
}

pub fn load_manifest(provider: &Provider, url: &str) -> Result<Manifest, String> {
    info!("Loading manifest");

    let mut loader = Loader::new();

    let mut manifest = loader.load_manifest(provider, url)?;

    // Load this runtime's native implementations
    loader.add_lib(provider, ilt::get_ilt(), url)?;

    info!("adding flowstdlib");
    // TODO - when loader can load a library from a reference in the manifest via it's WASM
    loader.add_lib(provider, flowstdlib::ilt::get_ilt(),
                   &format!("{}flowstdlib/ilt.json", url))?; // TODO fix this URL

    // This doesn't do anything currently - leaving here for the future
    // as when this loads libraries from manifest, previous manual adding of
    // libs will not be needed
    Loader::load_libraries(provider, &manifest)?;

    info!("resolving implementations");
    // Find the implementations for all functions in this flow
    loader.resolve_implementations(&mut manifest, provider, "fake manifest_url").
        chain_err(|| "Could not resolve the implementations of loaded functions")?;

    Ok(manifest)
}

fn set_manifest_contents(document: &Document, content: &str) {
    let manifest_el = document.get_element_by_id("manifest").expect("could not find 'manifest' element");
    manifest_el.set_inner_html(&pretty_print_json_for_html(&content));
}

fn setup_run_button(document: &Document) {
    let run = Closure::wrap(Box::new(move || {
        info!("run clicked");
    }) as Box<dyn FnMut()>);
    document
        .get_element_by_id("run_button")
        .expect("could not find 'run_button' element")
        .dyn_ref::<HtmlButtonElement>()
        .expect("#run_button should be an `HtmlButtonElement`")
        .set_onclick(Some(run.as_ref().unchecked_ref()));
    run.forget();
}

fn setup_actions(document: &Document) -> Result<(), JsValue> {
    setup_run_button(document);

    Ok(())
}

fn run_submission(submission: Submission) {
    let mut coordinator = Coordinator::new(0);

    info!("Submitting flow for execution");
    coordinator.submit(submission);
}

fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function to get better error messages if we ever panic.
    #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();
}

fn init_logging(arg: Option<String>) {
    let level = match arg {
        Some(string) => {
            match Level::from_str(&string) {
                Ok(ll) => ll,
                Err(_) => DEFAULT_LOG_LEVEL
            }
        }
        None => DEFAULT_LOG_LEVEL
    };

    wasm_logger::init(
        wasm_logger::Config::new(level)
            .message_on_new_line()
    );

    info!("Logging initialized to level: '{}'", level);
}

fn get_log_level(document: &Document) -> Option<String> {
    let log_level_el = document.get_element_by_id("log_level").expect("could not find 'log_level' element");
    log_level_el.text_content()
}

// Called by our JS entry point
#[wasm_bindgen]
pub fn main_js() -> Result<(), JsValue> {
    set_panic_hook();
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");

    let log_level_arg = get_log_level(&document);

    init_logging(log_level_arg);

    info(&document)?;

    let flow_lib_path = get_flow_lib_path(&document)?;

    setup_actions(&document)?;

    let manifest;

    if false {
        let flow_content: String = String::from_utf8_lossy(include_bytes!("hello_world.toml")).into();
        set_flow_contents(&document, &flow_content);

        let provider = MetaProvider::new(flow_content, flow_lib_path);

        let flow = load_flow(&provider, "file:://Users/andrew/workspace/flow/ide/crate/src/hello_world.toml")?;
        manifest = compile(&flow, true, "/Users/andrew/workflow/flow")?;

        let manifest_content = serde_json::to_string_pretty(&manifest)
            .chain_err(|e| "Could not pretty format Json content")?;
        set_manifest_contents(&document, &manifest_content);
    } else {
        let manifest_content = String::from_utf8_lossy(include_bytes!("hello_world.json")).to_string();
        set_manifest_contents(&document, &manifest_content);

        let provider = MetaProvider::new(manifest_content, flow_lib_path);

        manifest = load_manifest(&provider, "file://")?;
    }

    let submission = Submission::new(manifest, 1, false, None);

    run_submission(submission);

    Ok(())
}