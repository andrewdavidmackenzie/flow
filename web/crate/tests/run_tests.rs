extern crate wasm_bindgen_test;

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);
// run these tests using: wasm-pack test --chrome --headless -- --features wasm-bindgen

#[wasm_bindgen_test]
fn test_load_manifest() {
    assert!(true);
}