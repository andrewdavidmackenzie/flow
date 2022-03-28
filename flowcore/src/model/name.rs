use std::fmt;

use error_chain::bail;
use serde_derive::{Deserialize, Serialize};
use shrinkwraprs::Shrinkwrap;

use crate::errors::*;
use crate::model::route::Route;
use crate::model::validation::Validate;

/// `Name` is a String that names various types of objects
#[derive(Shrinkwrap, Hash, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Name(String);

/// Implement Name struct
impl Name {
    /// Return true if the Name is empty
    pub fn empty(&self) -> bool {
        self.is_empty()
    }
}

/// Trait implemented by objects that have a Name
pub trait HasName {
    /// Return a reference to the name of the struct implementing this trait
    fn name(&self) -> &Name;
    /// Return a reference to the alias (also a Name type) of the struct implementing this trait
    fn alias(&self) -> &Name;
}

impl Validate for Name {
    fn validate(&self) -> Result<()> {
        // Names cannot be numbers as they can be confused with array indexes for array outputs
        if self.parse::<usize>().is_ok() {
            bail!(
                "Name '{}' cannot be a number, they are reserved for array indexes",
                self
            );
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

impl From<&Name> for Name {
    fn from(string: &Name) -> Self {
        string.clone()
    }
}

impl From<&Route> for Name {
    fn from(route: &Route) -> Self {
        Name::from(&route.to_string())
    }
}

#[cfg(test)]
mod test {
    use crate::model::validation::Validate;

    use super::Name;

    #[test]
    fn validates_when_empty() {
        let name = Name::default();
        assert!(name.validate().is_ok());
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
        name.validate().expect("Name did not validate as expected");
    }
}
