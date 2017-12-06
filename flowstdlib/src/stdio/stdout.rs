use flowrlib::function::Function;

pub struct Stdout {
    // some state
}

const DEFINITION: &'static str ="
name = 'Stdout'
[[input]]
name = 'stdout'
type = 'String'";

impl Stdout {
    fn run(stdout: String) {
        println!("{}", stdout);
    }
}

unsafe impl Sync for Stdout {}

impl Function for Stdout {
    fn define() -> &'static str {
        DEFINITION
    }
}