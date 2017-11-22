use loader::loader::Validate;
use description::datatype::DataType;

pub type IORef = String;

#[derive(Deserialize, Debug)]
pub struct IO {
	name: IORef,
	datatype: DataType,
    function: Option<String>, // TODO for now
    value: Option<String> // TODO for now
}

impl Validate for IO {
	fn validate(&self) -> Result<(), String> {
		self.name.validate()?;
		// TODO validate datatype exists?

        Ok(())
	}
}