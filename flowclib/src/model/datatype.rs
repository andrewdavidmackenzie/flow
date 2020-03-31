use std::fmt;

use error_chain::bail;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use shrinkwraprs::Shrinkwrap;

use crate::errors::*;

const DATATYPES: &[&str] = &["String", "Value", "Number", "Bool", "Map", "Array", "Null"];

#[derive(Shrinkwrap, Hash, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DataType(String);

impl From<&str> for DataType {
    fn from(s: &str) -> Self {
        DataType(s.to_string())
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub trait HasDataType {
    fn datatype(&self, level: usize) -> DataType;
}

pub trait TypeCheck {
    fn valid(&self) -> Result<()>;
    fn is_array(&self) -> bool;
    fn is_generic(&self) -> bool;
}

/// Take a json data value and return the type string for it, recursively
/// going down when the type is a container type (Array or Map(Object))
pub fn type_string(value: &Value) -> String {
    match value {
        Value::String(_) => "String".into(),
        Value::Bool(_) => "Boolean".into(),
        Value::Number(_) => "Number".into(),
        Value::Array(array) => format!("Array/{}", type_string(&array[0])),
        Value::Object(map) => format!("Map/{}", type_string(&map.values().cloned().next().unwrap())),
        Value::Null => "Null".into()
    }
}

impl TypeCheck for DataType {
    fn valid(&self) -> Result<()> {
        // Split the type hierarchy and check all levels are valid
        let type_levels = self.split('/');

        for type_level in type_levels {
            if !DATATYPES.contains(&type_level) {
                bail!("Type '{}' is invalid", &self);
            }
        }
        return Ok(());
    }

    fn is_array(&self) -> bool {
        self == &DataType::from("Array")
    }

    fn is_generic(&self) -> bool {
        self == &DataType::from("Value")
    }
}

#[test]
fn valid_data_string_type() {
    let string_type = DataType::from("String");
    string_type.valid().unwrap();
}

#[test]
fn valid_data_json_type() {
    let json_type = DataType::from("Value");
    json_type.valid().unwrap();
}

#[test]
#[should_panic]
fn invalid_data_type() {
    let string_type = DataType::from("foo");
    string_type.valid().unwrap();
}

#[test]
fn is_array_true() {
    let array_type = DataType::from("Array");
    assert!(array_type.is_array());
}

#[test]
fn is_array_false() {
    let string_type = DataType::from("String");
    assert_eq!(string_type.is_array(), false);
}