use std::fmt;

use model::name::Name;
use model::name::HasName;
use model::name::HasRoute;
use model::function::Function;
use loader::loader::Validate;

#[derive(Default, Deserialize, Debug)]
pub struct FunctionReference {
    #[serde(rename = "name")]
    pub alias: Name,
    pub source: String,
    #[serde(skip_deserializing)]
    pub function: Function
}

// TODO figure out how to have this derived automatically for types needing it
impl HasName for FunctionReference {
    fn name(&self) -> &str {
        &self.alias[..]
    }
}

impl HasRoute for FunctionReference {
    fn route(&self) -> &str {
        &self.function.route[..]
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
        write!(f, "\t\t\t\talias: \t{}\n\t\t\t\t\timplementation:\n\t\t\t\t\t\t\tsource: \t{}\n",
               self.alias, self.source)
    }
}