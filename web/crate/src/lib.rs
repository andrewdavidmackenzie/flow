extern crate flowclib;
extern crate flowrlib;
extern crate flowstdlib;

use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Called by our JS entry point to run the example.
#[wasm_bindgen]
pub fn run() -> Result<(), JsValue> {
    set_panic_hook();

    // Use `web_sys`'s global `window` function to get a handle on the global
    // window object.
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let body = document.body().expect("document should have a body");

    // Get versions of libraries we link with
    let flowstdlib_version = flowstdlib::info::version();
    let flowrlib_version = flowrlib::info::version();
    let flowclib_version = flowclib::info::version();

    let std = document.create_element("p")?;
    std.set_inner_html(&format!("flowstdlib: version = {}", flowstdlib_version));
    body.append_child(&std)?;

    let runtime = document.create_element("p")?;
    runtime.set_inner_html(&format!("flowrlib: version = {}", flowrlib_version));
    body.append_child(&runtime)?;

    let compiler = document.create_element("p")?;
    compiler.set_inner_html(&format!("flowclib: version = {}", flowclib_version));
    body.append_child(&compiler)?;

    Ok(())
}

fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function to get better error messages if we ever panic.
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
