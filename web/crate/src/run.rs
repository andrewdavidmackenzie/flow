use flowrlib::coordinator::{Coordinator, Submission};
use flowrlib::loader::Loader;
use flowrlib::manifest::Manifest;
use log;
use std::fmt::Debug;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_logger;
use web_sys::Document;
use web_sys::HtmlButtonElement;
use webprovider::content::provider::MetaProvider;

use crate::runtime::ilt;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn info(document: &Document) -> Result<(), JsValue> {
    let flowstdlib_el = document.get_element_by_id("flowstdlib").expect("could not find 'flowstdlib' element");
    flowstdlib_el.set_inner_html(&format!("flowstdlib: version = {}", flowstdlib::info::version()));

    let flowrlib_el = document.get_element_by_id("flowrlib").expect("could not find 'flowrlib' element");
    flowrlib_el.set_inner_html(&format!("flowrlib: version = {}", flowrlib::info::version()));

    let flowclib_el = document.get_element_by_id("flowclib").expect("could not find 'flowclib' element");
    flowclib_el.set_inner_html(&format!("flowclib: version = {}", flowclib::info::version()));

    Ok(())
}

fn pretty_print_for_html<S: Into<String> + Debug>(string: S) -> String {
    format!("{:?}", string)
        .replace("\\\"", "\"")
        .replace("\\n", "<br/>")
        .replace("\"{", "{")
        .replace("}\"", "}")
}

fn load_manifest(document: &Document, _url: &str) -> Result<Manifest, String> {
    info!("Loading manifest");
    let provider = &MetaProvider {};

    let content = String::from_utf8_lossy(include_bytes!("hello_world.json"));
    let mut manifest = Manifest::from_str(&content)?;

    let manifest_el = document.get_element_by_id("manifest").expect("could not find 'stderr' element");
    manifest_el.set_inner_html(&pretty_print_for_html(content));

    let mut loader = Loader::new();

    // Load this runtime's native implementations
    loader.add_lib(provider, ilt::get_ilt(), "")?;

    info!("adding flowstdlib");
    // TODO - when loader can load a library from a reference in the manifest via it's WASM
    loader.add_lib(provider, flowstdlib::ilt::get_ilt(),
                   &format!("{}flowstdlib/ilt.json", "file://"))?;

    // This doesn't do anything currently - leaving here for the future
    // as when this loads libraries from manifest, previous manual adding of
    // libs will not be needed
    Loader::load_libraries(provider, &manifest)?;

    info!("resolving implementations");
    // Find the implementations for all functions in this flow
    loader.resolve_implementations(&mut manifest, provider, "fake manifest_url").
        map_err(|e| e.to_string())?;

    Ok(manifest)
}

fn init_logging(_document: &Document) {
    wasm_logger::init(
        wasm_logger::Config::new(log::Level::Debug)
            .message_on_new_line()
    );

    info!("Logging initialized");
}


fn setup_load_button(document: &Document) {
    let load = Closure::wrap(Box::new(move || {
        info!("load clicked");
    }) as Box<dyn FnMut()>);
    document
        .get_element_by_id("load_button")
        .expect("could not find 'load_button' element")
        .dyn_ref::<HtmlButtonElement>()
        .expect("#load_button should be an `HtmlButtonElement`")
        .set_onclick(Some(load.as_ref().unchecked_ref()));
    load.forget();
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
    setup_load_button(document);
    setup_run_button(document);

    Ok(())
}

// Called by our JS entry point to run the example.
#[wasm_bindgen]
pub fn run() -> Result<(), JsValue> {
    set_panic_hook();
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");

    init_logging(&document);

    info(&document)?;

    setup_actions(&document)?;

    let submission = load(&document, "fake url")?;

    run_submission(submission);

    Ok(())
}

fn load(document: &Document, url: &str) -> Result<Submission, String> {
    let manifest = load_manifest(&document, url)?;
    Ok(Submission::new(manifest, 1, false, None))
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

#[cfg(test)]
mod test {
    #[test]
    fn test_load_manifest() {
        assert!(true);
    }
}