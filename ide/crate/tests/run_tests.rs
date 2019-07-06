extern crate wasm_bindgen_test;

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);
// run these tests using:
//     wasm-pack test --chrome --headless -- --features wasm-bindgen
// which is used by the 'make test' Makefile target

#[wasm_bindgen_test]
fn test_load_manifest() {
    let manifest_content = String::from_utf8_lossy(include_bytes!("hello_world.json"));
    println!("Manifest: \n{}", manifest_content);
//    let manifest = run::load_manifest(&manifest_content, "file::hello_world.json");
//    assert_eq!("context", manifest.unwrap().metadata.alias);
}