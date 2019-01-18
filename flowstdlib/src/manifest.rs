use flowrlib::implementation_table::ImplementationLocator::Native;
use flowrlib::implementation_table::ImplementationLocator::Wasm;
use flowrlib::implementation_table::ImplementationLocatorTable;

pub fn get_manifest<'a>() -> ImplementationLocatorTable<'a> {
    let mut manifest = ImplementationLocatorTable::new();

    manifest.insert("//flowstdlib/control/tap/Tap", Native(&::control::tap::Tap));
    manifest.insert("//flowstdlib/control/compare/Compare", Native(&::control::compare::Compare{}));
    manifest.insert("//flowstdlib/math/add/Add", Native(&::math::add::Add{}));
    manifest.insert("//flowstdlib/fmt/to_string/ToString", Native(&::fmt::to_string::ToString{}));
    manifest.insert("//flowstdlib/fmt/to_number/ToNumber", Native(&::fmt::to_number::ToNumber{}));
    manifest.insert("//flowstdlib/zero_fifo/Fifo", Native(&::zero_fifo::Fifo{}));

    // TODO remove
    manifest.insert("//flowstdlib/test/wasm", Wasm("test.wasm"));

    manifest
}