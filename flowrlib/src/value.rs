use serde_json::Value as JsonValue;
use runnable::Runnable;
use implementation::Implementation;

const ONLY_INPUT: usize = 0;

pub struct Value {
    name: String,
    number_of_inputs: usize,
    id: usize,
    initial_value: Option<JsonValue>,
    implementation: Box<Implementation>,
    value: JsonValue,
    output_routes: Vec<(&'static str, usize, usize)>,
}

impl Value {
    pub fn new(name: &str,
               number_of_inputs: usize,
               _input_depths: Vec<usize>,
               id: usize,
               implementation: Box<Implementation>,
               initial_value: Option<JsonValue>,
               output_routes: Vec<(&'static str, usize, usize)>) -> Value {
        Value {
            name: name.to_string(),
            number_of_inputs,
            id,
            initial_value,
            implementation,
            value: JsonValue::Null,
            output_routes,
        }
    }
}

impl Runnable for Value {
    fn name(&self) -> &str {
        &self.name
    }

    fn number_of_inputs(&self) -> usize { self.number_of_inputs }

    fn id(&self) -> usize { self.id }

    /*
        If an initial value is defined then write it to the current value.
        Return true if ready to run as all inputs (single in this case) are satisfied.
    */
    fn init(&mut self) -> bool {
        let value = self.initial_value.clone();
        if let Some(v) = value {
            debug!("\tValue initialized by writing '{:?}' to input", &v);
            self.write_input(ONLY_INPUT, v);
        }
        self.can_run()
    }

    /*
        Update the value stored - this should only be called when the value has already been
        consumed by all the listeners and hence it can be overwritten.
    */
    fn write_input(&mut self, _input_number: usize, input_value: JsonValue) {
        self.value = input_value;
    }

    fn input_full(&self, _input_number: usize) -> bool {
        !self.value.is_null()
    }

    // Responds true if all inputs have been satisfied and can be run - false otherwise
    fn can_run(&self) -> bool {
        !self.value.is_null()
    }

    fn get_inputs(&mut self) -> Vec<Vec<JsonValue>> {
        if self.number_of_inputs == 0 { // never get's refreshed, is a constant!
            vec!(vec!(self.value.clone()))
        } else {
            vec!(vec!(self.value.take())) // consume the value and it will get refilled later
        }
    }

    fn output_destinations(&self) -> &Vec<(&'static str, usize, usize)> {
        &self.output_routes
    }

    fn implementation(&self) -> &Box<Implementation> { &self.implementation }
}

#[cfg(test)]
mod test {
    use serde_json::Value as JsonValue;

    #[test]
    fn destructure_output_base_route() {
        let json = json!("my_value");
        assert_eq!(json.pointer("").unwrap(), "my_value");
    }

    #[test]
    fn destructure_json_value() {
        let json: JsonValue = json!({ "sub_route": "sub_value" });
        assert_eq!(json.pointer("/sub_route").unwrap(), "sub_value");
    }
}