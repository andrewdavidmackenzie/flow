const DATATYPES: &'static [&'static str] = &["String"];

pub type DataType = String;

pub trait HasDataType {
    fn datatype(&self) -> &str;
}

pub trait TypeCheck {
    fn validate(&self) -> Result<(), String>;
}

impl TypeCheck for DataType {
    fn validate(&self) -> Result<(), String> {
        if DATATYPES.contains(&&self[..]) {
            return Ok(());
        }

        Err(format!("DataType '{}' is unknown", &self))
    }
}

#[test]
fn valid_data_type() {
    let string_type = DataType::from("String".to_string());
    string_type.validate().unwrap();
}

#[test]
#[should_panic]
fn invalid_data_type() {
    let string_type = DataType::from("foo".to_string());
    string_type.validate().unwrap();
}
