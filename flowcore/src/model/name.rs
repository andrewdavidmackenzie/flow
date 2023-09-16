use error_chain::bail;
use crate::errors::*;
use crate::model::validation::Validate;

/// `Name` is a String that names various types of objects
pub type Name = String;

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
