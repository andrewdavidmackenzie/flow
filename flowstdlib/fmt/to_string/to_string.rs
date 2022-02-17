use flowmacro::flow_function;
use serde_json::{json, Value};

#[flow_function]
fn _to_string(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let input = &inputs[0];
    Ok((Some(json!(input.to_string())), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::{json, Value};
    use std::collections::HashMap;

    use super::_to_string;

    fn test_to_string(value: Value, string: &str) {
        let inputs = vec![value];
        let (result, _) = _to_string(&inputs).expect("_to_string() failed");

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
