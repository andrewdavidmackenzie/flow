use flowrlib::implementation_table::ImplementationLocator::Native;
use flowrlib::implementation_table::ImplementationLocatorTable;

pub fn get_ilt<'a>() -> ImplementationLocatorTable<'a> {
    let mut ilt = ImplementationLocatorTable::new();

    ilt.locators.insert("//flowr/args/get/Get".to_string(), Native(&::args::get::Get{}));
    ilt.locators.insert("//flowr/file_write/FileWrite".to_string(), Native(&::file::file_write::FileWrite{}));
    ilt.locators.insert("//flowr/stdio/readline/Readline".to_string(), Native(&::stdio::readline::Readline{}));
    ilt.locators.insert("//flowr/stdio/stdin/Stdin".to_string(), Native(&::stdio::stdin::Stdin{}));
    ilt.locators.insert("//flowr/stdio/stdout/Stdout".to_string(), Native(&::stdio::stdout::Stdout{}));
    ilt.locators.insert("//flowr/stdio/stderr/Stderr".to_string(), Native(&::stdio::stderr::Stderr{}));

    ilt
}