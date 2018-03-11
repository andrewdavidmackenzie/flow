use flowrlib::implementation::Implementation;

pub struct Stderr;

impl Implementation for Stderr {
    fn run(&self, mut inputs: Vec<Option<String>>) -> Option<String> {
        eprintln!("{}", inputs.remove(0).unwrap());
        None
    }
}