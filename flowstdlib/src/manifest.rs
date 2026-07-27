use std::sync::Arc;

use url::Url;

use flowcore::model::lib_manifest::ImplementationLocator::Native;
use flowcore::model::lib_manifest::LibraryManifest;
use flowcore::model::metadata::MetaData;

use crate::errors::Result;
use crate::{charts, control, data, fmt, math, matrix};

/// Return the `LibraryManifest` for this library
/// # Errors
///
/// Will return `Err` if the manifest cannot be created
#[allow(clippy::too_many_lines)]
pub fn get() -> Result<LibraryManifest> {
    let metadata = MetaData {
        name: env!("CARGO_PKG_NAME").into(),
        version: env!("CARGO_PKG_VERSION").into(),
        description: env!("CARGO_PKG_DESCRIPTION").into(),
        authors: env!("CARGO_PKG_AUTHORS")
            .split(':')
            .map(std::string::ToString::to_string)
            .collect(),
    };
    let lib_url = Url::parse(&format!("lib://{}", metadata.name))?;
    let mut manifest = LibraryManifest::new(lib_url, metadata);

    // Charts module functions
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/charts/histogram")?,
        Native(Arc::new(charts::histogram::Histogram)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/charts/time_series")?,
        Native(Arc::new(charts::time_series::TimeSeries)),
    );

    // Control module functions
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/control/compare_switch")?,
        Native(Arc::new(control::compare_switch::CompareSwitch)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/control/index")?,
        Native(Arc::new(control::index::Index)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/control/join")?,
        Native(Arc::new(control::join::Join)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/control/route")?,
        Native(Arc::new(control::route::Route)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/control/select")?,
        Native(Arc::new(control::select::Select)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/control/tap")?,
        Native(Arc::new(control::tap::Tap)),
    );

    // Data module functions
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/accumulate")?,
        Native(Arc::new(data::accumulate::Accumulate)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/append")?,
        Native(Arc::new(data::append::Append)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/array_extract")?,
        Native(Arc::new(data::array_extract::ArrayExtract)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/array_get")?,
        Native(Arc::new(data::array_get::ArrayGet)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/array_set")?,
        Native(Arc::new(data::array_set::ArraySet)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/bin_count")?,
        Native(Arc::new(data::bin_count::BinCount)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/min")?,
        Native(Arc::new(data::min::Min)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/max")?,
        Native(Arc::new(data::max::Max)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/avg")?,
        Native(Arc::new(data::avg::Avg)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/count")?,
        Native(Arc::new(data::count::Count)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/duplicate")?,
        Native(Arc::new(data::duplicate::Duplicate)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/enumerate")?,
        Native(Arc::new(data::enumerate::Enumerate)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/info")?,
        Native(Arc::new(data::info::Info)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/ordered_split")?,
        Native(Arc::new(data::ordered_split::OrderedSplit)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/remove")?,
        Native(Arc::new(data::remove::Remove)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/sort")?,
        Native(Arc::new(data::sort::Sort)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/split")?,
        Native(Arc::new(data::split::Split)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/data/zip")?,
        Native(Arc::new(data::zip::Zip)),
    );

    // Format module functions
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/fmt/reverse")?,
        Native(Arc::new(fmt::reverse::Reverse)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/fmt/to_json")?,
        Native(Arc::new(fmt::to_json::ToJson)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/fmt/to_string")?,
        Native(Arc::new(fmt::to_string::ToString)),
    );

    // Math module functions
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/math/add")?,
        Native(Arc::new(math::add::Add)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/math/compare")?,
        Native(Arc::new(math::compare::Compare)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/math/cos")?,
        Native(Arc::new(math::cos::Cos)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/math/divide")?,
        Native(Arc::new(math::divide::Divide)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/math/multiply")?,
        Native(Arc::new(math::multiply::Multiply)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/math/range_split")?,
        Native(Arc::new(math::range_split::RangeSplit)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/math/sin")?,
        Native(Arc::new(math::sin::Sin)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/math/sqrt")?,
        Native(Arc::new(math::sqrt::Sqrt)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/math/subtract")?,
        Native(Arc::new(math::subtract::Subtract)),
    );
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/math/tan")?,
        Native(Arc::new(math::tan::Tan)),
    );

    // Matrix module functions
    manifest.locators.insert(
        Url::parse("lib://flowstdlib/matrix/duplicate_rows")?,
        Native(Arc::new(matrix::duplicate_rows::DuplicateRows)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/matrix/multiply_row")?,
        Native(Arc::new(matrix::multiply_row::MultiplyRow)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/matrix/transpose")?,
        Native(Arc::new(matrix::transpose::Transpose)),
    );

    manifest.locators.insert(
        Url::parse("lib://flowstdlib/matrix/compose_matrix")?,
        Native(Arc::new(matrix::compose_matrix::ComposeMatrix)),
    );

    Ok(manifest)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod test {
    use super::*;

    #[test]
    fn native_manifest_loads() {
        let manifest = get().unwrap();
        assert!(
            !manifest.locators.is_empty(),
            "Native manifest should have locators"
        );
    }

    #[test]
    fn native_manifest_has_all_functions() {
        let manifest = get().unwrap();

        let expected = [
            // charts
            "lib://flowstdlib/charts/histogram",
            "lib://flowstdlib/charts/time_series",
            // control
            "lib://flowstdlib/control/compare_switch",
            "lib://flowstdlib/control/index",
            "lib://flowstdlib/control/join",
            "lib://flowstdlib/control/route",
            "lib://flowstdlib/control/select",
            "lib://flowstdlib/control/tap",
            // data
            "lib://flowstdlib/data/accumulate",
            "lib://flowstdlib/data/append",
            "lib://flowstdlib/data/array_extract",
            "lib://flowstdlib/data/array_get",
            "lib://flowstdlib/data/array_set",
            "lib://flowstdlib/data/bin_count",
            "lib://flowstdlib/data/count",
            "lib://flowstdlib/data/duplicate",
            "lib://flowstdlib/data/enumerate",
            "lib://flowstdlib/data/info",
            "lib://flowstdlib/data/max",
            "lib://flowstdlib/data/min",
            "lib://flowstdlib/data/avg",
            "lib://flowstdlib/data/ordered_split",
            "lib://flowstdlib/data/remove",
            "lib://flowstdlib/data/sort",
            "lib://flowstdlib/data/split",
            "lib://flowstdlib/data/zip",
            // fmt
            "lib://flowstdlib/fmt/reverse",
            "lib://flowstdlib/fmt/to_json",
            "lib://flowstdlib/fmt/to_string",
            // math
            "lib://flowstdlib/math/add",
            "lib://flowstdlib/math/compare",
            "lib://flowstdlib/math/cos",
            "lib://flowstdlib/math/divide",
            "lib://flowstdlib/math/multiply",
            "lib://flowstdlib/math/range_split",
            "lib://flowstdlib/math/sin",
            "lib://flowstdlib/math/sqrt",
            "lib://flowstdlib/math/subtract",
            "lib://flowstdlib/math/tan",
            // matrix
            "lib://flowstdlib/matrix/compose_matrix",
            "lib://flowstdlib/matrix/duplicate_rows",
            "lib://flowstdlib/matrix/multiply_row",
            "lib://flowstdlib/matrix/transpose",
        ];

        for url_str in &expected {
            let url = Url::parse(url_str).unwrap();
            assert!(
                manifest.locators.contains_key(&url),
                "Missing native implementation: {url_str}"
            );
        }

        assert_eq!(
            manifest.locators.len(),
            expected.len(),
            "Native manifest has {} entries but expected {}",
            manifest.locators.len(),
            expected.len()
        );
    }
}
