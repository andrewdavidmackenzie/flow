pub struct Value {
	value: String, // TODO
}

impl Value {
	// Associated function
	pub fn new(value: &str) -> Value {
		Value {
			value: value.to_string()
		}
	}
}