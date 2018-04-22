const DATATYPES: &'static [&'static str] = &["String", "Json", "Number", "Bool", "Map", "Array"];

pub type DataType = String;

pub trait HasDataType {
    fn datatype(&self, level: usize) -> &str;
}

pub trait TypeCheck {
    fn valid(&self) -> Result<(), String>;
}

impl TypeCheck for DataType {
    fn valid(&self) -> Result<(), String> {
        // Split the type hierarchy and check all levels are valid
        let type_levels = self.split('/');

        for type_level in type_levels {
            if !DATATYPES.contains(&type_level) {
                return Err(format!("Type '{}' is invalid", &self));
            }
        }
        return Ok(());
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
