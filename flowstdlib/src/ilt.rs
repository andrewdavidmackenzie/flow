use flowrlib::implementation_table::ImplementationLocator::Native;
use flowrlib::implementation_table::ImplementationLocator::Wasm;
use flowrlib::implementation_table::ImplementationLocatorTable;
use std::sync::Arc;

pub fn get_ilt() -> ImplementationLocatorTable {
    let mut ilt = ImplementationLocatorTable::new();

    ilt.locators.insert("lib://flowstdlib/math/add/Add".to_string(),
                        Native(Arc::new(::math::add::Add {})));
    ilt.locators.insert("lib://flowstdlib/math/divide/Divide".to_string(),
                        Native(Arc::new(::math::divide::Divide {})));
    ilt.locators.insert("lib://flowstdlib/math/subtract/Subtract".to_string(),
                        Native(Arc::new(::math::subtract::Subtract{})));

    ilt.locators.insert("lib://flowstdlib/control/tap/Tap".to_string(),
                        Native(Arc::new(::control::tap::Tap)));
    ilt.locators.insert("lib://flowstdlib/control/compare/Compare".to_string(),
                        Native(Arc::new(::control::compare::Compare {})));
    ilt.locators.insert("lib://flowstdlib/fmt/to_string/ToString".to_string(),
                        Native(Arc::new(::fmt::to_string::ToString {})));
    ilt.locators.insert("lib://flowstdlib/fmt/to_number/ToNumber".to_string(),
                        Native(Arc::new(::fmt::to_number::ToNumber {})));
    ilt.locators.insert("lib://flowstdlib/img/format_png/FormatPNG".to_string(),
                        Native(Arc::new(::img::format_png::FormatPNG {})));

    ilt.locators.insert("lib://flowstdlib/data/buffer/Buffer".to_string(),
                        Native(Arc::new(::data::buffer::Buffer {})));
    ilt.locators.insert("lib://flowstdlib/data/compose_array/ComposeArray".to_string(),
                        Native(Arc::new(::data::compose_array::ComposeArray {})));
    ilt.locators.insert("lib://flowstdlib/data/zip/Zip".to_string(),
                        Native(Arc::new(::data::zip::Zip {})));

    // TODO remove this fake added wasm function with a real one, with a Makefile to build it when wasm execution is ready
    ilt.locators.insert("lib://flowstdlib/fmt/reverse/Reverse".to_string(),
                        Wasm(("src/fmt/reverse.wasm".to_string(), "reverse".to_string())));

    ilt
}