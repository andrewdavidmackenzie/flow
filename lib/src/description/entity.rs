use loader::loader::Validate;
use description::name::Name;
use description::connection::IOName;
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
	pub io: Vec<IOName>,
    function: Option<String>, // TODO for now
    value: Option<String> // TODO for now
}

impl Validate for Entity {
	fn validate(&self) -> Result<(), String> {
		self.name.validate()
	}
}
