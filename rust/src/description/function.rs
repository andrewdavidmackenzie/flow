use description::io::IOSet;
use description::name::{Name, Validates};
use parser::parser;

pub struct Function {
	name: Name,
	pub ios: IOSet,
}

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