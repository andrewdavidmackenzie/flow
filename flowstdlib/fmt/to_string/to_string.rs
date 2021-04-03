use serde_json::{json, Value};

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RUN_AGAIN, RunAgain};

#[derive(FlowImpl)]
/// Convert an input type to a String
#[derive(Debug)]
pub struct ToString;

// The data to convert to a String. Current types supported are:
// * Null - A null will be printed as "Null"
// * Bool - Boolean JSON value
// * Number - A JSON Number
// * String - a bit redundant, but it works
// * Array - An JSON array of values that can be converted, they are converted one by one
// * Object - a Map of names/objects that will also be printed out
impl Implementation for ToString {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let input = &inputs[0];
        (Some(json!(input.to_string())), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use serde_json::{json, Value};

    use super::Implementation;
    use super::ToString;

    fn test_to_string(value: Value, string: &str) {
        let to_string = ToString {};
        let inputs = vec![value];
        let (result, _) = to_string.run(&inputs);

        match result {
            Some(value) => {
                assert_eq!(value.as_str(), Some(string));
            }
            None => panic!("No Result returned"),
        }
    }

    #[test]
    fn test_null_input() {
        test_to_string(serde_json::Value::Null, "null");
    }

    #[test]
    fn test_string_input() {
        test_to_string(json!("Hello World"), "\"Hello World\"");
    }

    #[test]
    fn test_bool_input() {
        test_to_string(json!(true), "true");
        test_to_string(json!(false), "false");
    }

    #[test]
    fn test_number_input() {
        test_to_string(json!(42), "42");
    }

    #[test]
    fn test_array_input() {
        test_to_string(json!([1, 2, 3, 4]), "[1,2,3,4]");
    }

    #[test]
    fn test_map_input() {
        let mut map: HashMap<&str, u32> = HashMap::new();
        map.insert("Meaning", 42);
        map.insert("Size", 9);
        test_to_string(json!(map), "{\"Meaning\":42,\"Size\":9}");
    }
}
