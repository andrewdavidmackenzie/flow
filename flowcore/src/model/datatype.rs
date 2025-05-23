use std::fmt;

use error_chain::bail;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use std::marker::PhantomData;
use std::str::FromStr;

use serde::de;
use serde::de::Deserializer;
use crate::errors::{Error, Result};
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

/// `DataType` is just a String defining what data type is being used
#[derive(Hash, Debug, PartialEq, Ord, PartialOrd, Eq, Clone, Default, Serialize, Deserialize)]
pub struct DataType {
    string: String,
}

impl FromStr for DataType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(DataType {
            string: s.to_string()
        })
    }
}

/// A custom deserializer for a String or a Sequence of Strings for `DataTypes`
///
/// # Errors
///
/// Will return `Err` if deserialization fails
#[allow(clippy::module_name_repetitions)]
pub fn datatype_or_datatype_array<'de, D>(deserializer: D) -> std::result::Result<Vec<DataType>, D::Error>
    where
        D: Deserializer<'de>,
{
    struct StringOrVec(PhantomData<Vec<DataType>>);

    impl<'de> de::Visitor<'de> for StringOrVec {
        type Value = Vec<DataType>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("DataType or list of DataTypes")
        }

        fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
        {
            Ok(vec![DataType::from(value)])
        }

        fn visit_seq<S>(self, mut visitor: S) -> std::result::Result<Self::Value, S::Error>
            where
                S: de::SeqAccess<'de> {
            let mut vec: Vec<DataType> = Vec::new();

            while let Some(element) = visitor.next_element::<String>()? {
                vec.push(DataType::from(element));
            }

            Ok(vec)
        }
    }

    deserializer.deserialize_any(StringOrVec(PhantomData))
}

impl From<&str> for DataType {
    fn from(s: &str) -> Self {
        DataType {
            string: s.to_string()
        }
    }
}

impl From<String> for DataType {
    fn from(s: String) -> Self {
        DataType {
            string: s
        }
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.string)
    }
}

/// Trait that is used on multiple objects to get their data type
pub trait HasDataTypes {
    /// Return a reference to the datatype of the object implementing this trait
    fn datatypes(&self) -> &Vec<DataType>;
}

impl DataType {
    /// Determine if a datatype specified in a flow is a valid datatype or not
    ///
    /// # Errors
    ///
    /// Will return `Err` if `self.string` does not describe a valid `DataType`
    pub fn valid(&self) -> Result<()> {
        if self.string.is_empty() {  // generic type
            return Ok(());
        }

        // Split the type hierarchy and check all levels are valid
        let type_levels = self.string.split('/');

        for type_level in type_levels {
            if !DATA_TYPES.contains(&type_level) {
                bail!("Type '{}' is invalid", &self);
            }
        }
        Ok(())
    }

    /// Return if this datatype is an array or not
    #[must_use]
    pub fn is_array(&self) -> bool {
        self.string.starts_with(ARRAY_TYPE)
    }

    /// Return true if this datatype is generic (not specified at compile time and can contain
    /// any other datatype) or not
    #[must_use]
    pub fn is_generic(&self) -> bool {
        self.string.is_empty()
    }

    /// If an Array --> Return Option of the `DataType` the array holds
    /// If Generic  --> Return Generic
    /// If not an Array --> Return None
    pub fn array_type(&self) -> Option<DataType> {
        if self.is_generic() {
            Some(GENERIC_TYPE.into()) // We can only assume Generic for the contents of array
        } else if self.is_array() {
            self.string.strip_prefix(&format!("{ARRAY_TYPE}/")).map(DataType::from)
        } else {
            None
        }
    }

    /// Return the `DataType` for a Json `Value`, including nested values in arrays or maps
    pub(crate) fn value_type(value: &Value) -> Result<DataType> {
        match value {
            Value::String(_) => Ok(STRING_TYPE.into()),
            Value::Bool(_) => Ok(BOOLEAN_TYPE.into()),
            Value::Number(_) => Ok(NUMBER_TYPE.into()),
            Value::Array(array) => {
                if array.is_empty() {
                    Ok(DataType::from(format!("{ARRAY_TYPE}/{GENERIC_TYPE}")))
                } else {
                    Ok(DataType::from(format!("{ARRAY_TYPE}/{}",
                                              Self::value_type(array.first().ok_or("Could not get input")?)?)))
                }
            }
            Value::Object(map) => {
                if let Some(map_entry) = map.values().next() {
                    Ok(DataType::from(format!("{OBJECT_TYPE}/{}", Self::value_type(map_entry)?)))
                } else {
                    Ok(OBJECT_TYPE.into())
                }
            }
            Value::Null => Ok(NULL_TYPE.into()),
        }
    }

