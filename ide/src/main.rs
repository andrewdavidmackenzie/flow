pub fn main() {
    println!("flowide: version = {}", env!("CARGO_PKG_VERSION"));
    println!("flowrlib: version = {}", flowrlib::info::version());
    println!("flowclib: version = {}", flowclib::info::version());
}