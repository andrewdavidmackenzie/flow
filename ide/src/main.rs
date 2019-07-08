pub fn main() {
    println!("flowide: version = {}", env!("CARGO_PKG_VERSION"));
    println!("flowstdlib: version = {}", flowstdlib::info::version());
    println!("flowrlib: version = {}", flowrlib::info::version());
    println!("flowclib: version = {}", flowclib::info::version());
}