    /// Determine how deeply nested in arrays this `DataType` is.
    /// If not an array, returns 0
    ///
    /// This is used at compile time by the compiler to determine the array order
    /// of an input or output
    #[must_use]
    pub fn type_array_order(&self) -> i32 {
        // Avoid recursing into array type if Generic (see array_type above)
        if self.is_generic() {
            0
        } else if let Some(array_contents) = self.array_type() {
            array_contents.type_array_order() + 1
        } else {
            0
        }
    }

    /// Determine how deeply nested in arrays this `serde_json::value::Value` is.
    /// If not an array, returns 0
    ///
    /// This is used at run time to determine the array order of
    /// an actual value being processed
    #[must_use]
    pub fn value_array_order(value: &Value) -> i32 {
        match value {
            Value::Array(array) if !array.is_empty() => {
                if let Some(value) = array.first() {
                    1 + Self::value_array_order(value)
                } else {
                    1
                }
            }
            Value::Array(array) if array.is_empty() => 1,
            _ => 0,
        }
    }

    /// For a set of output types to be compatible with a destination's set of types
    /// ALL of the `output_types` must have a compatible input type, to guarantee that any
    /// of the valid types produced can be handled by the destination
    ///
    /// # Errors
    ///
    /// Will return `Err` if the `from` or `to` arrays of `DataType` are empty, or if the
    /// types of the two `IO` are incompatible
    pub fn compatible_types(from: &[DataType], to: &[DataType], from_subroute: &Route) -> Result<()> {
        if from.is_empty() || to.is_empty() {
            bail!("Either from or to IO does nopt specify any types")
        }

        for from_type in from {
            let mut compatible_destination_type = false;
            for to_type in to {
                let from_sub_type = Self::subtype_using_subroute(from_type, from_subroute)?;
                if Self::two_compatible_types(&from_sub_type, to_type).is_ok() {
                    compatible_destination_type = true;
                }
            }
            if !compatible_destination_type {
                bail!("Could not find a compatible destination type in '{:?}' for '{:?}'",
                from, to)
            }
        }

        Ok(()) // found a compatible source and destination type pair
    }

    fn subtype_using_subroute(full_type: &DataType, subroute: &Route) -> Result<DataType> {
        if subroute.depth() == 0 {
            return Ok(full_type.clone());
        }

        if full_type.is_generic() {
            return Ok(full_type.clone());
        }

        let mut full_type_split = full_type.string.split('/').collect::<Vec<&str>>();

        if subroute.depth() > full_type_split.len() {
            bail!("Depth of subroute '{}' is greater than the depth of the type '{}'",
                subroute, full_type)
        }

        // drop the first 'depth' segments in order to get the sub-type referred to by the route
        Ok(DataType::from(full_type_split.split_off(subroute.depth() - 1).join("/")))
    }

    /// Determine if a source type and a destination type are compatible, counting on
    /// serialization of arrays of types to types.
    fn two_compatible_types(from: &DataType, to: &DataType) -> Result<()> {
        // generic at compile time, can't assume it won't be compatible with the destination
        // Relies on serialization of an array of generics into an input of some type or
        // array of generics sent to array of some specific type (runtime conversion)
        if from.array_type() == Some(GENERIC_TYPE.into()) {
            return Ok(());
        }

        // destination can accept any type - with or without the runtime serializing
        if to.array_type() == Some(GENERIC_TYPE.into()) {
            return Ok(());
        }

        match from.type_array_order() - to.type_array_order() {
            0 => if from == to { // from and to types are the same - hence compatible
                return Ok(());
            },

            1 => return Self::two_compatible_types(&from.array_type().ok_or("From type is not an array!")?,
                                                   to),

            -1 => return Self::two_compatible_types(from,
                                                    &to.array_type().ok_or("To type is not an array!")?),

            2 => {
                let from = from.array_type().ok_or("From type is not an array!")?;
                let from = from.array_type().ok_or("From type is not an array!")?;
                return Self::two_compatible_types(&from, to);
            }

            -2 => {
                let to = to.array_type().ok_or("To type is not an array!")?;
                let to = to.array_type().ok_or("To type is not an array!")?;
                return Self::two_compatible_types(from, &to);
            }

            _ => bail!("Cannot encapsulate/serialize arrays with a order difference of more than two")
        }

        bail!("The types '{}' and '{}' are incompatible", from, to)
    }
}

