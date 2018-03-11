use flowrlib::implementation::Implementation;

pub struct Readline;

impl Implementation for Readline {
    fn run(&self, _inputs: Vec<Option<String>>) -> Option<String> {
        use std::io::{self, BufRead};

        let stdin = io::stdin();
        let mut iterator = stdin.lock().lines();
        let line = iterator.next().unwrap().unwrap();
        Some(line)
    }
}