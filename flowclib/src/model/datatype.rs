const DATATYPES: &'static [&'static str] = &["String", "Json", "Number", "Bool"];

pub type DataType = String;

pub trait HasDataType {
    fn datatype(&self) -> &str;
}

pub trait TypeCheck {
    fn valid(&self) -> Result<(), String>;
}

impl TypeCheck for DataType {
    fn valid(&self) -> Result<(), String> {
        if DATATYPES.contains(&&self[..]) {
            return Ok(());
        }

        Err(format!("Type '{}' is unknown", &self))
    }
}

#[test]
fn valid_data_string_type() {
    let string_type = DataType::from("String".to_string());
    string_type.valid().unwrap();
}

#[test]
fn valid_data_json_type() {
    let json_type = DataType::from("Json".to_string());
    json_type.valid().unwrap();
}

#[test]
#[should_panic]
fn invalid_data_type() {
    let string_type = DataType::from("foo".to_string());
    string_type.valid().unwrap();
}
