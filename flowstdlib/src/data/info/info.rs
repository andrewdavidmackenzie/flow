use serde_json::{json, Value};

use flowcore::{RUN_AGAIN, RunAgain};
use flowcore::errors::Result;
use flowcore::model::datatype::{ARRAY_TYPE, BOOLEAN_TYPE, NULL_TYPE, NUMBER_TYPE, OBJECT_TYPE, STRING_TYPE};
use flowmacro::flow_function;

fn type_string(value: &Value) -> Result<String> {
    match value {
        Value::String(_) => Ok(STRING_TYPE.into()),
        Value::Bool(_) => Ok(BOOLEAN_TYPE.into()),
        Value::Number(_) => Ok(NUMBER_TYPE.into()),
        Value::Array(array) => Ok(format!("{ARRAY_TYPE}/{}", type_string(array.first().ok_or("Could not get array")?)?)),
        Value::Object(map) => {
            if let Some(value) = &map.values().next().cloned() {
                Ok(format!("{OBJECT_TYPE}/{}", type_string(value)?))
            } else {
                Ok(format!("{OBJECT_TYPE}/Unknown"))
            }
        }
        Value::Null => Ok(NULL_TYPE.into()),
    }
}

#[flow_function]
fn inner_info(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let mut output_map = serde_json::Map::new();

    let (rows, cols) = match inputs.first().ok_or("Could not get rows & cols")? {
        Value::String(string) => (1, string.len()),
        Value::Bool(_boolean) => (1, 1),
        Value::Number(_number) => (1, 1),
        Value::Array(vec) => match vec.first().ok_or("Could not get array")? {
            Value::Array(row) => (vec.len(), row.len()), // Array of Arrays
            _ => (1, vec.len()),
        },
        Value::Object(map) => (map.len(), 2),
        Value::Null => (0, 0),
    };

    let data_type = type_string(inputs.first().ok_or("Could not get type")?)?;
    output_map.insert("rows".into(), json!(rows));
    output_map.insert("columns".into(), json!(cols));
    output_map.insert("type".into(), json!(data_type));

    Ok((Some(Value::Object(output_map)), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::{Map, Value};
    use serde_json::json;

    use flowcore::model::datatype::{NUMBER_TYPE, OBJECT_TYPE};
    use flowcore::model::datatype::ARRAY_TYPE;
    use flowcore::model::datatype::BOOLEAN_TYPE;
    use flowcore::model::datatype::NULL_TYPE;
    use flowcore::model::datatype::STRING_TYPE;

    use super::inner_info;

    #[test]
    fn info_on_number() {
        let inputs = vec![json!(1)];
        let (result, _) = inner_info(&inputs).expect("_info() failed");
        let output_map = result.expect("Could not get the Value from the output");

        assert_eq!(output_map.pointer("/type").expect("Could not get the /type from the output"), &json!(NUMBER_TYPE));
        assert_eq!(output_map.pointer("/rows").expect("Could not get the /rows from the output"), &json!(1));
        assert_eq!(output_map.pointer("/columns").expect("Could not get the /columns from the output"), &json!(1));
    }

    #[test]
    fn info_on_boolean() {
        let inputs = vec![json!(true)];
        let (result, _) = inner_info(&inputs).expect("_info() failed");
        let output_map = result.expect("Could not get the Value from the output");

        assert_eq!(output_map.pointer("/type").expect("Could not get the /type from the output"), &json!(BOOLEAN_TYPE));
        assert_eq!(output_map.pointer("/rows").expect("Could not get the /row from the output"), &json!(1));
        assert_eq!(output_map.pointer("/columns").expect("Could not get the /columns from the output"), &json!(1));
    }

    #[test]
    fn info_on_string() {
        let string = json!("Hello");
        let (result, _) = inner_info(&[string]).expect("_info() failed");
        let output_map = result.expect("Could not get the Value from the output");

        assert_eq!(output_map.pointer("/type").expect("Could not get the /type from the output"), &json!(STRING_TYPE));
        assert_eq!(output_map.pointer("/rows").expect("Could not get the /row from the output"), &json!(1));
        assert_eq!(output_map.pointer("/columns").expect("Could not get the /columns from the output"), &json!(5));
    }

    #[test]
    fn info_on_null() {
        let inputs = vec![Value::Null];
        let (result, _) = inner_info(&inputs).expect("_info() failed");
        let output_map = result.expect("Could not get the Value from the output");

        assert_eq!(output_map.pointer("/rows").expect("Could not get the /rows from the output"), &json!(0));
        assert_eq!(output_map.pointer("/type").expect("Could not get the /type from the output"), &json!(NULL_TYPE));
        assert_eq!(output_map.pointer("/columns").expect("Could not get the /column from the output"), &json!(0));
    }

    #[test]
    fn info_on_array_of_number() {
        let inputs = vec![json!([1, 2, 3])];
        let (result, _) = inner_info(&inputs).expect("_info() failed");
        let output_map = result.expect("Could not get the Value from the output");

        assert_eq!(output_map.pointer("/type").expect("Could not get the /type from the output"),
                   &json!(format!("{ARRAY_TYPE}/{NUMBER_TYPE}")));
        assert_eq!(output_map.pointer("/rows").expect("Could not get the /rows from the output"), &json!(1));
        assert_eq!(output_map.pointer("/columns").expect("Could not get the /column from the output"), &json!(3));
    }

    #[test]
    fn info_on_array_of_array_of_number() {
        let array_array_numbers = json!([[1, 2, 3], [4, 5, 6]]);
        let (result, _) = inner_info(&[array_array_numbers]).expect("_info() failed");
        let output_map = result.expect("Could not get the Value from the output");

        assert_eq!(
            output_map.pointer("/type").expect("Could not get the /type from the output"),
            &json!(format!("{ARRAY_TYPE}/{ARRAY_TYPE}/{NUMBER_TYPE}"))
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
        let (result, _) = inner_info(&inputs).expect("_info() failed");
        let output_map = result.expect("Could not get the Value from the output");

        assert_eq!(output_map.pointer("/type").expect("Could not get the /type from the output"),
                   &json!(format!("{OBJECT_TYPE}/{NUMBER_TYPE}")));
        assert_eq!(output_map.pointer("/rows").expect("Could not get the /rows from the output"), &json!(2));
        assert_eq!(output_map.pointer("/columns").expect("Could not get the /columns from the output"), &json!(2));
    }

    #[test]
    fn info_on_map_of_arrays_of_number() {
        let mut map = Map::new();
        map.insert("0".into(), json!([1, 2]));
        map.insert("1".into(), json!([3, 4]));
        let inputs = vec![Value::Object(map)];
        let (result, _) = inner_info(&inputs).expect("_info() failed");
        let output_map = result.expect("Could not get the Value from the output");

        assert_eq!(
            output_map.pointer("/type").expect("Could not get the /type from the output"),
            &json!(format!("{OBJECT_TYPE}/{ARRAY_TYPE}/{NUMBER_TYPE}"))
        );
        assert_eq!(output_map.pointer("/rows").expect("Could not get the /rows from the output"), &json!(2));
        assert_eq!(output_map.pointer("/columns").expect("Could not get the /columns from the output"), &json!(2));
    }
}
