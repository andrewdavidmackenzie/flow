use loader::loader::Validate;

pub type Name = String;

impl Validate for Name {
    fn validate(&self) -> Result<(), String> {
        if self.is_empty() {
            return Err(format!("Name cannot have an empty or whitespace name"));
        }
        Ok(())
    }
}

pub trait HasName {
    fn name(&self) -> &str;
}

pub trait HasRoute {
    fn route(&self) -> &str;
}

#[test]
fn does_not_validate_when_empty() {
    let name = "".to_string();
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