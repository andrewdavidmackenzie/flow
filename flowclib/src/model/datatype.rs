const DATATYPES: &'static [&'static str] = &["String", "Json", "Number", "Bool", "Map", "Array"];

pub type DataType = String;

pub trait HasDataType {
    fn datatype(&self, level: usize) -> DataType;
}

pub trait TypeCheck {
    fn valid(&self) -> Result<(), String>;
    fn is_array(&self) -> bool;
    fn is_generic(&self) -> bool;
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

    fn is_array(&self) -> bool {
        self == &DataType::from("Array")
    }

    fn is_generic(&self) -> bool {
        self == &DataType::from("Json")
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

#[test]
fn is_array_true() {
    let array_type = DataType::from("Array".to_string());
    assert!(array_type.is_array());
}

#[test]
fn is_array_false() {
    let string_type = DataType::from("String".to_string());
    assert_eq!(string_type.is_array(), false);
}