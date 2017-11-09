use parser::parser;

pub type Name<'a> = &'a str;

// Define a trait to be able to add a function to String
pub trait Validates {
	fn validate_fields(&self) -> parser::Result;
}

impl<'a> Validates for Name<'a> {
	fn validate_fields(&self) -> parser::Result {
		if self.is_empty() {
			return parser::Result::Error(format!("Name cannot have an empty or whitespace name"));
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
        let name= "";
        match name.validate_fields() {
            parser::Result::Error(e) => {},
            _ => {assert!(false)},
        }
    }

    #[test]
    fn validates_when_has_value() {
        let name : Name = "test";
        match name.validate_fields() {
            parser::Result::Valid => {},
            _ => {assert!(false)},
        }
    }
}