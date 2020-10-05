use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::{json, Value};

#[derive(FlowImpl)]
/// Output info about the input value
#[derive(Debug)]
pub struct Info;

fn type_string(value: &Value) -> String {
    match value {
        Value::String(_) => "String".into(),
        Value::Bool(_) => "Boolean".into(),
        Value::Number(_) => "Number".into(),
        Value::Array(array) => format!("Array/{}", type_string(&array[0])),
        Value::Object(map) => format!("Map/{}", type_string(&map.values().cloned().next().unwrap())),
        Value::Null => "Null".into()
    }
}

impl Implementation for Info {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let mut output_map = serde_json::Map::new();

        let (rows, cols) = match &inputs[0] {
            Value::String(string) => (1, string.len()),
            Value::Bool(_boolean) => (1, 1),
            Value::Number(_number) => (1, 1),
            Value::Array(vec) => match &vec[0] {
                Value::Array(row) => (vec.len(), row.len()), // Array of Arrays
                _ => (1, vec.len())
            },
            Value::Object(map) => (map.len(), 2),
            Value::Null => (0, 0)
        };

        output_map.insert("rows".into(), json!(rows));
        output_map.insert("columns".into(), json!(cols));
        output_map.insert("type".into(), json!(type_string(& inputs[0])));

        (Some(Value::Object(output_map)), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use flow_impl::Implementation;
    use serde_json::{Map, Value};
    use serde_json::json;

    #[test]
    fn info_on_number() {
        let inputs = vec!(json!(1));
        let info = super::Info {};
        let (result, _) = info.run(&inputs);
        let output_map = result.unwrap();

        assert_eq!(output_map.pointer("/type").unwrap(), &json!("Number"));
        assert_eq!(output_map.pointer("/rows").unwrap(), &json!(1));
        assert_eq!(output_map.pointer("/columns").unwrap(), &json!(1));
    }

    #[test]
    fn info_on_boolean() {
        let inputs = vec!(json!(true));
        let info = super::Info {};
        let (result, _) = info.run(&inputs);
        let output_map = result.unwrap();

        assert_eq!(output_map.pointer("/type").unwrap(), &json!("Boolean"));
        assert_eq!(output_map.pointer("/rows").unwrap(), &json!(1));
        assert_eq!(output_map.pointer("/columns").unwrap(), &json!(1));
    }

    #[test]
    fn info_on_string() {
        let string = json!("Hello");
        let info = super::Info {};
        let (result, _) = info.run(&[string]);
        let output_map = result.unwrap();

        assert_eq!(output_map.pointer("/type").unwrap(), &json!("String"));
        assert_eq!(output_map.pointer("/rows").unwrap(), &json!(1));
        assert_eq!(output_map.pointer("/columns").unwrap(), &json!(5));
    }

    #[test]
    fn info_on_null() {
        let inputs = vec!(Value::Null);
        let info = super::Info {};
        let (result, _) = info.run(&inputs);
        let output_map = result.unwrap();

        assert_eq!(output_map.pointer("/type").unwrap(), &json!("Null"));
        assert_eq!(output_map.pointer("/rows").unwrap(), &json!(0));
        assert_eq!(output_map.pointer("/columns").unwrap(), &json!(0));
    }

    #[test]
    fn info_on_array_of_number() {
        let inputs = vec!(json!([1, 2, 3]));
        let info = super::Info {};
        let (result, _) = info.run(&inputs);
        let output_map = result.unwrap();

        assert_eq!(output_map.pointer("/type").unwrap(), &json!("Array/Number"));
        assert_eq!(output_map.pointer("/rows").unwrap(), &json!(1));
        assert_eq!(output_map.pointer("/columns").unwrap(), &json!(3));
    }

    #[test]
    fn info_on_array_of_array_of_number() {
        let array_array_numbers = json!([ [1, 2, 3], [4, 5, 6] ]);
        let info = super::Info {};
        let (result, _) = info.run(&[array_array_numbers]);
        let output_map = result.unwrap();

        assert_eq!(output_map.pointer("/type").unwrap(), &json!("Array/Array/Number"));
        assert_eq!(output_map.pointer("/rows").unwrap(), &json!(2));
        assert_eq!(output_map.pointer("/columns").unwrap(), &json!(3));
    }

    #[test]
    fn info_on_map_of_number() {
        let mut map = Map::new();
        map.insert("0".into(), json!(1));
        map.insert("1".into(), json!(2));
        let inputs = vec!(Value::Object(map));
        let info = super::Info {};
        let (result, _) = info.run(&inputs);
        let output_map = result.unwrap();

        assert_eq!(output_map.pointer("/type").unwrap(), &json!("Map/Number"));
        assert_eq!(output_map.pointer("/rows").unwrap(), &json!(2));
        assert_eq!(output_map.pointer("/columns").unwrap(), &json!(2));
    }

    #[test]
    fn info_on_map_of_arrays_of_number() {
        let mut map = Map::new();
        map.insert("0".into(), json!([1, 2]));
        map.insert("1".into(), json!([3, 4]));
        let inputs = vec!(Value::Object(map));
        let info = super::Info {};
        let (result, _) = info.run(&inputs);
        let output_map = result.unwrap();

        assert_eq!(output_map.pointer("/type").unwrap(), &json!("Map/Array/Number"));
        assert_eq!(output_map.pointer("/rows").unwrap(), &json!(2));
        assert_eq!(output_map.pointer("/columns").unwrap(), &json!(2));
    }
}