use flowrlib::implementation::Implementation;

pub struct Add;

impl Implementation for Add {
    fn run(&self, inputs: Vec<Option<String>>) -> Option<String> {
        let i1 = inputs[0].clone().unwrap().parse::<i32>().unwrap();
        let i2 = inputs[1].clone().unwrap().parse::<i32>().unwrap();
        let o1 = i1 + i2;
        Some(o1.to_string())
    }
}