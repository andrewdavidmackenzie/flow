use std::io;
use std::io::Write;

use flowrlib::debug_client::DebugClient;

/*
    A simple CLI (i.e. stdin and stdout) debug client that implements the DebugClient trait
    defined in the flowrlib library.
*/
pub struct CLIDebugClient {}

/*
    Implement a client for the debugger that reads and writes to standard input and output
*/
impl DebugClient for CLIDebugClient {
    fn init(&self) {
    }

    fn display(&self, output: &str) {
        print!("{}", output);
        io::stdout().flush().unwrap();
    }

    fn read_input(&self, input: &mut String) -> io::Result<usize> {
        io::stdin().read_line(input)
    }
}