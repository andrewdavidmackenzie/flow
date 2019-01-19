use flowrlib::implementation_table::ImplementationLocator::Native;
use flowrlib::implementation_table::ImplementationLocatorTable;

pub fn get_ilt<'a>() -> ImplementationLocatorTable<'a> {
    let mut ilt = ImplementationLocatorTable::new();

    ilt.locators.insert("//flowstdlib/control/tap/Tap".to_string(), Native(&::control::tap::Tap));
    ilt.locators.insert("//flowstdlib/control/compare/Compare".to_string(), Native(&::control::compare::Compare{}));
    ilt.locators.insert("//flowstdlib/math/add/Add".to_string(), Native(&::math::add::Add{}));
    ilt.locators.insert("//flowstdlib/fmt/to_string/ToString".to_string(), Native(&::fmt::to_string::ToString{}));
    ilt.locators.insert("//flowstdlib/fmt/to_number/ToNumber".to_string(), Native(&::fmt::to_number::ToNumber{}));
    ilt.locators.insert("//flowstdlib/zero_fifo/Fifo".to_string(), Native(&::zero_fifo::Fifo{}));

    ilt
}