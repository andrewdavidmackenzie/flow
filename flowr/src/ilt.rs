use flowrlib::implementation_table::ImplementationLocator::Native;
use flowrlib::implementation_table::ImplementationLocatorTable;

pub fn get_ilt<'a>() -> ImplementationLocatorTable<'a> {
    let mut ilt = ImplementationLocatorTable::new();

    ilt.locators.insert("lib://flowr/args/get/Get".to_string(), Native(&::args::get::Get{}));
    ilt.locators.insert("lib://flowr/file_write/FileWrite".to_string(), Native(&::file::file_write::FileWrite{}));
    ilt.locators.insert("lib://flowr/stdio/readline/Readline".to_string(), Native(&::stdio::readline::Readline{}));
    ilt.locators.insert("lib://flowr/stdio/stdin/Stdin".to_string(), Native(&::stdio::stdin::Stdin{}));
    ilt.locators.insert("lib://flowr/stdio/stdout/Stdout".to_string(), Native(&::stdio::stdout::Stdout{}));
    ilt.locators.insert("lib://flowr/stdio/stderr/Stderr".to_string(), Native(&::stdio::stderr::Stderr{}));

    ilt
}