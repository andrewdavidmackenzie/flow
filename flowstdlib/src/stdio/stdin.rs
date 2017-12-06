use flowrlib::function::Function;

pub struct Stdin;

const DEFINITION: &'static str ="
name = 'Stdin'
[[output]]
name = 'stdin'
type = 'String'";

impl Stdin {
    fn run() -> String {
        use std::io::{self, Read};

        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        buffer
    }
}

impl Function for Stdin {
    fn define() -> &'static str {
        DEFINITION
    }
}