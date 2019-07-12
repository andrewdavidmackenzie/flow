extern crate flowclib;

#[no_mangle]
pub extern fn version() -> &'static str {
    flowclib::info::version()
}