use compiler::loader::Validate;

pub type Name = String;

impl Validate for Name {
    fn validate(&self) -> Result<(), String> {
        if self.is_empty() {
            return Err(format!("Name '{}' cannot have an empty or whitespace name", self));
        }

        // Names cannot be numbers as they can be confused with array indexes for Array outputs
        if let Ok(_) = self.parse::<usize>() {
            return Err(format!("Name '{}' cannot be a number, they are reserved for array indexes", self));
        }

        Ok(())
    }
}

pub trait HasName {
    fn name(&self) -> &Name;
    fn alias(&self) -> &Name;
}

#[cfg(test)]
mod test {
    use super::Name;
    use compiler::loader::Validate;

    #[test]
    fn does_not_validate_when_empty() {
        let name = "".to_string();
        match name.validate() {
            Err(_) => {}
            Ok(_) => { assert!(false) }
        }
    }

    #[test]
    fn number_does_not_validate() {
        let name = "123".to_string();
        match name.validate() {
            Err(_) => {}
            Ok(_) => { assert!(false) }
        }
    }

    #[test]
    fn validates_when_has_value() {
        let name: Name = "test".to_string();
        match name.validate() {
            Ok(_) => {}
            Err(_) => { assert!(false) }
        }
    }
}