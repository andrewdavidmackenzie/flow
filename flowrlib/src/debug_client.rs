use std::io;

pub enum Param {
    Wildcard,
    Numeric(usize),
    Output((usize, String)),
    Input((usize, usize)),
    Block((usize, usize)),
}

pub enum Command {
    Breakpoint(Option<Param>),
    Continue,
    Delete(Option<Param>),
    Exit,
    Inspect,
    List,
    Print(Option<Param>),
    Reset,
    Step(Option<Param>)
}

pub trait DebugClient {
    fn init(&self);
    fn display(&self, output: &str);
    fn read_input(&self, input: &mut String) -> io::Result<usize>;
    fn get_command(&self, job_number: usize) -> Command;
}