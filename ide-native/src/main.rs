mod flow;

pub fn main() {
    flowclib_version();
}

fn flowclib_version() {
    let flowclib_version = flow::version();
    println!("Flowclib: {}", flowclib_version);
}