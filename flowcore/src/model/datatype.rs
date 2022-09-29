use std::fmt;

use error_chain::bail;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use shrinkwraprs::Shrinkwrap;

use crate::errors::*;
use crate::model::route::Route;

/// Generic type is represented as an empty string
pub const GENERIC_TYPE: &str = "";

/// Json "object" data type (a Map in other languages)
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

const DATA_TYPES: &[&str] = &[OBJECT_TYPE, STRING_TYPE, NUMBER_TYPE, BOOLEAN_TYPE, ARRAY_TYPE,
    NULL_TYPE, GENERIC_TYPE];

/// Datatype is just a string defining what data type is being used
#[derive(Shrinkwrap, Hash, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Ord, PartialOrd)]
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

impl fmt::Debug for DataType {
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
        if self.is_empty() {  // generic type
            return Ok(());
        }

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
        self.is_empty()
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
                    OBJECT_TYPE.into()
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


    /// For a set of output types to be compatible with a destination's set of types
    /// ALL of the output_types must have a compatible input type, to guarantee that any
    /// of the valid types produced can be handled by the destination
    pub fn compatible_types(from: &[DataType], to: &[DataType], from_subroute: &Route) -> Result<()> {
        if from.is_empty() || to.is_empty() {
            bail!("Either from or to IO does nopt specify any types")
        }

        for output_type in from {
            let mut compatible_destination_type = false;
            for input_type in to {
                if Self::two_compatible_types(output_type, input_type, from_subroute) {
                    compatible_destination_type = true;
                }
            }
            if !compatible_destination_type {
                bail!("Could not find a compatible destination type in '{:?}' for '{:?}'",
                from, to)
            }
        }

        Ok(()) // all output_types found a compatible destination type
    }

