use flowrlib::implementation_table::ImplementationLocator::Native;
use flowrlib::implementation_table::ImplementationLocatorTable;
use std::sync::Arc;

pub fn get_ilt() -> ImplementationLocatorTable {
    let mut ilt = ImplementationLocatorTable::new();

    ilt.locators.insert("lib://runtime/args/get/Get".to_string(), Native(Arc::new(super::args::get::Get)));
    ilt.locators.insert("lib://runtime/file/file_write/FileWrite".to_string(), Native(Arc::new(super::file::file_write::FileWrite{})));
    ilt.locators.insert("lib://runtime/stdio/readline/Readline".to_string(), Native(Arc::new(super::stdio::readline::Readline{})));
    ilt.locators.insert("lib://runtime/stdio/stdin/Stdin".to_string(), Native(Arc::new(super::stdio::stdin::Stdin{})));
    ilt.locators.insert("lib://runtime/stdio/stdout/Stdout".to_string(), Native(Arc::new(super::stdio::stdout::Stdout{})));
    ilt.locators.insert("lib://runtime/stdio/stderr/Stderr".to_string(), Native(Arc::new(super::stdio::stderr::Stderr{})));

    ilt
}