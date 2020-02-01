#![deny(missing_docs)]
//! `flowstdlib` is a standard library of functions for `flow` programs to use.
//! It can be compiled and linked natively to a run-time, or each function can be
//! compiled to WebAssembly and loaded from file by the run-time.

#[cfg(feature = "native")]
use std::sync::Arc;

#[cfg(feature = "native")]
use flowrlib::lib_manifest::{ImplementationLocator::Native, LibraryManifest};
#[cfg(feature = "native")]
use flowrlib::manifest::MetaData;

#[cfg(feature = "native")]
/// Control functions
pub mod control;

#[cfg(feature = "native")]
/// Data functions
pub mod data;

#[cfg(feature = "native")]
/// Formatting functions
pub mod fmt;

#[cfg(feature = "native")]
/// Imaging functions
pub mod img;

#[cfg(feature = "native")]
/// Maths functions
pub mod math;

#[cfg(feature = "native")]
/// Return the `LibraryManifest` for the functions in the library
pub fn get_manifest() -> LibraryManifest {
    let metadata = MetaData {
        name: "flowstdlib".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        description: "Flow Standard Library".into(),
        author_name: "Andrew Mackenzie".into(),
        author_email: "andrew@mackenzie-serres.net".into(),

    };
    let mut manifest = LibraryManifest::new(metadata);

    manifest.locators.insert("lib://flowstdlib/data/zip/Zip".to_string(),
                             Native(Arc::new(data::zip::zip::Zip)));
    manifest.locators.insert("lib://flowstdlib/fmt/to_string/ToString".to_string(),
                             Native(Arc::new(fmt::to_string::to_string::ToString)));
    manifest.locators.insert("lib://flowstdlib/img/format_png/FormatPNG".to_string(),
                             Native(Arc::new(img::format_png::format_png::FormatPNG)));
    manifest.locators.insert("lib://flowstdlib/math/divide/Divide".to_string(),
                             Native(Arc::new(math::divide::divide::Divide)));
    manifest.locators.insert("lib://flowstdlib/control/join/Join".to_string(),
                             Native(Arc::new(control::join::join::Join)));
    manifest.locators.insert("lib://flowstdlib/control/tap/Tap".to_string(),
                             Native(Arc::new(control::tap::tap::Tap)));
    manifest.locators.insert("lib://flowstdlib/fmt/reverse/Reverse".to_string(),
                             Native(Arc::new(fmt::reverse::reverse::Reverse)));
    manifest.locators.insert("lib://flowstdlib/fmt/to_number/ToNumber".to_string(),
                             Native(Arc::new(fmt::to_number::to_number::ToNumber)));
    manifest.locators.insert("lib://flowstdlib/math/add/Add".to_string(),
                             Native(Arc::new(math::add::add::Add)));
    manifest.locators.insert("lib://flowstdlib/control/compare/Compare".to_string(),
                             Native(Arc::new(control::compare::compare::Compare)));
    manifest.locators.insert("lib://flowstdlib/math/subtract/Subtract".to_string(),
                             Native(Arc::new(math::subtract::subtract::Subtract)));
    manifest.locators.insert("lib://flowstdlib/data/compose_array/ComposeArray".to_string(),
                             Native(Arc::new(data::compose_array::compose_array::ComposeArray)));
    manifest.locators.insert("lib://flowstdlib/data/buffer/Buffer".to_string(),
                             Native(Arc::new(data::buffer::buffer::Buffer)));

    manifest
}

