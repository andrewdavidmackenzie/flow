use parser::parser;

pub type Name = String;

// Define a trait to be able to add a function to String
pub trait Validates {
	fn validate_fields(&self, type_name: &str) -> parser::Result;
}

impl Validates for Name {
	fn validate_fields(&self, type_name: &str) -> parser::Result {
		if self.is_empty() {
			return parser::Result::Error(format!("{} cannot have an empty or whitespace name", type_name));
		}
		parser::Result::Valid
	}
}

#[cfg(test)]
mod tests {
    use description::name::{Name, Validates};
    use parser::parser;

    #[test]
    fn does_not_validate_when_empty() {
        let name = Name::new();
        match name.validate_fields("Name") {
            parser::Result::Error(e) => {},
            _ => {assert!(false)},
        }
    }

    #[test]
    fn validates_when_has_value() {
        let name : Name = "test".to_string();
        match name.validate_fields("Name") {
            parser::Result::Valid => {},
            _ => {assert!(false)},
        }
    }
}