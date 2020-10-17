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
        name: env!("CARGO_PKG_NAME").into(),
        version: env!("CARGO_PKG_VERSION").into(),
        description: env!("CARGO_PKG_DESCRIPTION").into(),
        authors: env!("CARGO_PKG_AUTHORS").split(':').map(|s| s.to_string()).collect()
    };
    let mut manifest = LibraryManifest::new(metadata);


    // Control
    manifest.locators.insert("lib://flowstdlib/control/compare_switch/CompareSwitch".to_string(),
                             Native(Arc::new(control::compare_switch::CompareSwitch)));
    manifest.locators.insert("lib://flowstdlib/control/index/Index".to_string(),
                             Native(Arc::new(control::index::Index)));
    manifest.locators.insert("lib://flowstdlib/control/join/Join".to_string(),
                             Native(Arc::new(control::join::Join)));
    manifest.locators.insert("lib://flowstdlib/control/route/Route".to_string(),
                             Native(Arc::new(control::route::Route)));
    manifest.locators.insert("lib://flowstdlib/control/select/Select".to_string(),
                             Native(Arc::new(control::select::Select)));
    manifest.locators.insert("lib://flowstdlib/control/tap/Tap".to_string(),
                             Native(Arc::new(control::tap::Tap)));

    // Data
    manifest.locators.insert("lib://flowstdlib/data/append/Append".to_string(),
                             Native(Arc::new(data::append::Append)));
    manifest.locators.insert("lib://flowstdlib/data/accumulate/Accumulate".to_string(),
                             Native(Arc::new(data::accumulate::Accumulate)));
    manifest.locators.insert("lib://flowstdlib/data/buffer/Buffer".to_string(),
                             Native(Arc::new(data::buffer::Buffer)));
    manifest.locators.insert("lib://flowstdlib/data/count/Count".to_string(),
                             Native(Arc::new(data::count::Count)));
    manifest.locators.insert("lib://flowstdlib/data/duplicate_rows/DuplicateRows".to_string(),
                             Native(Arc::new(data::duplicate_rows::DuplicateRows)));
    manifest.locators.insert("lib://flowstdlib/data/duplicate/Duplicate".to_string(),
                             Native(Arc::new(data::duplicate::Duplicate)));
    manifest.locators.insert("lib://flowstdlib/data/enumerate/Enumerate".to_string(),
                             Native(Arc::new(data::enumerate::Enumerate)));
    manifest.locators.insert("lib://flowstdlib/data/info/Info".to_string(),
                             Native(Arc::new(data::info::Info)));
    manifest.locators.insert("lib://flowstdlib/data/multiply_row/MultiplyRow".to_string(),
                             Native(Arc::new(data::multiply_row::MultiplyRow)));
    manifest.locators.insert("lib://flowstdlib/data/ordered_split/OrderedSplit".to_string(),
                             Native(Arc::new(data::ordered_split::OrderedSplit)));
    manifest.locators.insert("lib://flowstdlib/data/remove/Remove".to_string(),
                             Native(Arc::new(data::remove::Remove)));
    manifest.locators.insert("lib://flowstdlib/data/sort/Sort".to_string(),
                             Native(Arc::new(data::sort::Sort)));
    manifest.locators.insert("lib://flowstdlib/data/split/Split".to_string(),
                             Native(Arc::new(data::split::Split)));
    manifest.locators.insert("lib://flowstdlib/data/transpose/Transpose".to_string(),
                             Native(Arc::new(data::transpose::Transpose)));
    manifest.locators.insert("lib://flowstdlib/data/zip/Zip".to_string(),
                             Native(Arc::new(data::zip::Zip)));

    // Format
    manifest.locators.insert("lib://flowstdlib/fmt/reverse/Reverse".to_string(),
                             Native(Arc::new(fmt::reverse::Reverse)));
    manifest.locators.insert("lib://flowstdlib/fmt/to_json/ToJson".to_string(),
                             Native(Arc::new(fmt::to_json::ToJson)));
    manifest.locators.insert("lib://flowstdlib/fmt/to_string/ToString".to_string(),
                             Native(Arc::new(fmt::to_string::ToString)));

    // Img
    manifest.locators.insert("lib://flowstdlib/img/format_png/FormatPNG".to_string(),
                             Native(Arc::new(img::format_png::FormatPNG)));

    // Math
    manifest.locators.insert("lib://flowstdlib/math/add/Add".to_string(),
                             Native(Arc::new(math::add::Add)));
    manifest.locators.insert("lib://flowstdlib/math/compare/Compare".to_string(),
                             Native(Arc::new(math::compare::Compare)));
    manifest.locators.insert("lib://flowstdlib/math/divide/Divide".to_string(),
                             Native(Arc::new(math::divide::Divide)));
    manifest.locators.insert("lib://flowstdlib/math/multiply/Multiply".to_string(),
                             Native(Arc::new(math::multiply::Multiply)));
    manifest.locators.insert("lib://flowstdlib/math/sqrt/Sqrt".to_string(),
                             Native(Arc::new(math::sqrt::Sqrt)));
    manifest.locators.insert("lib://flowstdlib/math/subtract/Subtract".to_string(),
                             Native(Arc::new(math::subtract::Subtract)));

    manifest
}

