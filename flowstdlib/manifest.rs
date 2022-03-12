use std::sync::Arc;

use url::Url;

use flowcore::model::lib_manifest::ImplementationLocator::Native;
use flowcore::model::lib_manifest::LibraryManifest;
use flowcore::model::metadata::MetaData;

use crate::{control, data, fmt, math};
use crate::errors::Result;

/// Return the LibraryManifest for this library
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
    let lib_url = Url::parse(&format!("lib://{}", metadata.name))?;
    let mut manifest = LibraryManifest::new(lib_url, metadata);

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/data/zip")?,
            Native(Arc::new(data::zip::Zip)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/data/split")?,
            Native(Arc::new(data::split::Split)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/data/duplicate")?,
            Native(Arc::new(data::duplicate::Duplicate)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/data/duplicate_rows")?,
            Native(Arc::new(data::duplicate_rows::DuplicateRows)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/control/tap")?,
            Native(Arc::new(control::tap::Tap)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/data/append")?,
            Native(Arc::new(data::append::Append)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/data/count")?,
            Native(Arc::new(data::count::Count)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/control/select")?,
            Native(Arc::new(control::select::Select)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/control/compare_switch")?,
            Native(Arc::new(control::compare_switch::CompareSwitch)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/control/join")?,
            Native(Arc::new(control::join::Join)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/fmt/reverse")?,
            Native(Arc::new(fmt::reverse::Reverse)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/data/accumulate")?,
            Native(Arc::new(data::accumulate::Accumulate)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/control/route")?,
            Native(Arc::new(control::route::Route)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/fmt/to_json")?,
            Native(Arc::new(fmt::to_json::ToJson)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/data/buffer")?,
            Native(Arc::new(data::buffer::Buffer)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/data/ordered_split")?,
            Native(Arc::new(data::ordered_split::OrderedSplit)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/math/range_split")?,
            Native(Arc::new(math::range_split::RangeSplit)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/math/subtract")?,
            Native(Arc::new(math::subtract::Subtract)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/data/multiply_row")?,
            Native(Arc::new(data::multiply_row::MultiplyRow)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/math/compare")?,
            Native(Arc::new(math::compare::Compare)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/control/index")?,
            Native(Arc::new(control::index::Index)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/data/enumerate")?,
            Native(Arc::new(data::enumerate::Enumerate)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/math/divide")?,
            Native(Arc::new(math::divide::Divide)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/data/remove")?,
            Native(Arc::new(data::remove::Remove)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/fmt/to_string")?,
            Native(Arc::new(fmt::to_string::ToString)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/math/sqrt")?,
            Native(Arc::new(math::sqrt::Sqrt)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/data/info")?,
            Native(Arc::new(data::info::Info)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/math/add")?,
            Native(Arc::new(math::add::Add)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/math/multiply")?,
            Native(Arc::new(math::multiply::Multiply)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/data/sort")?,
            Native(Arc::new(data::sort::Sort)),
        );

    manifest.locators.insert(
            Url::parse("lib://flowstdlib/data/transpose")?,
            Native(Arc::new(data::transpose::Transpose)),
        );

    Ok(manifest)
}