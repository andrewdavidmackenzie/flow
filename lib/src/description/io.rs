use loader::loader::Validate;
use description::datatype::DataType;
use description::name::Name;

pub type IORef = String;

#[derive(Deserialize, Debug)]
pub struct IO {
	pub name: Name, // Input/Output points on Entities, Flows, Sinks, Sources have unique names
	datatype: DataType,
}

impl Validate for IO {
	fn validate(&self) -> Result<(), String> {
		self.name.validate() // TODO early return here try!() ????
		// TODO validate datatype exists?
	}
}