#![deny(missing_docs)]
//! `flowstdlib` is a standard library of functions for `flow` programs to use.
//! It can be compiled and linked natively to a run-time, or each function can be
//! compiled to WebAssembly and loaded from file by the run-time.

use std::sync::Arc;

use flowrlib::lib_manifest::{ImplementationLocator::Native, LibraryManifest};
use flowrlib::manifest::MetaData;

/// Control functions
pub mod control;

/// Data functions
pub mod data;

/// Formatting functions
pub mod fmt;

/// Imaging functions
pub mod img;

/// Maths functions
pub mod math;

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


    // Control
    manifest.locators.insert("lib://flowstdlib/control/compare/Compare".to_string(),
                             Native(Arc::new(control::compare::compare::Compare)));
    manifest.locators.insert("lib://flowstdlib/control/compare_switch/CompareSwitch".to_string(),
                             Native(Arc::new(control::compare_switch::compare_switch::CompareSwitch)));
    manifest.locators.insert("lib://flowstdlib/control/join/Join".to_string(),
                             Native(Arc::new(control::join::join::Join)));
    manifest.locators.insert("lib://flowstdlib/control/route/Route".to_string(),
                             Native(Arc::new(control::route::route::Route)));
    manifest.locators.insert("lib://flowstdlib/control/select/Select".to_string(),
                             Native(Arc::new(control::select::select::Select)));
    manifest.locators.insert("lib://flowstdlib/control/tap/Tap".to_string(),
                             Native(Arc::new(control::tap::tap::Tap)));

    // Data
    manifest.locators.insert("lib://flowstdlib/data/accumulate/Accumulate".to_string(),
                             Native(Arc::new(data::accumulate::accumulate::Accumulate)));
    manifest.locators.insert("lib://flowstdlib/data/buffer/Buffer".to_string(),
                             Native(Arc::new(data::buffer::buffer::Buffer)));
    manifest.locators.insert("lib://flowstdlib/data/compose_array/ComposeArray".to_string(),
                             Native(Arc::new(data::compose_array::compose_array::ComposeArray)));
    manifest.locators.insert("lib://flowstdlib/data/info/Info".to_string(),
                             Native(Arc::new(data::info::info::Info)));
    manifest.locators.insert("lib://flowstdlib/data/remove/Remove".to_string(),
                             Native(Arc::new(data::remove::remove::Remove)));
    manifest.locators.insert("lib://flowstdlib/data/transpose/Transpose".to_string(),
                             Native(Arc::new(data::transpose::transpose::Transpose)));
    manifest.locators.insert("lib://flowstdlib/data/zip/Zip".to_string(),
                             Native(Arc::new(data::zip::zip::Zip)));

    // Format
    manifest.locators.insert("lib://flowstdlib/fmt/reverse/Reverse".to_string(),
                             Native(Arc::new(fmt::reverse::reverse::Reverse)));
    manifest.locators.insert("lib://flowstdlib/fmt/to_number/ToNumber".to_string(),
                             Native(Arc::new(fmt::to_number::to_number::ToNumber)));
    manifest.locators.insert("lib://flowstdlib/fmt/to_string/ToString".to_string(),
                             Native(Arc::new(fmt::to_string::to_string::ToString)));

    // Img
    manifest.locators.insert("lib://flowstdlib/img/format_png/FormatPNG".to_string(),
                             Native(Arc::new(img::format_png::format_png::FormatPNG)));

    // Math
    manifest.locators.insert("lib://flowstdlib/math/add/Add".to_string(),
                             Native(Arc::new(math::add::add::Add)));
    manifest.locators.insert("lib://flowstdlib/math/divide/Divide".to_string(),
                             Native(Arc::new(math::divide::divide::Divide)));
    manifest.locators.insert("lib://flowstdlib/math/multiply/Multiply".to_string(),
                             Native(Arc::new(math::multiply::multiply::Multiply)));
    manifest.locators.insert("lib://flowstdlib/math/sqrt/Sqrt".to_string(),
                             Native(Arc::new(math::sqrt::sqrt::Sqrt)));
    manifest.locators.insert("lib://flowstdlib/math/subtract/Subtract".to_string(),
                             Native(Arc::new(math::subtract::subtract::Subtract)));

    manifest
}

