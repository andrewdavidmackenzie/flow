use std::sync::Arc;

use flowrlib::lib_manifest::{ImplementationLocator::Native, ImplementationLocator::Wasm, LibraryManifest};

use crate::control;
use crate::data;
use crate::fmt;
use crate::img;
use crate::math;

pub fn get_manifest() -> LibraryManifest {
    let mut manifest = LibraryManifest::new();

    manifest.locators.insert("lib://flowstdlib/math/add/Add".to_string(),
                        Native(Arc::new(math::add::Add {})));
    manifest.locators.insert("lib://flowstdlib/math/divide/Divide".to_string(),
                        Native(Arc::new(math::divide::Divide {})));
    manifest.locators.insert("lib://flowstdlib/math/subtract/Subtract".to_string(),
                        Native(Arc::new(math::subtract::Subtract{})));

    manifest.locators.insert("lib://flowstdlib/control/tap/Tap".to_string(),
                        Native(Arc::new(control::tap::Tap)));
    manifest.locators.insert("lib://flowstdlib/control/compare/Compare".to_string(),
                        Native(Arc::new(control::compare::Compare {})));
    manifest.locators.insert("lib://flowstdlib/fmt/to_string/ToString".to_string(),
                        Native(Arc::new(fmt::to_string::ToString {})));
    manifest.locators.insert("lib://flowstdlib/fmt/to_number/ToNumber".to_string(),
                        Native(Arc::new(fmt::to_number::ToNumber {})));
    manifest.locators.insert("lib://flowstdlib/img/format_png/FormatPNG".to_string(),
                        Native(Arc::new(img::format_png::FormatPNG {})));

    manifest.locators.insert("lib://flowstdlib/data/buffer/Buffer".to_string(),
                        Native(Arc::new(data::buffer::Buffer {})));
    manifest.locators.insert("lib://flowstdlib/data/compose_array/ComposeArray".to_string(),
                        Native(Arc::new(data::compose_array::ComposeArray {})));
    manifest.locators.insert("lib://flowstdlib/data/zip/Zip".to_string(),
                        Native(Arc::new(data::zip::Zip {})));

    // TODO remove this fake added wasm function with a real one, with a Makefile to build it when wasm execution is ready
    manifest.locators.insert("lib://flowstdlib/fmt/reverse/Reverse".to_string(),
                        Wasm(("src/fmt/reverse.wasm".to_string(), "reverse".to_string())));

    manifest
}