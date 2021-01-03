use std::fmt;

use error_chain::bail;
use serde_derive::{Deserialize, Serialize};
use shrinkwraprs::Shrinkwrap;

use crate::compiler::loader::Validate;
use crate::errors::*;
use crate::model::route::Route;

#[derive(Shrinkwrap, Hash, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Name(String);

// This is used for serde in io.rs
impl Name {
    pub fn empty(&self) -> bool {
        self.is_empty()
    }
}

impl Validate for Name {
    fn validate(&self) -> Result<()> {
        if self.is_empty() {
            bail!("Name '{}' cannot have an empty or whitespace name", self);
        }

        // Names cannot be numbers as they can be confused with array indexes for Array outputs
        if self.parse::<usize>().is_ok() {
            bail!("Name '{}' cannot be a number, they are reserved for array indexes", self);
        }

        Ok(())
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for Name {
    fn from(string: &str) -> Self {
        Name(string.to_string())
    }
}

impl From<String> for Name {
    fn from(string: String) -> Self {
        Name(string)
    }
}

impl From<&String> for Name {
    fn from(string: &String) -> Self {
        Name(string.to_string())
    }
}

impl From<&Route> for Name {
    fn from(route: &Route) -> Self {
        Name::from(&route.to_string())
    }
}

pub trait HasName {
    fn name(&self) -> &Name;
    fn alias(&self) -> &Name;
}

#[cfg(test)]
mod test {
    use crate::compiler::loader::Validate;

    use super::Name;

    #[test]
    fn does_not_validate_when_empty() {
        let name = Name::default();
        if name.validate().is_ok() {
            panic!()
        }
    }

    #[test]
    fn number_does_not_validate() {
        let name = Name::from("123");
        if name.validate().is_ok() {
            panic!();
        }
    }

    #[test]
    fn validates_when_has_value() {
        let name: Name = Name::from("test");
        name.validate().unwrap();
    }
}