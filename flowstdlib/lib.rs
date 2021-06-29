#![warn(clippy::unwrap_used)]

#[macro_use]
extern crate error_chain;

use std::sync::Arc;

use url::Url;

use flowcore::lib_manifest::{ImplementationLocator::Native, LibraryManifest};
use flowcore::manifest::MetaData;

/// Control functions
pub mod control;

/// Data functions
pub mod data;

/// Formatting functions
pub mod fmt;

/// Maths functions
pub mod math;

/// We'll put our errors in an `errors` module, and other modules in this crate will `use errors::*;`
/// to get access to everything `error_chain` creates.
#[doc(hidden)]
pub mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain! {}
}

// Specify the errors we will produce and foreign links
#[doc(hidden)]
error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    foreign_links {
        Url(url::ParseError);
    }
}

/// Return the `LibraryManifest` for the functions in the library
pub fn get_manifest() -> Result<LibraryManifest> {
    let metadata = MetaData {
        name: env!("CARGO_PKG_NAME").into(),
        version: env!("CARGO_PKG_VERSION").into(),
        description: env!("CARGO_PKG_DESCRIPTION").into(),
        authors: env!("CARGO_PKG_AUTHORS")
            .split(':')
            .map(|s| s.to_string())
            .collect(),
    };
    let mut manifest = LibraryManifest::new(metadata);

    // Control
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/control/compare_switch/compare_switch")?,
        Native(Arc::new(control::compare_switch::CompareSwitch)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/control/index/index")?,
        Native(Arc::new(control::index::Index)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/control/join/join")?,
        Native(Arc::new(control::join::Join)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/control/route/route")?,
        Native(Arc::new(control::route::Route)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/control/select/select")?,
        Native(Arc::new(control::select::Select)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/control/tap/tap")?,
        Native(Arc::new(control::tap::Tap)),
    );

    // Data
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/append/append")?,
        Native(Arc::new(data::append::Append)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/accumulate/accumulate")?,
        Native(Arc::new(data::accumulate::Accumulate)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/buffer/buffer")?,
        Native(Arc::new(data::buffer::Buffer)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/count/count")?,
        Native(Arc::new(data::count::Count)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/duplicate_rows/duplicate_rows")?,
        Native(Arc::new(data::duplicate_rows::DuplicateRows)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/duplicate/duplicate")?,
        Native(Arc::new(data::duplicate::Duplicate)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/enumerate/enumerate")?,
        Native(Arc::new(data::enumerate::Enumerate)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/info/info")?,
        Native(Arc::new(data::info::Info)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/multiply_row/multiply_row")?,
        Native(Arc::new(data::multiply_row::MultiplyRow)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/ordered_split/ordered_split")?,
        Native(Arc::new(data::ordered_split::OrderedSplit)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/remove/remove")?,
        Native(Arc::new(data::remove::Remove)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/sort/sort")?,
        Native(Arc::new(data::sort::Sort)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/split/split")?,
        Native(Arc::new(data::split::Split)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/transpose/transpose")?,
        Native(Arc::new(data::transpose::Transpose)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/zip/zip")?,
        Native(Arc::new(data::zip::Zip)),
    );

    // Format
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/fmt/reverse/reverse")?,
        Native(Arc::new(fmt::reverse::Reverse)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/fmt/to_json/to_json")?,
        Native(Arc::new(fmt::to_json::ToJson)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/fmt/to_string/to_string")?,
        Native(Arc::new(fmt::to_string::ToString)),
    );

    // Math
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/math/add/add")?,
        Native(Arc::new(math::add::Add)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/math/compare/compare")?,
        Native(Arc::new(math::compare::Compare)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/math/divide/divide")?,
        Native(Arc::new(math::divide::Divide)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/math/multiply/multiply")?,
        Native(Arc::new(math::multiply::Multiply)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/math/sqrt/sqrt")?,
        Native(Arc::new(math::sqrt::Sqrt)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/math/subtract/subtract")?,
        Native(Arc::new(math::subtract::Subtract)),
    );

    Ok(manifest)
}

#[cfg(test)]
mod test {
    use std::path::Path;

    #[test]
    fn check_manifest() {
        // check the manifest was created
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("manifest.json");
        assert!(manifest.exists());
    }

    #[test]
    fn get_manifest_test() {
        let manifest = super::get_manifest().expect("Could not get manifest");
        assert_eq!(manifest.locators.len(), 30);
    }
}