    /// Determine if a source type and a destination type are compatible, counting on
    /// serialization of arrays of types to types.
    fn two_compatible_types(from: &DataType, to: &DataType, from_subroute: &Route) -> bool {
        // TODO get the real datatype using `from` DataType and `from_subroute`

        // from and too types are the same - hence compatible
        if from == to && from_subroute.is_empty() {
            return true;
        }

        // TODO make this invalid, this is when we have a gate process that accepts a number and
        // passes a number at runtime, but the definition doesn't know the input type and so can
        // only state the output type as generic fix with #1187
        if from.is_generic() && from_subroute.is_empty() {
            return true;
        }

        // destination can accept any type - with or without the runtime serializing the from objects
        if to.is_generic() || to.array_of(&DataType::from("")) {
            return true;
        }

        if to.array_of(from) {
            return true;
        }

        // to select an element from an array source, it must be an array
        if from_subroute.is_array_selector() && !from.is_array() {
            return false;
        }

        // the source is an array of the destination type - runtime can serialize the elements
        if from.array_of(to) {
            return true;
        }

        // Relies on serialization of an array of generics into an input of some type
        // TODO remove when we implement pass-through of types through generic i/os
        if from.array_of(&DataType::from(GENERIC_TYPE)) && !to.is_array() {
            return true;
        }

        // Relies on array of generics into an input of array of some type
        // hence relies on generic to something working
        // TODO remove when we implement pass-through of types through generic i/os
        if from.array_of(&DataType::from(GENERIC_TYPE)) && to.is_array() {
            return true;
        }

        false
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


    mod type_conversion {
        use crate::model::datatype::{ARRAY_TYPE, BOOLEAN_TYPE, DataType, GENERIC_TYPE, NULL_TYPE, NUMBER_TYPE, OBJECT_TYPE, STRING_TYPE};
        use crate::model::io::IO;
        use crate::model::route::Route;

        /// # Array serialization and {type} to array wrapping
        ///
        /// ## {type} being sent
        ///   Value Type   Input Type
        /// * {type}   --> {type} (is_array = false) - input and sent to the function as an array
        /// * {type}   --> array (is_array = true)   - {type} will be converted to a one element array
        ///
        /// ## array of {type} being sent
        ///   Value Type        Input Type
        /// * array/{type}  --> array (is_array = true)
        /// * array/{type}  --> object (is_array = false) - values in Array will be serialized
        ///                     and sent to input one by one)

        #[test]
        fn valid_type_conversions() {
            let valid_type_conversions: Vec<(String, String, &str)> = vec![
                // equal types are compatible (equality)
                (OBJECT_TYPE.into(), OBJECT_TYPE.into(), ""),
                (NUMBER_TYPE.into(), NUMBER_TYPE.into(), ""),
                (NULL_TYPE.into(), NULL_TYPE.into(), ""),
                (STRING_TYPE.into(), STRING_TYPE.into(), ""),
                (BOOLEAN_TYPE.into(), BOOLEAN_TYPE.into(), ""),

                // any type is compatible with a generic destination (generic)
                (NUMBER_TYPE.into(), GENERIC_TYPE.into(), ""),
                (NULL_TYPE.into(), GENERIC_TYPE.into(), ""),
                (STRING_TYPE.into(), GENERIC_TYPE.into(), ""),
                (BOOLEAN_TYPE.into(), GENERIC_TYPE.into(), ""),
                (format!("{}/{}", ARRAY_TYPE, NUMBER_TYPE), GENERIC_TYPE.into(), ""),
                (format!("{}/{}/{}", ARRAY_TYPE, ARRAY_TYPE, NUMBER_TYPE), GENERIC_TYPE.into(), ""),

                // any type is compatible with a destination of array of same type (wrapping)
                (NUMBER_TYPE.into(), format!("{}/{}", ARRAY_TYPE, NUMBER_TYPE), ""),
                (NULL_TYPE.into(), format!("{}/{}", ARRAY_TYPE, NULL_TYPE), ""),
                (STRING_TYPE.into(), format!("{}/{}", ARRAY_TYPE, STRING_TYPE), ""),
                (BOOLEAN_TYPE.into(), format!("{}/{}", ARRAY_TYPE, BOOLEAN_TYPE), ""),

                // any type is compatible with a destination of array of generic (wrapping + generic)
                // runtime wraps object to array of type, destination can accept any type in array
                (NUMBER_TYPE.into(), format!("{}/{}", ARRAY_TYPE, GENERIC_TYPE), ""),
                (NULL_TYPE.into(), format!("{}/{}", ARRAY_TYPE, GENERIC_TYPE), ""),
                (STRING_TYPE.into(), format!("{}/{}", ARRAY_TYPE, GENERIC_TYPE), ""),
                (BOOLEAN_TYPE.into(), format!("{}/{}", ARRAY_TYPE, GENERIC_TYPE), ""),

                // an array of types can be serialized to a destination of same type (array serialization)
                (format!("{}/{}", ARRAY_TYPE, OBJECT_TYPE), OBJECT_TYPE.into(), ""),
                (format!("{}/{}", ARRAY_TYPE, NUMBER_TYPE), NUMBER_TYPE.into(), ""),
                (format!("{}/{}", ARRAY_TYPE, NULL_TYPE), NULL_TYPE.into(), ""),
                (format!("{}/{}", ARRAY_TYPE, STRING_TYPE), STRING_TYPE.into(), ""),
                (format!("{}/{}", ARRAY_TYPE, BOOLEAN_TYPE), BOOLEAN_TYPE.into(), ""),

                // A type can be selected from an array of same type (array selection)
                (format!("{}/{}", ARRAY_TYPE, NUMBER_TYPE), NUMBER_TYPE.into(), "/0"),
                (format!("{}/{}", ARRAY_TYPE, NULL_TYPE), NULL_TYPE.into(), "/0"),
                (format!("{}/{}", ARRAY_TYPE, ARRAY_TYPE), ARRAY_TYPE.into(), "/0"),
                (format!("{}/{}", ARRAY_TYPE, STRING_TYPE), STRING_TYPE.into(), "/0"),
                (format!("{}/{}", ARRAY_TYPE, BOOLEAN_TYPE), BOOLEAN_TYPE.into(), "/0"),
                (format!("{}/{}", ARRAY_TYPE, OBJECT_TYPE), OBJECT_TYPE.into(), "/0"),

                // equality of first order arrays of types
                (format!("{}/{}", ARRAY_TYPE, NUMBER_TYPE), format!("{}/{}", ARRAY_TYPE, NUMBER_TYPE), ""),
                (format!("{}/{}", ARRAY_TYPE, NULL_TYPE), format!("{}/{}", ARRAY_TYPE, NULL_TYPE), ""),
                (format!("{}/{}", ARRAY_TYPE, STRING_TYPE), format!("{}/{}", ARRAY_TYPE, STRING_TYPE), ""),
                (format!("{}/{}", ARRAY_TYPE, BOOLEAN_TYPE), format!("{}/{}", ARRAY_TYPE, BOOLEAN_TYPE), ""),
                (format!("{}/{}", ARRAY_TYPE, OBJECT_TYPE), format!("{}/{}", ARRAY_TYPE, OBJECT_TYPE), ""),


                // serialization of second order arrays to first order arrays of same type
                (format!("{}/{}/{}", ARRAY_TYPE, ARRAY_TYPE, NUMBER_TYPE), format!("{}/{}", ARRAY_TYPE, NUMBER_TYPE), ""),
                (format!("{}/{}/{}", ARRAY_TYPE, ARRAY_TYPE, NULL_TYPE), format!("{}/{}", ARRAY_TYPE, NULL_TYPE), ""),
                (format!("{}/{}/{}", ARRAY_TYPE, ARRAY_TYPE, STRING_TYPE), format!("{}/{}", ARRAY_TYPE, STRING_TYPE), ""),
                (format!("{}/{}/{}", ARRAY_TYPE, ARRAY_TYPE, BOOLEAN_TYPE), format!("{}/{}", ARRAY_TYPE, BOOLEAN_TYPE), ""),
                (format!("{}/{}/{}", ARRAY_TYPE, ARRAY_TYPE, OBJECT_TYPE), format!("{}/{}", ARRAY_TYPE, OBJECT_TYPE), ""),

                // TODO maybe make illegal all those Null types that don't make much sense?

                // TODO make invalid - cannot guarantee that an object can convert to array of number
                (format!("{}/{}", ARRAY_TYPE, GENERIC_TYPE), format!("{}/{}/{}", ARRAY_TYPE, ARRAY_TYPE, NUMBER_TYPE), ""),

                // TODO make invalid - object cannot be guaranteed to convert to number
                (format!("{}/{}", ARRAY_TYPE, GENERIC_TYPE), format!("{}/{}", ARRAY_TYPE, NUMBER_TYPE), ""),
                // TODO make invalid - not it's used to get a generic object from get/json/1 and pass it as a number to another input
                (format!("{}/{}", ARRAY_TYPE, GENERIC_TYPE), format!("{}/{}", ARRAY_TYPE, NUMBER_TYPE), "/1"),
                // TODO make invalid - array should need subtype
                (ARRAY_TYPE.into(), ARRAY_TYPE.into(), ""),
                // TODO make invalid - array should need subtype
                (ARRAY_TYPE.into(), GENERIC_TYPE.into(), ""),
                // TODO make invalid - array should need subtype
                (ARRAY_TYPE.into(), format!("{}/{}", ARRAY_TYPE, ARRAY_TYPE), ""),
                (ARRAY_TYPE.into(), format!("{}/{}", ARRAY_TYPE, GENERIC_TYPE), ""),
            ];

            for test in valid_type_conversions.iter() {
                assert!(DataType::compatible_types(
                    &[DataType::from(&test.0 as &str)],
                    &[DataType::from(&test.1 as &str)],
                    &Route::from(test.2)).is_ok());
            }
        }

        #[test]
        fn invalid_type_conversions() {
            let invalid_type_conversions: Vec<(String, String, &str)> = vec![
            // object source is only compatible with object destination or array of
            (OBJECT_TYPE.into(), NUMBER_TYPE.into(), ""  ), // cannot convert object to number
            (OBJECT_TYPE.into(), NULL_TYPE.into(), ""  ), // cannot convert object to null
            (OBJECT_TYPE.into(), ARRAY_TYPE.into(), ""  ), // cannot convert object to array
            (OBJECT_TYPE.into(), STRING_TYPE.into(), ""  ), // cannot convert object to string
            (OBJECT_TYPE.into(), BOOLEAN_TYPE.into(), ""  ), // cannot convert object to boolean

            // selecting from an array not allowed on non-array types
            (NUMBER_TYPE.into(), NUMBER_TYPE.into(), "/0"), // cannot select from a non-array
            (OBJECT_TYPE.into(), NUMBER_TYPE.into(), "/0"), // cannot select from a non-array
            (NULL_TYPE.into(), NUMBER_TYPE.into(), "/0"), // cannot select from a non-array
            (STRING_TYPE.into(), NUMBER_TYPE.into(), "/0"), // cannot select from a non-array
            (BOOLEAN_TYPE.into(), NUMBER_TYPE.into(), "/0"), // cannot select from a non-array
            ];

            for test in invalid_type_conversions.iter() {
                assert!(DataType::compatible_types(
                    &[DataType::from(&test.0 as &str)],
                    &[DataType::from(&test.1 as &str)],
                    &Route::from(test.2)).is_err());
            }
        }

        #[test]
        fn simple_to_simple() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(STRING_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ).is_ok());
        }

