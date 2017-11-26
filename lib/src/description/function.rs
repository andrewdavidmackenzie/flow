use std::fmt;

use description::name::Name;
use description::name::Named;
use description::io::IO;
use loader::loader::Validate;

#[derive(Default, Deserialize, Debug)]
pub struct FunctionReference {
    #[serde(rename = "name")]
    pub alias: Name,
    pub source: String,
    #[serde(skip_deserializing)]
    pub route: String,
    #[serde(skip_deserializing)]
    pub function: Function
}

// TODO figure out how to have this derived automatically for types needing it
impl Named for FunctionReference {
    fn name(&self) -> &str {
        &self.alias[..]
    }
}

impl Validate for FunctionReference {
    fn validate(&self) -> Result<(), String> {
        self.alias.validate()
        // Pretty much anything is a valid PathBuf - so not sure how to validate source...
    }
}

impl fmt::Display for FunctionReference {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\t\t\t\talias: \t{}\n\t\t\t\t\troute: \t{}\n\t\t\t\t\timplementation:\n\t\t\t\t\t\t\tsource: \t{}\n",
               self.alias, self.route, self.source)
    }
}

#[derive(Default, Deserialize, Debug)]
pub struct Function {
    pub name: Name,
    pub input: Option<Vec<IO>>,
    pub output: Option<Vec<IO>>,
    pub implementation: Option<String>, // TODO for now
    #[serde(skip_deserializing)]
    pub route: String
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

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\t\t\t\t\t\tname: \t\t{}\t\n\t\t\t\t\t\t\troute: \t\t{}\n",
               self.name, self.route).unwrap();
        write!(f, "\t\t\t\t\t\t\tinputs: \t{:?}\n",
               self.input).unwrap();
        write!(f, "\t\t\t\t\t\t\toutputs: \t{:?}\n",
               self.output)
    }
}