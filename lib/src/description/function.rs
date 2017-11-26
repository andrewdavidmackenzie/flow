use std::fmt;

use description::name::Name;
use description::name::Named;
use description::io::IO;
use loader::loader::Validate;

#[derive(Default, Deserialize, Debug)]
pub struct FunctionReference {
    #[serde(rename = "name")]
    pub reference_name: Name,
    pub source: String,
    #[serde(skip_deserializing)]
    pub function: Function,
    #[serde(skip_deserializing)]
    pub hierarchy_name: String
}

// TODO figure out how to have this derived automatically for types needing it
impl Named for FunctionReference {
    fn name(&self) -> &str {
        &self.reference_name[..]
    }
}

impl Validate for FunctionReference {
    fn validate(&self) -> Result<(), String> {
        self.reference_name.validate()
        // Pretty much anything is a valid PathBuf - so not sure how to validate source...
    }
}

impl fmt::Display for FunctionReference {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FunctionReference:\n\tname: {}\n\thierarchy_name: {}\n\tsource: {}",
               self.reference_name, self.hierarchy_name, self.source)
    }
}

#[derive(Default, Deserialize, Debug)]
pub struct Function {
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