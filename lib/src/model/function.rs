use std::fmt;

use model::name::Name;
use model::name::HasName;
use model::io::IO;
use loader::loader::Validate;

#[derive(Default, Deserialize, Debug)]
pub struct Function {
    pub name: Name,
    pub input: Option<Vec<IO>>,
    pub output: Option<Vec<IO>>,
    pub implementation: Option<String>, // TODO for now
    #[serde(skip_deserializing)]
    pub route: String
}

impl HasName for Function {
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

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\t\t\t\t\t\tname: \t\t{}\n",
               self.name).unwrap();
        write!(f, "\t\t\t\t\t\t\tinputs: \t{:?}\n",
               self.input).unwrap();
        write!(f, "\t\t\t\t\t\t\toutputs: \t{:?}\n",
               self.output)
    }
}