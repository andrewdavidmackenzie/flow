use description::name::{Name, Validates};
use description::io::IOSet;
use parser::parser;

pub struct Entity {
	pub name: Name,
	pub ios: IOSet,
}

impl Entity {
	pub fn load(&self) -> parser::Result {
		// TODO check entity can be found in system
		// and load it's ioSet
		parser::Result::Valid
	}

	pub fn validate_fields(&self) -> parser::Result  {
		self.name.validate_fields("Entity"); // TODO early return
		self.ios.validate_fields(); // TODO early return

		parser::Result::Valid
	}
}
