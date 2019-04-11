use function::Function;
use manifest::{MetaData, Manifest};

pub struct Flow {
    pub metadata: MetaData,
    pub functions: Vec<Function>,
}

impl Flow {
    pub fn new(manifest: &Manifest) -> Self {
        Flow {
            metadata: manifest.metadata.clone(),
            functions: Vec::<Function>::new()
        }
    }

    /*
        Add a Function to the flow so it can be used while running the flow
    */
    pub fn add(&mut self, function: Function) {
        // wrap in an Arc and Mutex so it can be used between multiple threads
        self.functions.push(function);
    }
}