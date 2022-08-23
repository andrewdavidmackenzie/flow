use std::fmt;

use error_chain::bail;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use shrinkwraprs::Shrinkwrap;

use crate::errors::*;

/// Json "object" data type
pub const OBJECT_TYPE: &str = "object";

/// Json "string" data type
pub const STRING_TYPE: &str = "string";

/// Json "number" data type
pub const NUMBER_TYPE: &str = "number";

/// Json "boolean" data type
pub const BOOLEAN_TYPE: &str = "boolean";

/// Json "array" data type
pub const ARRAY_TYPE: &str = "array";

/// Json "null" data type
pub const NULL_TYPE: &str = "null";

const DATA_TYPES: &[&str] = &[OBJECT_TYPE, STRING_TYPE, NUMBER_TYPE, BOOLEAN_TYPE, ARRAY_TYPE, NULL_TYPE];

/// Datatype is just a string defining what data type is being used
#[derive(Shrinkwrap, Hash, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Ord, PartialOrd)]
pub struct DataType(String);

impl From<&str> for DataType {
    fn from(s: &str) -> Self {
        DataType(s.to_string())
    }
}

impl From<String> for DataType {
    fn from(s: String) -> Self {
        DataType(s)
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A set of datatypes
pub struct DataTypeList(Vec<DataType>);

impl fmt::Display for DataTypeList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[")?;
        for dt in &self.0 {
            write!(f, "{}, ", dt.0)?;
        }
        write!(f, "]")
    }
}

/// Trait that is used on multiple objects to get their data type
pub trait HasDataTypes {
    /// Return a reference to the datatype of the object implementing this trait
    fn datatypes(&self) -> &Vec<DataType>;
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
        self.starts_with(ARRAY_TYPE)
    }

    /// Return true if this datatype is generic (not specified at compile time and can contain
    /// any other datatype) or not
    pub fn is_generic(&self) -> bool {
        self == &DataType::from(OBJECT_TYPE)
    }

    /// Determine if this data type is an array of the `second` type
    pub fn array_of(&self, second: &Self) -> bool {
        &DataType::from(format!("{}/{}", ARRAY_TYPE, second).as_str()) == self
    }

    /// Get the data type the array holds
    pub fn within_array(&self) -> Result<DataType> {
        self.strip_prefix(&format!("{}/", ARRAY_TYPE)).map(DataType::from).ok_or_else(||
            {
                Error::from("DataType is not an array of Types")
            })
    }

    /// Take a json data value and return the type string for it, recursively
    /// going down when the type is a container type (array or object)
    pub fn type_string(value: &Value) -> String {
        match value {
            Value::String(_) => STRING_TYPE.into(),
            Value::Bool(_) => BOOLEAN_TYPE.into(),
            Value::Number(_) => NUMBER_TYPE.into(),
            Value::Array(array) => format!("{}/{}",
                                           ARRAY_TYPE, Self::type_string(&array[0])),
            Value::Object(map) => {
                if let Some(map_entry) = map.values().next() {
                    format!("{}/{}", OBJECT_TYPE, Self::type_string(map_entry))
                } else {
                    OBJECT_TYPE.to_owned()
                }
            }
            Value::Null => NULL_TYPE.into(),
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
    use crate::model::datatype::{ARRAY_TYPE, OBJECT_TYPE, STRING_TYPE};

    use super::DataType;

    #[test]
    fn valid_data_string_type() {
        let string_type = DataType::from(STRING_TYPE);
        string_type
            .valid()
            .expect("'string' DataType should be valid");
    }

    #[test]
    fn valid_data_json_type() {
        let json_type = DataType::from(OBJECT_TYPE);
        json_type.valid().expect("'object' DataType should be valid");
    }

    #[test]
    fn invalid_data_type() {
        let string_type = DataType::from("foo");
        assert!(string_type.valid().is_err());
    }

    #[test]
    fn is_array_true() {
        let array_type = DataType::from(ARRAY_TYPE);
        assert!(array_type.is_array());
    }

    #[test]
    fn is_array_false() {
        let string_type = DataType::from(STRING_TYPE);
        assert!(!string_type.is_array());
    }
}
