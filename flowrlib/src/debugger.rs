pub struct Debugger {}

impl Debugger {
    pub fn new() -> Self {
        Debugger {}
    }

    pub fn enter(&mut self) {
        println!("Debug> ");
    }
}
