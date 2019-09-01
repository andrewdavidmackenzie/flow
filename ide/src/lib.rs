#[macro_use]
extern crate error_chain;
extern crate flow_impl;
extern crate flowclib;
extern crate flowrlib;
#[macro_use]
extern crate log;
extern crate nodeprovider;
#[macro_use]
extern crate serde_json;
extern crate web_sys;

use flowclib::compiler::compile;
use flowclib::compiler::loader;
use flowclib::generator::generate;
use flowclib::model::flow::Flow;
use flowclib::model::process::Process::FlowProcess;
use flowrlib::coordinator::{Coordinator, Submission};
use flowrlib::loader::Loader;
use flowrlib::manifest::Manifest;
use flowrlib::provider::Provider;
use log::Level;
use nodeprovider::content::provider::MetaProvider;
use std::fmt::Debug;
use std::str::FromStr;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_logger;
use web_sys::Document;
use web_sys::HtmlButtonElement;

use crate::runtime::manifest;

// We'll put our errors in an `errors` module, and other modules in
// this crate will `use errors::*;` to get access to everything
// `error_chain!` creates.
pub mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain! {}
}

error_chain! {
    foreign_links {
        Compiler(flowclib::errors::Error);
        Io(::std::io::Error);
    }
}

mod runtime;

const DEFAULT_LOG_LEVEL: Level = Level::Error;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn info(document: &Document) {
    let flowide_el = document.get_element_by_id("flowide").expect("could not find 'flowide' element");
    flowide_el.set_inner_html(&format!("flowide: version = {}", env!("CARGO_PKG_VERSION")));

    let flowrlib_el = document.get_element_by_id("flowrlib").expect("could not find 'flowrlib' element");
    flowrlib_el.set_inner_html(&format!("flowrlib: version = {}", flowrlib::info::version()));

    let flowclib_el = document.get_element_by_id("flowclib").expect("could not find 'flowclib' element");
    flowclib_el.set_inner_html(&format!("flowclib: version = {}", flowclib::info::version()));
}

fn get_flow_lib_path(document: &Document) -> Result<String> {
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

fn load_flow(provider: &dyn Provider, url: &str) -> Result<Flow> {
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
fn compile(flow: &Flow, debug_symbols: bool, manifest_dir: &str) -> Result<Manifest> {
    info!("Compiling Flow to Manifest");
    let tables = compile::compile(flow)?;

    generate::create_manifest(&flow, debug_symbols, &manifest_dir, &tables)
        .chain_err(|| "COuld not compile flow to manifest")
}

pub fn load_manifest(provider: &dyn Provider, url: &str) -> Result<Manifest> {
    info!("Loading manifest");

    let mut loader = Loader::new();

    let mut manifest = loader.load_manifest(provider, url)
        .chain_err(|| "Could not load the manifest")?;

    // Load this runtime's native implementations
    loader.add_lib(provider, manifest::get_manifest(), url)
        .chain_err(|| "Could not add library to loader")?;

    // This doesn't do anything currently - leaving here for the future
    // as when this loads libraries from manifest, previous manual adding of
    // libs will not be needed
    loader.load_libraries(provider, &manifest).chain_err(|| "Could not load libraries")?;

    info!("resolving implementations");
    // Find the implementations for all functions in this flow
    loader.resolve_implementations(&mut manifest, provider, "fake manifest_url")
        .chain_err(|| "Could not resolve implementations of loaded functions")?;

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

fn setup_actions(document: &Document) -> Result<()> {
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

// Declaring a JS function using wasm_bindgen and doing the rest yourself
// to be able to call it from rust
#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

// exporting a function so that it can be called from JS
#[wasm_bindgen]
pub fn export_from_rust(a: u32, b: u32) -> u32 {
    a + b
}

// This is like the `main` function, except for JavaScript.
#[wasm_bindgen(start)]
pub fn main_js() -> std::result::Result<(), JsValue> {
    alert("Flow IDE has started!");

    set_panic_hook();
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");

    let log_level_arg = get_log_level(&document);

    init_logging(log_level_arg);

    info(&document);

    let flow_lib_path = get_flow_lib_path(&document).map_err(|e| JsValue::from_str(&e.to_string()))?;

    setup_actions(&document).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let manifest;

    if false {
        let flow_content: String = String::from_utf8_lossy(include_bytes!("hello_world.toml")).into();
        set_flow_contents(&document, &flow_content);

        let provider = MetaProvider::new(flow_content, flow_lib_path);

        let flow = load_flow(&provider, "file:://Users/andrew/workspace/flow/ide/crate/src/hello_world.toml")
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        manifest = compile(&flow, true, "/Users/andrew/workflow/flow")
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let manifest_content = serde_json::to_string_pretty(&manifest)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        set_manifest_contents(&document, &manifest_content);
    } else {
        let manifest_content = String::from_utf8_lossy(include_bytes!("hello_world.json")).to_string();
        set_manifest_contents(&document, &manifest_content);

        let provider = MetaProvider::new(manifest_content, flow_lib_path);

        manifest = load_manifest(&provider, "file://")
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
    }

    let submission = Submission::new(manifest, 1, false, None);

    run_submission(submission);

    Ok(())
}

