use std::fmt;

use error_chain::bail;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use shrinkwraprs::Shrinkwrap;

use crate::errors::*;

const DATA_TYPES: &[&str] = &["String", "Value", "Number", "Bool", "Map", "Array", "Null"];

/// Datatype is just a string defining what data type is being used
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

/// Trait that is used on multiple objects to get their data type
pub trait HasDataType {
    /// Return a reference to the datatype of the object implementing this trait
    fn datatype(&self) -> &DataType;
}

impl DataType {
    /// Determine if a datatype specified in a flow is a valid datatype or not
    pub fn valid(&self) -> Result<()> {
        // Split the type hierarchy and check all levels are valid
        let type_levels = self.split('/');

        for type_level in type_levels {
            if !DATA_TYPES.contains(&type_level) {
                bail!("Type '{}' is invalid", &self);
            }
        }
        Ok(())
    }

    /// Return if this datatype is an array or not
    pub fn is_array(&self) -> bool {
        self.starts_with("Array")
    }

    /// Return true if this datatype is generic (not specified at compile time and can contain
    /// any other datatype) or not
    pub fn is_generic(&self) -> bool {
        self == &DataType::from("Value")
    }

    /// Determine if this data type is an array of the other
    pub fn array_of(&self, second: &Self) -> bool {
        &DataType::from(format!("Array/{}", second).as_str()) == self
    }

    /// Get the data type the array holds
    pub fn within_array(&self) -> Option<DataType> {
        self.strip_prefix("Array/").map(DataType::from)
    }

    /// Take a json data value and return the type string for it, recursively
    /// going down when the type is a container type (Array or Map(Object))
    pub fn type_string(value: &Value) -> String {
        match value {
            Value::String(_) => "String".into(),
            Value::Bool(_) => "Boolean".into(),
            Value::Number(_) => "Number".into(),
            Value::Array(array) => format!("Array/{}", Self::type_string(&array[0])),
            Value::Object(map) => {
                if let Some(map_entry) = map.values().next() {
                    format!("Map/{}", Self::type_string(map_entry))
                } else {
                    "Map".to_owned()
                }
            }
            Value::Null => "Null".into(),
        }
    }

    /// Take a string description of a DataType and determine how deeply nested in arrays it is
    pub fn array_order(&self) -> Result<i32> {
        if self.is_array() {
            let array_contents = self.within_array().ok_or("DataType is not an Array type")?;
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
        string_type
            .valid()
            .expect("'String' DataType should be valid");
    }

    #[test]
    fn valid_data_json_type() {
        let json_type = DataType::from("Value");
        json_type.valid().expect("'Value' DataType should be valid");
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
        assert!(!string_type.is_array());
    }
}
