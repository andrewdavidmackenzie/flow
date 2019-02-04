use std::io;
use std::io::Write;
use runlist::RunList;

pub struct Debugger {
}

impl Debugger {
    pub fn new() -> Self {
        Debugger {}
    }

    pub fn enter(&self, _run_list: &RunList) {
        let mut input = String::new();
        loop {
            print!("Debug> ");
            io::stdout().flush().unwrap();
            match io::stdin().read_line(&mut input) {
                Ok(_n) => {
                    // parse command
                    // if continue, then return
                    return;
                }
                Err(_) => {}
            };
        }
    }
}
