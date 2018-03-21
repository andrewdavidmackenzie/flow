use std::fmt;

use model::name::Name;
use model::name::HasName;
use model::connection::HasRoute;
use model::function::Function;
use loader::loader::Validate;

// This structure is (optionally) found as part of a flow file - inline in the description
#[derive(Deserialize, Debug)]
pub struct FunctionReference {
    pub alias: Name,
    pub source: String,
    #[serde(skip_deserializing)]
    pub function: Function,
}

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
    }
}

impl fmt::Display for FunctionReference {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\t\t\t\t\talias: \t{}\n\t\t\t\t\timplementation:\n\t\t\t\t\t\t\t\tsource: \t{}\n",
               self.alias, self.source)
    }
}