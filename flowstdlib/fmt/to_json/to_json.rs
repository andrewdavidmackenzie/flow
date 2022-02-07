use serde_json::Value;

use flow_macro::flow_function;

#[flow_function]
fn _to_json(inputs: &[Value]) -> (Option<Value>, RunAgain) {
    let input = &inputs[0];

    if input.is_null() {
        (Some(Value::Null), RUN_AGAIN)
    } else if input.is_string() {
        match input.as_str() {
            Some(string) => match serde_json::from_str(string) {
                Ok(json) => (Some(json), RUN_AGAIN),
                Err(_) => (
                    Some(serde_json::Value::String(string.to_string())),
                    RUN_AGAIN,
                ),
            },
            None => (None, RUN_AGAIN),
        }
    } else {
        (Some(input.clone()), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use serde_json::{json, Value};
    use super::_to_json;

    fn test_to_json(string: &str, expected_value: Value) {
        let inputs = vec![json!(string)];
        let (result, _) = _to_json(&inputs);

        match result {
            Some(value) => {
                assert_eq!(value, expected_value);
            }
            None => panic!("No Result returned"),
        }
    }

    #[test]
    fn parse_string() {
        test_to_json("\"Hello World\"", json!("Hello World"));
    }

    #[test]
    fn parse_number() {
        test_to_json("42", json!(42));
    }

    #[test]
    fn parse_null() {
        test_to_json("null", serde_json::Value::Null);
    }

    #[test]
    fn parse_array() {
        test_to_json("[1,2,3,4]", json!([1, 2, 3, 4]));
    }

    // Can't be parsed directly into json so return String
    #[test]
    fn parse_invalid() {
        test_to_json("-1.20,0.35", json!("-1.20,0.35"));
    }

    #[test]
    fn parse_map() {
        let mut map: HashMap<&str, u32> = HashMap::new();
        map.insert("Meaning", 42);
        map.insert("Size", 9);
        test_to_json("{\"Meaning\":42,\"Size\":9}", json!(map));
    }
}
