use std::io;

pub trait DebugClient {
    // TODO change to accept FromStr ???
    fn display(&self, output: &str);
    fn read_input(&self, input: &mut String) -> io::Result<usize>;
}