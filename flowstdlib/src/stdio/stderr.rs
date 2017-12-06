use flowrlib::function::Function;

pub struct Stderr;

const DEFINITION: &'static str =
"name = 'Stderr'
[[input]]
name = 'stderr'
type = 'String'";

impl Stderr {
    pub fn run(stderr: String) {
        eprintln!("{}", stderr);
    }
}

impl Function for Stderr {
    fn define() -> &'static str {
        DEFINITION
    }
}