#[cfg(test)]
mod test {
    use crate::model::datatype::{ARRAY_TYPE, GENERIC_TYPE, NUMBER_TYPE, OBJECT_TYPE, STRING_TYPE};
    use crate::model::route::Route;

    use super::DataType;

    #[test]
    fn subtype_empty_route() {
        let array_of_numbers_type = DataType::from(format!("{ARRAY_TYPE}/{NUMBER_TYPE}"));
        assert_eq!(DataType::subtype_using_subroute(&array_of_numbers_type,
                                                    &Route::from(""))
                       .expect("Could not get subtype"), array_of_numbers_type);
    }

    #[test]
    fn invalid_subtype_route() {
        assert!(DataType::subtype_using_subroute(&DataType::from(NUMBER_TYPE),
                                                 &Route::from("/1")).is_err());
    }

    #[test]
    fn subtype_of_generic() {
        assert!(DataType::subtype_using_subroute(&DataType::from(GENERIC_TYPE),
                                                 &Route::from("/1")).is_ok());
    }

    #[test]
    fn array_of_numbers_subtype() {
        let subtype = DataType::subtype_using_subroute(
            &DataType::from(format!("{ARRAY_TYPE}/{NUMBER_TYPE}")),
            &Route::from("/1"))
            .expect("Could not get subtype");
        assert_eq!(subtype, DataType::from(NUMBER_TYPE));
    }

    #[test]
    fn array_of_array_of_strings_subtype() {
        let subtype = DataType::subtype_using_subroute(
            &DataType::from(format!("{ARRAY_TYPE}/{STRING_TYPE}/{STRING_TYPE}")),
            &Route::from("/2/1"))
            .expect("Could not get subtype");
        assert_eq!(subtype, DataType::from(STRING_TYPE));
    }

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
        use serde_json::json;

        use crate::model::datatype::{ARRAY_TYPE, BOOLEAN_TYPE, DataType, GENERIC_TYPE, NULL_TYPE, NUMBER_TYPE, OBJECT_TYPE, STRING_TYPE};
        use crate::model::io::IO;
        use crate::model::route::Route;

        #[test]
        fn test_array_order_0() {
            let value = json!(1);
            assert_eq!(DataType::value_array_order(&value), 0);
        }

        #[test]
        fn test_array_order_1_empty_array() {
            let value = json!([]);
            assert_eq!(DataType::value_array_order(&value), 1);
        }

        #[test]
        fn test_array_order_1() {
            let value = json!([1, 2, 3]);
            assert_eq!(DataType::value_array_order(&value), 1);
        }

        #[test]
        fn test_array_order_2() {
            let value = json!([[1, 2, 3], [2, 3, 4]]);
            assert_eq!(DataType::value_array_order(&value), 2);
        }

        /// # Array serialization and {type} to array wrapping
        ///
        /// ## {type} being sent
        ///   Value Type   Input Type
        /// * {type}   --> {type} (`is_array` = false) - input and sent to the function as an array
        /// * {type}   --> array (`is_array` = true)   - {type} will be converted to a one element array
        ///
        /// ## array of {type} being sent
        ///   Value Type        Input Type
        /// * array/{type}  --> array (`is_array` = true)
        /// * array/{type}  --> object (`is_array` = false) - values in Array will be serialized
        ///   and sent to input one by one
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
                (format!("{ARRAY_TYPE}/{NUMBER_TYPE}"), GENERIC_TYPE.into(), ""),
                (format!("{ARRAY_TYPE}/{ARRAY_TYPE}/{NUMBER_TYPE}"), GENERIC_TYPE.into(), ""),

                // any type is compatible with a destination of array of same type (wrapping)
                (NUMBER_TYPE.into(), format!("{ARRAY_TYPE}/{NUMBER_TYPE}"), ""),
                (NULL_TYPE.into(), format!("{ARRAY_TYPE}/{NULL_TYPE}"), ""),
                (STRING_TYPE.into(), format!("{ARRAY_TYPE}/{STRING_TYPE}"), ""),
                (BOOLEAN_TYPE.into(), format!("{ARRAY_TYPE}/{BOOLEAN_TYPE}"), ""),

