use description::name::Name;
use description::name::Named;
use loader::loader::Validate;

use std::fmt;

#[derive(Deserialize, Debug)]
pub struct Value {
    pub name: Name,
    pub datatype: Name,
    pub value: Option<String>
}

// TODO figure out how to have this derived automatically for types needing it
impl Named for Value {
    fn name(&self) -> &str {
        &self.name[..]
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
        write!(f, "Value:\n\tname: {}\n\tdatatype: {}\n\tvalue: {:?}", self.name, self.datatype, self.value)
    }
}