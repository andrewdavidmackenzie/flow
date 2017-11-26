use description::name::Name;
use description::name::HasName;
use description::name::HasRoute;
use loader::loader::Validate;

use std::fmt;

#[derive(Deserialize, Debug)]
pub struct Value {
    pub name: Name,
    pub datatype: Name,
    pub value: Option<String>,
    #[serde(skip_deserializing)]
    pub route: String,
}

// TODO figure out how to have this derived automatically for types needing it
impl HasName for Value {
    fn name(&self) -> &str {
        &self.name[..]
    }
}

impl HasRoute for Value {
    fn route(&self) -> &str {
        &self.route[..]
    }
}

impl Validate for Value {
    fn validate(&self) -> Result<(), String> {
        if let Some(ref value) = self.value {
            value.validate()?;
        }
        self.datatype.validate()
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\tname: \t\t{}\n\t\t\t\t\troute: \t\t{}\n\t\t\t\t\tdatatype: \t{}\n\t\t\t\t\tvalue: \t\t{:?}",
               self.name, self.route, self.datatype, self.value)
    }
}