                // any type is compatible with a destination of array of generic (wrapping + generic)
                // runtime wraps object to array of type, destination can accept any type in array
                (NUMBER_TYPE.into(), format!("{ARRAY_TYPE}/{GENERIC_TYPE}"), ""),
                (NULL_TYPE.into(), format!("{ARRAY_TYPE}/{GENERIC_TYPE}"), ""),
                (STRING_TYPE.into(), format!("{ARRAY_TYPE}/{GENERIC_TYPE}"), ""),
                (BOOLEAN_TYPE.into(), format!("{ARRAY_TYPE}/{GENERIC_TYPE}"), ""),

                // an array of types can be serialized to a destination of same type (array serialization)
                (format!("{ARRAY_TYPE}/{OBJECT_TYPE}"), OBJECT_TYPE.into(), ""),
                (format!("{ARRAY_TYPE}/{NUMBER_TYPE}"), NUMBER_TYPE.into(), ""),
                (format!("{ARRAY_TYPE}/{NULL_TYPE}"), NULL_TYPE.into(), ""),
                (format!("{ARRAY_TYPE}/{STRING_TYPE}"), STRING_TYPE.into(), ""),
                (format!("{ARRAY_TYPE}/{BOOLEAN_TYPE}"), BOOLEAN_TYPE.into(), ""),

                // Selection from an array of same type
                (format!("{ARRAY_TYPE}/{NUMBER_TYPE}"), NUMBER_TYPE.into(), "/0"),
                (format!("{ARRAY_TYPE}/{NULL_TYPE}"), NULL_TYPE.into(), "/0"),
                (format!("{ARRAY_TYPE}/{ARRAY_TYPE}"), ARRAY_TYPE.into(), "/0"),
                (format!("{ARRAY_TYPE}/{STRING_TYPE}"), STRING_TYPE.into(), "/0"),
                (format!("{ARRAY_TYPE}/{BOOLEAN_TYPE}"), BOOLEAN_TYPE.into(), "/0"),
                (format!("{ARRAY_TYPE}/{OBJECT_TYPE}"), OBJECT_TYPE.into(), "/0"),

                // Selection from a generic that maybe an array at runtime
                (GENERIC_TYPE.into(), GENERIC_TYPE.into(), "/0"),

                // equality of first order arrays of types
                (format!("{ARRAY_TYPE}/{NUMBER_TYPE}"), format!("{ARRAY_TYPE}/{NUMBER_TYPE}"), ""),
                (format!("{ARRAY_TYPE}/{NULL_TYPE}"), format!("{ARRAY_TYPE}/{NULL_TYPE}"), ""),
                (format!("{ARRAY_TYPE}/{STRING_TYPE}"), format!("{ARRAY_TYPE}/{STRING_TYPE}"), ""),
                (format!("{ARRAY_TYPE}/{BOOLEAN_TYPE}"), format!("{ARRAY_TYPE}/{BOOLEAN_TYPE}"), ""),
                (format!("{ARRAY_TYPE}/{OBJECT_TYPE}"), format!("{ARRAY_TYPE}/{OBJECT_TYPE}"), ""),

                // equality of second order arrays to second order arrays of same type
                (format!("{ARRAY_TYPE}/{ARRAY_TYPE}/{NUMBER_TYPE}"), format!("{ARRAY_TYPE}/{ARRAY_TYPE}/{NUMBER_TYPE}"), ""),
                (format!("{ARRAY_TYPE}/{ARRAY_TYPE}/{NULL_TYPE}"), format!("{ARRAY_TYPE}/{ARRAY_TYPE}/{NULL_TYPE}"), ""),
                (format!("{ARRAY_TYPE}/{ARRAY_TYPE}/{STRING_TYPE}"), format!("{ARRAY_TYPE}/{ARRAY_TYPE}/{STRING_TYPE}"), ""),
                (format!("{ARRAY_TYPE}/{ARRAY_TYPE}/{BOOLEAN_TYPE}"), format!("{ARRAY_TYPE}/{ARRAY_TYPE}/{BOOLEAN_TYPE}"), ""),
                (format!("{ARRAY_TYPE}/{ARRAY_TYPE}/{OBJECT_TYPE}"), format!("{ARRAY_TYPE}/{ARRAY_TYPE}/{OBJECT_TYPE}"), ""),

