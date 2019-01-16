use flowrlib::implementation_table::ImplementationLocator::Native;
use flowrlib::implementation_table::ImplementationLocatorTable;

pub fn get_manifest<'a>() -> ImplementationLocatorTable<'a> {
    let mut manifest = ImplementationLocatorTable::new();

    manifest.insert("//flowr/args/get/Get", Native(&::args::get::Get{}));
    manifest.insert("//flowr/file_write/FileWrite", Native(&::file::file_write::FileWrite{}));
    manifest.insert("//flowr/stdio/readline/Readline", Native(&::stdio::readline::Readline{}));
    manifest.insert("//flowr/stdio/stdin/Stdin", Native(&::stdio::stdin::Stdin{}));
    manifest.insert("//flowr/stdio/stdout/Stdout", Native(&::stdio::stdout::Stdout{}));
    manifest.insert("//flowr/stdio/stderr/Stderr", Native(&::stdio::stderr::Stderr{}));

    manifest
}