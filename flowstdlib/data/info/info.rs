use flow_macro::flow_function;
use serde_json::{json, Value};

fn type_string(value: &Value) -> String {
    match value {
        Value::String(_) => "String".into(),
        Value::Bool(_) => "Boolean".into(),
        Value::Number(_) => "Number".into(),
        Value::Array(array) => format!("Array/{}", type_string(&array[0])),
        Value::Object(map) => {
            if let Some(value) = &map.values().next().cloned() {
                format!("Map/{}", type_string(value))
            } else {
                "Map/Unknown".into()
            }
        }
        Value::Null => "Null".into(),
    }
}

#[flow_function]
fn _info(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let mut output_map = serde_json::Map::new();

    let (rows, cols) = match &inputs[0] {
        Value::String(string) => (1, string.len()),
        Value::Bool(_boolean) => (1, 1),
        Value::Number(_number) => (1, 1),
        Value::Array(vec) => match &vec[0] {
            Value::Array(row) => (vec.len(), row.len()), // Array of Arrays
            _ => (1, vec.len()),
        },
        Value::Object(map) => (map.len(), 2),
        Value::Null => (0, 0),
    };

    output_map.insert("rows".into(), json!(rows));
    output_map.insert("columns".into(), json!(cols));
    output_map.insert("type".into(), json!(type_string(&inputs[0])));

    Ok((Some(Value::Object(output_map)), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::{Map, Value};
    use serde_json::json;

    use super::_info;

    #[test]
    fn info_on_number() {
        let inputs = vec![json!(1)];
        let (result, _) = _info(&inputs).expect("_info() failed");
        let output_map = result.expect("Could not get the Value from the output");

        assert_eq!(output_map.pointer("/type").expect("Could not get the /type from the output"), &json!("Number"));
        assert_eq!(output_map.pointer("/rows").expect("Could not get the /rows from the output"), &json!(1));
        assert_eq!(output_map.pointer("/columns").expect("Could not get the /columns from the output"), &json!(1));
    }

    #[test]
    fn info_on_boolean() {
        let inputs = vec![json!(true)];
        let (result, _) = _info(&inputs).expect("_info() failed");
        let output_map = result.expect("Could not get the Value from the output");

        assert_eq!(output_map.pointer("/type").expect("Could not get the /type from the output"), &json!("Boolean"));
        assert_eq!(output_map.pointer("/rows").expect("Could not get the /row from the output"), &json!(1));
        assert_eq!(output_map.pointer("/columns").expect("Could not get the /columns from the output"), &json!(1));
    }

    #[test]
    fn info_on_string() {
        let string = json!("Hello");
        let (result, _) = _info(&[string]).expect("_info() failed");
        let output_map = result.expect("Could not get the Value from the output");

        assert_eq!(output_map.pointer("/type").expect("Could not get the /type from the output"), &json!("String"));
        assert_eq!(output_map.pointer("/rows").expect("Could not get the /row from the output"), &json!(1));
        assert_eq!(output_map.pointer("/columns").expect("Could not get the /columns from the output"), &json!(5));
    }

    #[test]
    fn info_on_null() {
        let inputs = vec![Value::Null];
        let (result, _) = _info(&inputs).expect("_info() failed");
        let output_map = result.expect("Could not get the Value from the output");

        assert_eq!(output_map.pointer("/type").expect("Could not get the /type from the output"), &json!("Null"));
        assert_eq!(output_map.pointer("/rows").expect("Could not get the /rows from the output"), &json!(0));
        assert_eq!(output_map.pointer("/columns").expect("Could not get the /column from the output"), &json!(0));
    }

    #[test]
    fn info_on_array_of_number() {
        let inputs = vec![json!([1, 2, 3])];
        let (result, _) = _info(&inputs).expect("_info() failed");
        let output_map = result.expect("Could not get the Value from the output");

        assert_eq!(output_map.pointer("/type").expect("Could not get the /type from the output"), &json!("Array/Number"));
        assert_eq!(output_map.pointer("/rows").expect("Could not get the /rows from the output"), &json!(1));
        assert_eq!(output_map.pointer("/columns").expect("Could not get the /column from the output"), &json!(3));
    }

    #[test]
    fn info_on_array_of_array_of_number() {
        let array_array_numbers = json!([[1, 2, 3], [4, 5, 6]]);
        let (result, _) = _info(&[array_array_numbers]).expect("_info() failed");
        let output_map = result.expect("Could not get the Value from the output");

        assert_eq!(
            output_map.pointer("/type").expect("Could not get the /type from the output"),
            &json!("Array/Array/Number")
        );
        assert_eq!(output_map.pointer("/rows").expect("Could not get the /rows from the output"), &json!(2));
        assert_eq!(output_map.pointer("/columns").expect("Could not get the /columns from the output"), &json!(3));
    }

    #[test]
    fn info_on_map_of_number() {
        let mut map = Map::new();
        map.insert("0".into(), json!(1));
        map.insert("1".into(), json!(2));
        let inputs = vec![Value::Object(map)];
        let (result, _) = _info(&inputs).expect("_info() failed");
        let output_map = result.expect("Could not get the Value from the output");

        assert_eq!(output_map.pointer("/type").expect("Could not get the /type from the output"), &json!("Map/Number"));
        assert_eq!(output_map.pointer("/rows").expect("Could not get the /rows from the output"), &json!(2));
        assert_eq!(output_map.pointer("/columns").expect("Could not get the /columns from the output"), &json!(2));
    }

    #[test]
    fn info_on_map_of_arrays_of_number() {
        let mut map = Map::new();
        map.insert("0".into(), json!([1, 2]));
        map.insert("1".into(), json!([3, 4]));
        let inputs = vec![Value::Object(map)];
        let (result, _) = _info(&inputs).expect("_info() failed");
        let output_map = result.expect("Could not get the Value from the output");

        assert_eq!(
            output_map.pointer("/type").expect("Could not get the /type from the output"),
            &json!("Map/Array/Number")
        );
        assert_eq!(output_map.pointer("/rows").expect("Could not get the /rows from the output"), &json!(2));
        assert_eq!(output_map.pointer("/columns").expect("Could not get the /columns from the output"), &json!(2));
    }
}