                // Selection of second order arrays to first order arrays of same type
                (format!("{ARRAY_TYPE}/{ARRAY_TYPE}/{NUMBER_TYPE}"), format!("{ARRAY_TYPE}/{NUMBER_TYPE}"), "/0"),
                (format!("{ARRAY_TYPE}/{ARRAY_TYPE}/{NULL_TYPE}"), format!("{ARRAY_TYPE}/{NULL_TYPE}"), "/0"),
                (format!("{ARRAY_TYPE}/{ARRAY_TYPE}/{STRING_TYPE}"), format!("{ARRAY_TYPE}/{STRING_TYPE}"), "/0"),
                (format!("{ARRAY_TYPE}/{ARRAY_TYPE}/{BOOLEAN_TYPE}"), format!("{ARRAY_TYPE}/{BOOLEAN_TYPE}"), "/0"),
                (format!("{ARRAY_TYPE}/{ARRAY_TYPE}/{OBJECT_TYPE}"), format!("{ARRAY_TYPE}/{OBJECT_TYPE}"), "/0"),

                // serialization of second order arrays to first order arrays of same type
                (format!("{ARRAY_TYPE}/{ARRAY_TYPE}/{NUMBER_TYPE}"), format!("{ARRAY_TYPE}/{NUMBER_TYPE}"), ""),
                (format!("{ARRAY_TYPE}/{ARRAY_TYPE}/{NULL_TYPE}"), format!("{ARRAY_TYPE}/{NULL_TYPE}"), ""),
                (format!("{ARRAY_TYPE}/{ARRAY_TYPE}/{STRING_TYPE}"), format!("{ARRAY_TYPE}/{STRING_TYPE}"), ""),
                (format!("{ARRAY_TYPE}/{ARRAY_TYPE}/{BOOLEAN_TYPE}"), format!("{ARRAY_TYPE}/{BOOLEAN_TYPE}"), ""),
                (format!("{ARRAY_TYPE}/{ARRAY_TYPE}/{OBJECT_TYPE}"), format!("{ARRAY_TYPE}/{OBJECT_TYPE}"), ""),

                // serialization of second order array of generic to first order arrays of same type
                (format!("{ARRAY_TYPE}/{ARRAY_TYPE}/{GENERIC_TYPE}"), format!("{ARRAY_TYPE}/{NUMBER_TYPE}"), ""),
                (format!("{ARRAY_TYPE}/{GENERIC_TYPE}"), format!("{ARRAY_TYPE}/{ARRAY_TYPE}/{NUMBER_TYPE}"), ""),
                (format!("{ARRAY_TYPE}/{GENERIC_TYPE}"), format!("{ARRAY_TYPE}/{NUMBER_TYPE}"), ""),
                (format!("{ARRAY_TYPE}/{GENERIC_TYPE}"), format!("{ARRAY_TYPE}/{NUMBER_TYPE}"), "/1"),
                (ARRAY_TYPE.into(), ARRAY_TYPE.into(), ""),
                (ARRAY_TYPE.into(), GENERIC_TYPE.into(), ""),
                (ARRAY_TYPE.into(), format!("{ARRAY_TYPE}/{ARRAY_TYPE}"), ""),
                (ARRAY_TYPE.into(), format!("{ARRAY_TYPE}/{GENERIC_TYPE}"), ""),