        #[test]
        fn simple_indexed_to_simple() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output/0");
            let to_io = IO::new(vec!(STRING_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ).is_ok());
        }

        #[test]
        fn simple_to_simple_mismatch() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(NUMBER_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ).is_err());
        }

        #[test]
        fn simple_indexed_to_array() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output/0");
            let to_io = IO::new(vec!("array/string".into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ).is_ok());
        }

        #[test]
        fn simple_to_array() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!("array/string".into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ).is_ok());
        }

        #[test]
        fn simple_to_array_mismatch() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!("array/number".into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ).is_err());
        }

        #[test]
        fn array_to_array() {
            let from_io = IO::new(vec!(ARRAY_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(ARRAY_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ).is_ok());
        }

        #[test]
        fn array_to_simple() {
            let from_io = IO::new(vec!("array/string".into()), "/p1/output");
            let to_io = IO::new(vec!(STRING_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ).is_ok());
        }

        #[test]
        fn multiple_output_type_to_single_input_type() {
            let from_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(STRING_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ).is_err());
        }

        #[test]
        fn multiple_output_type_to_generic_input_type() {
            let from_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into()),
                                  "/p1/output");
            let to_io = IO::new(vec!(GENERIC_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ).is_ok());
        }

        #[test]
        fn multiple_output_type_to_matching_input_types() {
            let from_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ).is_ok());
        }

        #[test]
        fn single_output_type_to_superset_input_types() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ).is_ok());
        }

        #[test]
        fn multiple_output_type_to_superset_input_types() {
            let from_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into(), ARRAY_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ).is_ok());
        }

        #[test]
        fn multiple_output_type_to_non_matching_input_types() {
            let from_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(STRING_TYPE.into(), ARRAY_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ).is_err());
        }

        #[test]
        fn single_output_type_to_non_matching_input_types() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(ARRAY_TYPE.into(), NUMBER_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ).is_err());
        }

        #[test]
        fn multiple_output_type_to_generic_input_types() {
            let from_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(ARRAY_TYPE.into(), GENERIC_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ).is_ok());
        }

        #[test]
        fn single_output_type_to_generic_input_types() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(ARRAY_TYPE.into(), GENERIC_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ).is_ok());
        }

        #[test]
        fn null_output_type_to_valid_input_types() {
            let from_io = IO::new(vec!(), "/p1/output");
            let to_io = IO::new(vec!(OBJECT_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ).is_err());
        }

        #[test]
        fn valid_output_type_to_null_input_types() {
            let from_io = IO::new(vec!(OBJECT_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ).is_err());
        }

        #[test]
        fn null_output_type_to_null_input_types() {
            let from_io = IO::new(vec!(), "/p1/output");
            let to_io = IO::new(vec!(), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default()
            ).is_err());
        }
    }
}
