use description::name::Name;
use description::io::IOSet;
use std::fmt;

#[derive(Deserialize, Debug)]
pub struct EntityRef {
    pub name: Name,
    pub source: String
}

impl fmt::Display for EntityRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Entity:\n\tname: {}\n\tsource: {}", self.name, self.source)
    }
}

#[derive(Deserialize)]
pub struct Entity {
	pub name: Name,
	pub ios: IOSet,
}

/*impl Entity {
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
}*/