                // Maps (Objects) of type with a selector
                (format!("{OBJECT_TYPE}/{NUMBER_TYPE}"), NUMBER_TYPE.into(), "/name"),
            ];

            for (case_number, test) in valid_type_conversions.iter().enumerate() {
                if DataType::compatible_types(
                    &[DataType::from(&test.0 as &str)],
                    &[DataType::from(&test.1 as &str)],
                    &Route::from(test.2)).is_err() {
                    panic!("Test Case #{case_number} failed");
                }
            }
        }

        #[test]
        fn invalid_type_conversions() {
            let invalid_type_conversions: Vec<(String, String, &str)> = vec![
                // object source is only compatible with object destination or array of
                (OBJECT_TYPE.into(), NUMBER_TYPE.into(), ""), // cannot convert object to number
                (OBJECT_TYPE.into(), NULL_TYPE.into(), ""), // cannot convert object to null
                (OBJECT_TYPE.into(), ARRAY_TYPE.into(), ""), // cannot convert object to array
                (OBJECT_TYPE.into(), STRING_TYPE.into(), ""), // cannot convert object to string
                (OBJECT_TYPE.into(), BOOLEAN_TYPE.into(), ""), // cannot convert object to boolean

                // selecting from an array not allowed on non-array types
                (NUMBER_TYPE.into(), NUMBER_TYPE.into(), "/0"), // cannot select from a non-array
                (OBJECT_TYPE.into(), NUMBER_TYPE.into(), "/0"), // cannot select from a non-array
                (NULL_TYPE.into(), NUMBER_TYPE.into(), "/0"), // cannot select from a non-array
                (STRING_TYPE.into(), NUMBER_TYPE.into(), "/0"), // cannot select from a non-array
                (BOOLEAN_TYPE.into(), NUMBER_TYPE.into(), "/0"), // cannot select from a non-array

                // selecting a type from an array to send to an incompatible input
                (format!("{ARRAY_TYPE}/{NUMBER_TYPE}"), STRING_TYPE.into(), "/0"),
                (format!("{ARRAY_TYPE}/{NULL_TYPE}"), STRING_TYPE.into(), "/0"),
                (format!("{ARRAY_TYPE}/{STRING_TYPE}"), NUMBER_TYPE.into(), "/0"),
                (format!("{ARRAY_TYPE}/{BOOLEAN_TYPE}"), OBJECT_TYPE.into(), "/0"),
                (format!("{ARRAY_TYPE}/{OBJECT_TYPE}"), BOOLEAN_TYPE.into(), "/0"),

                // Invalid object contents
                (format!("{OBJECT_TYPE}/{NUMBER_TYPE}"), STRING_TYPE.into(), "/name"),
                (format!("{OBJECT_TYPE}/{STRING_TYPE}"), STRING_TYPE.into(), ""),
            ];

            for test in invalid_type_conversions {
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
                &Route::default(),
            ).is_ok());
        }

        #[test]
        fn simple_indexed_to_simple() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output/0");
            let to_io = IO::new(vec!(STRING_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default(),
            ).is_ok());
        }

        #[test]
        fn simple_to_simple_mismatch() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(NUMBER_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default(),
            ).is_err());
        }

        #[test]
        fn simple_indexed_to_array() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output/0");
            let to_io = IO::new(vec!("array/string".into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default(),
            ).is_ok());
        }

        #[test]
        fn simple_to_array() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!("array/string".into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default(),
            ).is_ok());
        }

        #[test]
        fn simple_to_array_mismatch() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!("array/number".into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default(),
            ).is_err());
        }

        #[test]
        fn array_to_array() {
            let from_io = IO::new(vec!(ARRAY_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(ARRAY_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default(),
            ).is_ok());
        }

        #[test]
        fn array_to_simple() {
            let from_io = IO::new(vec!("array/string".into()), "/p1/output");
            let to_io = IO::new(vec!(STRING_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default(),
            ).is_ok());
        }

        #[test]
        fn multiple_output_type_to_single_input_type() {
            let from_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(STRING_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default(),
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
                &Route::default(),
            ).is_ok());
        }

        #[test]
        fn multiple_output_type_to_matching_input_types() {
            let from_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default(),
            ).is_ok());
        }

        #[test]
        fn single_output_type_to_superset_input_types() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default(),
            ).is_ok());
        }

        #[test]
        fn multiple_output_type_to_superset_input_types() {
            let from_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into(), ARRAY_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default(),
            ).is_ok());
        }

        #[test]
        fn multiple_output_type_to_non_matching_input_types() {
            let from_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(STRING_TYPE.into(), ARRAY_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default(),
            ).is_err());
        }

        #[test]
        fn single_output_type_to_non_matching_input_types() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(ARRAY_TYPE.into(), NUMBER_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default(),
            ).is_err());
        }

        #[test]
        fn multiple_output_type_to_generic_input_types() {
            let from_io = IO::new(vec!(STRING_TYPE.into(), NUMBER_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(ARRAY_TYPE.into(), GENERIC_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default(),
            ).is_ok());
        }

        #[test]
        fn single_output_type_to_generic_input_types() {
            let from_io = IO::new(vec!(STRING_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(ARRAY_TYPE.into(), GENERIC_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default(),
            ).is_ok());
        }

        #[test]
        fn null_output_type_to_valid_input_types() {
            let from_io = IO::new(vec!(), "/p1/output");
            let to_io = IO::new(vec!(OBJECT_TYPE.into()), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default(),
            ).is_err());
        }

        #[test]
        fn valid_output_type_to_null_input_types() {
            let from_io = IO::new(vec!(OBJECT_TYPE.into()), "/p1/output");
            let to_io = IO::new(vec!(), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default(),
            ).is_err());
        }

        #[test]
        fn null_output_type_to_null_input_types() {
            let from_io = IO::new(vec!(), "/p1/output");
            let to_io = IO::new(vec!(), "/p2");
            assert!(DataType::compatible_types(
                from_io.datatypes(),
                to_io.datatypes(),
                &Route::default(),
            ).is_err());
        }
    }
}
