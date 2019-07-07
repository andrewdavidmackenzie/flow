extern crate wasm_bindgen_test;

use futures::prelude::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// run these tests using:
//     wasm-pack test --chrome --headless -- --features wasm-bindgen
// which is used by the 'make test' Makefile target

// This runs a unit test in native Rust, so it can only use Rust APIs.
#[test]
fn rust_test() {
    assert_eq!(1, 1);
}

#[wasm_bindgen_test]
fn test_load_manifest() {
    let manifest_content = String::from_utf8_lossy(include_bytes!("hello_world.json"));
    println!("Manifest: \n{}", manifest_content);
//    let manifest = run::load_manifest(&manifest_content, "file::hello_world.json");
//    assert_eq!("context", manifest.unwrap().metadata.alias);
}

// This runs a unit test in the browser, so it can use browser APIs.
#[wasm_bindgen_test]
fn web_test() {
    assert_eq!(1, 1);
}


// This runs a unit test in the browser, and in addition it supports asynchronous Future APIs.
#[wasm_bindgen_test(async)]
fn async_test() -> impl Future<Item = (), Error = JsValue> {
    // Creates a JavaScript Promise which will asynchronously resolve with the value 42.
    let promise = js_sys::Promise::resolve(&JsValue::from(42));

    // Converts that Promise into a Future.
    // The unit test will wait for the Future to resolve.
    JsFuture::from(promise)
        .map(|x| {
            assert_eq!(x, 42);
        })
}
