use description::name::Name;
use description::connection::IO;
use loader::loader::Validate;

use std::path::PathBuf;

#[derive(Deserialize, Debug)]
pub struct FunctionRef {
	pub name: Name,
	pub source: String
}

#[derive(Deserialize, Debug)]
pub struct Function {
    #[serde(skip_deserializing)]
    pub source: PathBuf,
    pub name: Name,
    pub input: Option<Vec<IO>>,
    pub output: Option<Vec<IO>>,
    pub implementation: Option<String> // TODO for now
}

impl Validate for Function {
	fn validate(&self) -> Result<(), String> {
		self.name.validate()?;

        if let Some(ref inputs) = self.input {
            for i in inputs {
                i.validate()?
            }
        }

        if let Some(ref outputs) = self.output {
            for o in outputs {
                o.validate()?
            }
        }

        Ok(())
	}
}