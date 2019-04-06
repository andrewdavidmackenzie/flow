use std::sync::{Arc, Mutex};
use function::Function;

pub struct Flow {
    pub functions: Vec<Arc<Mutex<Function>>>,
}

impl Flow {
    pub fn new() -> Self {
        Flow {
            functions: Vec::<Arc<Mutex<Function>>>::new()
        }
    }

    pub fn add(&mut self, function: Function) {
        self.functions.push(Arc::new(Mutex::new(function)));
    }
}