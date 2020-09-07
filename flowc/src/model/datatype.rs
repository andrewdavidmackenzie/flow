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
    fn datatype(&self) -> &DataType;
}

impl DataType {
    pub fn valid(&self) -> Result<()> {
        // Split the type hierarchy and check all levels are valid
        let type_levels = self.split('/');

        for type_level in type_levels {
            if !DATATYPES.contains(&type_level) {
                bail!("Type '{}' is invalid", &self);
            }
        }
        Ok(())
    }

    pub fn is_array(&self) -> bool {
        self.starts_with("Array")
    }

    pub fn is_generic(&self) -> bool {
        self == &DataType::from("Value")
    }

    /// Determine if this data type is an array of the other
    pub fn array_of(&self, second: &Self) -> bool {
        &DataType::from(format!("Array/{}", second).as_str()) == self
    }

    /// Get the data type the array holds
    pub fn within_array(&self) -> Result<Self> {
        let mut subtype = self.to_string();
        if !subtype.starts_with("Array/") {
            bail!("Datatype is not an Array");
        }
        // Could do this with a slice of Self, without creating new string
        subtype.replace_range(0.."Array/".len(), "");
        Ok(Self::from(subtype.as_str()))
    }

    /// Take a json data value and return the type string for it, recursively
    /// going down when the type is a container type (Array or Map(Object))
    pub fn type_string(value: &Value) -> String {
        match value {
            Value::String(_) => "String".into(),
            Value::Bool(_) => "Boolean".into(),
            Value::Number(_) => "Number".into(),
            Value::Array(array) => format!("Array/{}", Self::type_string(&array[0])),
            Value::Object(map) => format!("Map/{}", Self::type_string(&map.values().cloned().next().unwrap())),
            Value::Null => "Null".into()
        }
    }

    /// Take a string description of a DataType and determine how deeply nested in arrays it is
    pub fn array_order(&self) -> Result<i32> {
        if self.is_array() {
            let array_contents = self.within_array()?;
            let sub_order = array_contents.array_order()?;
            Ok(1 + sub_order)
        } else {
            Ok(0)
        }
    }
}

#[cfg(test)]
mod test {
    use super::DataType;

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
    fn invalid_data_type() {
        let string_type = DataType::from("foo");
        assert!(string_type.valid().is_err());
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
}