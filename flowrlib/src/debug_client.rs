use std::io;

pub trait DebugClient {
    fn display(&self, output: &str);
    fn read_input(&self, input: &mut String) -> io::Result<usize>;
}