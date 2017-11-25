use description::name::Name;
use description::name::Named;
use description::io::IO;
use loader::loader::Validate;

use std::path::PathBuf;

#[derive(Deserialize, Debug)]
pub struct Function {
    #[serde(skip_deserializing)]
    pub source: PathBuf,
    pub name: Name,
    pub input: Option<Vec<IO>>,
    pub output: Option<Vec<IO>>,
    pub implementation: Option<String> // TODO for now
}

// TODO figure out how to have this derived automatically for types needing it
impl Named for Function {
    fn name(&self) -> &str {
        &self.name[..]
    }
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