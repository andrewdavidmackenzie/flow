use std::sync::Arc;

use flowrstructs::lib_manifest::{ImplementationLocator::Native, LibraryManifest};
use flowrstructs::manifest::MetaData;

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
    manifest.locators.insert("lib://flowstdlib/control/compare_switch/compare_switch".to_string(),
                             Native(Arc::new(control::compare_switch::CompareSwitch)));
    manifest.locators.insert("lib://flowstdlib/control/index/index".to_string(),
                             Native(Arc::new(control::index::Index)));
    manifest.locators.insert("lib://flowstdlib/control/join/join".to_string(),
                             Native(Arc::new(control::join::Join)));
    manifest.locators.insert("lib://flowstdlib/control/route/route".to_string(),
                             Native(Arc::new(control::route::Route)));
    manifest.locators.insert("lib://flowstdlib/control/select/select".to_string(),
                             Native(Arc::new(control::select::Select)));
    manifest.locators.insert("lib://flowstdlib/control/tap/tap".to_string(),
                             Native(Arc::new(control::tap::Tap)));

    // Data
    manifest.locators.insert("lib://flowstdlib/data/append/append".to_string(),
                             Native(Arc::new(data::append::Append)));
    manifest.locators.insert("lib://flowstdlib/data/accumulate/accumulate".to_string(),
                             Native(Arc::new(data::accumulate::Accumulate)));
    manifest.locators.insert("lib://flowstdlib/data/buffer/buffer".to_string(),
                             Native(Arc::new(data::buffer::Buffer)));
    manifest.locators.insert("lib://flowstdlib/data/count/count".to_string(),
                             Native(Arc::new(data::count::Count)));
    manifest.locators.insert("lib://flowstdlib/data/duplicate_rows/duplicate_rows".to_string(),
                             Native(Arc::new(data::duplicate_rows::DuplicateRows)));
    manifest.locators.insert("lib://flowstdlib/data/duplicate/duplicate".to_string(),
                             Native(Arc::new(data::duplicate::Duplicate)));
    manifest.locators.insert("lib://flowstdlib/data/enumerate/enumerate".to_string(),
                             Native(Arc::new(data::enumerate::Enumerate)));
    manifest.locators.insert("lib://flowstdlib/data/info/info".to_string(),
                             Native(Arc::new(data::info::Info)));
    manifest.locators.insert("lib://flowstdlib/data/multiply_row/multiply_row".to_string(),
                             Native(Arc::new(data::multiply_row::MultiplyRow)));
    manifest.locators.insert("lib://flowstdlib/data/ordered_split/ordered_split".to_string(),
                             Native(Arc::new(data::ordered_split::OrderedSplit)));
    manifest.locators.insert("lib://flowstdlib/data/remove/remove".to_string(),
                             Native(Arc::new(data::remove::Remove)));
    manifest.locators.insert("lib://flowstdlib/data/sort/sort".to_string(),
                             Native(Arc::new(data::sort::Sort)));
    manifest.locators.insert("lib://flowstdlib/data/split/split".to_string(),
                             Native(Arc::new(data::split::Split)));
    manifest.locators.insert("lib://flowstdlib/data/transpose/transpose".to_string(),
                             Native(Arc::new(data::transpose::Transpose)));
    manifest.locators.insert("lib://flowstdlib/data/zip/zip".to_string(),
                             Native(Arc::new(data::zip::Zip)));

    // Format
    manifest.locators.insert("lib://flowstdlib/fmt/reverse/reverse".to_string(),
                             Native(Arc::new(fmt::reverse::Reverse)));
    manifest.locators.insert("lib://flowstdlib/fmt/to_json/to_json".to_string(),
                             Native(Arc::new(fmt::to_json::ToJson)));
    manifest.locators.insert("lib://flowstdlib/fmt/to_string/to_string".to_string(),
                             Native(Arc::new(fmt::to_string::ToString)));

    // Img
    manifest.locators.insert("lib://flowstdlib/img/format_png/format_png".to_string(),
                             Native(Arc::new(img::format_png::FormatPNG)));

    // Math
    manifest.locators.insert("lib://flowstdlib/math/add/add".to_string(),
                             Native(Arc::new(math::add::Add)));
    manifest.locators.insert("lib://flowstdlib/math/compare/compare".to_string(),
                             Native(Arc::new(math::compare::Compare)));
    manifest.locators.insert("lib://flowstdlib/math/divide/divide".to_string(),
                             Native(Arc::new(math::divide::Divide)));
    manifest.locators.insert("lib://flowstdlib/math/multiply/multiply".to_string(),
                             Native(Arc::new(math::multiply::Multiply)));
    manifest.locators.insert("lib://flowstdlib/math/sqrt/sqrt".to_string(),
                             Native(Arc::new(math::sqrt::Sqrt)));
    manifest.locators.insert("lib://flowstdlib/math/subtract/subtract".to_string(),
                             Native(Arc::new(math::subtract::Subtract)));

    manifest
}

