use flowrlib::implementation::Implementation;

pub struct Stdin;

impl Implementation for Stdin {
    fn run(&self, _inputs: Vec<Option<String>>) -> Option<String> {
        use std::io::{self, Read};

        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        Some(buffer)
    }
}