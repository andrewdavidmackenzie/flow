use description::name::Name;

pub struct Function {
	name: Name
}

/*
impl Function {
	pub fn load(&self) -> parser::Result {
		// TODO check entity can be found in system
		// and load it's ioSet
		parser::Result::Valid
	}

	fn validate_fields(&self) -> parser::Result {
		self.name.validate_fields("Function"); // TODO early return
		self.ios.validate_fields()
	}
}
*/