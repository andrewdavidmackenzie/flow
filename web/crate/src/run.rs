use flowrlib::coordinator::{Coordinator, Submission};
use flowrlib::loader::Loader;
use flowrlib::manifest::Manifest;
use log;
use wasm_bindgen::prelude::*;
use wasm_logger;
use webprovider::content::provider::MetaProvider;

use crate::runtime::ilt;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn layout_panels() -> Result<(), JsValue> {
    // Use `web_sys`'s global `window` function to get a handle on the global
    // window object.
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let body = document.body().expect("document should have a body");

    // Get versions of libraries we link with
    let flowstdlib_version = flowstdlib::info::version();
    let flowrlib_version = flowrlib::info::version();
    let flowclib_version = flowclib::info::version();

    let info = document.create_element("p")?;
    body.append_child(&info)?;

    let mut text = document.create_text_node(&format!("flowstdlib: version = {}", flowstdlib_version));
    info.append_child(&text)?;
    text = document.create_text_node(&format!("flowrlib: version = {}", flowrlib_version));
    info.append_child(&text)?;
    text = document.create_text_node(&format!("flowclib: version = {}", flowclib_version));
    info.append_child(&text)?;

    let manifest_el = document.create_element("p")?;
    manifest_el.set_id("manifest");
    body.append_child(&manifest_el)?;

    let args_el = document.create_element("p")?;
    args_el.set_id("args");
    args_el.set_inner_html("arg1 arg2");
    body.append_child(&args_el)?;

    let std_out_el = document.create_element("p")?;
    std_out_el.set_id("stdout");
    body.append_child(&std_out_el)?;

    let std_err_el = document.create_element("p")?;
    std_err_el.set_id("stderr");
    body.append_child(&std_err_el)?;

    Ok(())
}

fn load_manifest(_url: &str) -> Result<Manifest, String> {
    let provider = &MetaProvider{};

    let content = String::from_utf8_lossy(include_bytes!("manifest.json"));
    let mut manifest = Manifest::from_str(&content)?;

    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let manifest_el = document.get_element_by_id("manifest").expect("could not find 'stderr' element");
    manifest_el.set_inner_html(&content);

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

fn init_logging() {
    wasm_logger::init(
        wasm_logger::Config::new(log::Level::Debug)
            .message_on_new_line()
    );

    info!("Logging initialized");
}

// Called by our JS entry point to run the example.
#[wasm_bindgen]
pub fn run() -> Result<(), JsValue> {
    set_panic_hook();
    init_logging();

    info!("Laying out panels");
    layout_panels()?;

    info!("Loading manifest");
    let manifest = load_manifest("fake url")?;

    info!("Creating Submission");
    let submission = Submission::new(manifest, 1, false, None);

    info!("Creating Coordinator");
    let mut coordinator = Coordinator::new(0);

    info!("Submitting flow for execution");
    coordinator.submit(submission);

    Ok(())
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