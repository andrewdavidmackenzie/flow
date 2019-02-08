use std::sync::Arc;

use flowrlib::implementation_table::ImplementationLocator::Native;
use flowrlib::implementation_table::ImplementationLocatorTable;

pub fn get_ilt() -> ImplementationLocatorTable {
    let mut ilt = ImplementationLocatorTable::new();

    ilt.locators.insert("lib://flowr/args/get/Get".to_string(), Native(Arc::new(::args::get::Get{})));
    ilt.locators.insert("lib://flowr/file/file_write/FileWrite".to_string(), Native(Arc::new(::file::file_write::FileWrite{})));
    ilt.locators.insert("lib://flowr/stdio/readline/Readline".to_string(), Native(Arc::new(::stdio::readline::Readline{})));
    ilt.locators.insert("lib://flowr/stdio/stdin/Stdin".to_string(), Native(Arc::new(::stdio::stdin::Stdin{})));
    ilt.locators.insert("lib://flowr/stdio/stdout/Stdout".to_string(), Native(Arc::new(::stdio::stdout::Stdout{})));
    ilt.locators.insert("lib://flowr/stdio/stderr/Stderr".to_string(), Native(Arc::new(::stdio::stderr::Stderr{})));

    ilt
}