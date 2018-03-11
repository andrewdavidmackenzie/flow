use flowrlib::implementation::Implementation;

pub struct Stdout;

impl Implementation for Stdout {
    fn run(&self, mut inputs: Vec<Option<String>>) -> Option<String> {
        println!("{}", inputs.remove(0).unwrap());
        None
    }
}