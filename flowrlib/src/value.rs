
pub struct Value {
    pub initial_value: Option<&'static str>,
    pub value: Option<String>,
    pub output_count: u32, // TODO make private and have constructor
}

impl Value {
    pub fn update(&mut self, new_value: String) {
        self.value = Some(new_value);
        println!("value updated to: {:?}", &self.value);
    }
}