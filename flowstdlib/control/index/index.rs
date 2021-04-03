use serde_json::{json, Value};

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};

#[derive(FlowImpl)]
/// Pass thru a value based on the index of an item in the stream of values
#[derive(Debug)]
pub struct Index;

impl Implementation for Index {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let value = inputs[0].clone();
        let previous_value = inputs[1].clone();
        let previous_index = inputs[2].as_i64().unwrap();
        let select_index = inputs[3].as_i64().unwrap();

        let mut output_map = serde_json::Map::new();

        let index = previous_index + 1;

        match select_index {
            // A 'select_index' value of -1 indicates to output the last value before the null
            -1 if value.is_null() => {
                output_map.insert("selected_value".into(), json!(previous_value))
            }
            // If 'select_value' is not -1 then see if it matches the current index
            _ if select_index == index => output_map.insert("selected_value".into(), json!(value)),
            _ => None,
        };

        // Output the 'value" and its index
        output_map.insert("value".into(), json!(value));
        output_map.insert("index".into(), json!(index));

        (Some(Value::Object(output_map)), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use serde_json::{json, Value};

    use flowcore::Implementation;

    use super::Index;

    #[test]
    fn select_index_0() {
        let indexor = Index {};

        let value = json!(42);
        let previous_value = Value::Null;
        let previous_index = json!(-1);
        let select_index = json!(0);

        let inputs = vec![value, previous_value, previous_index, select_index];

        let (result, _) = indexor.run(&inputs);

        let output_map = result.unwrap();

        assert_eq!(output_map.pointer("/selected_value").unwrap(), &json!(42));
    }

    #[test]
    fn not_select_index_0() {
        let indexor = Index {};

        let value = json!(42);
        let previous_value = Value::Null;
        let previous_index = json!(1);
        let select_index = json!(1);

        let inputs = vec![value, previous_value, previous_index, select_index];

        let (result, _) = indexor.run(&inputs);

        let output_map = result.unwrap();

        assert_eq!(output_map.pointer("/selected_value"), None);
    }

    #[test]
    fn select_index_1() {
        let indexor = Index {};

        let value = json!(42);
        let previous_value = Value::Null;
        let previous_index = json!(0);
        let select_index = json!(1);

        let inputs = vec![value, previous_value, previous_index, select_index];

        let (result, _) = indexor.run(&inputs);

        let output_map = result.unwrap();

        assert_eq!(output_map.pointer("/selected_value").unwrap(), &json!(42));
    }

    #[test]
    fn not_select_index_1() {
        let indexor = Index {};

        let value = json!(42);
        let previous_value = Value::Null;
        let previous_index = json!(1);
        let select_index = json!(0);

        let inputs = vec![value, previous_value, previous_index, select_index];

        let (result, _) = indexor.run(&inputs);

        let output_map = result.unwrap();

        assert_eq!(output_map.pointer("/selected_value"), None);
    }

    #[test]
    fn select_last() {
        let indexor = Index {};

        let value = Value::Null;
        let previous_value = json!(42);
        let previous_index = json!(7);
        let select_index = json!(-1);

        let inputs = vec![value, previous_value, previous_index, select_index];

        let (result, _) = indexor.run(&inputs);

        let output_map = result.unwrap();

        assert_eq!(output_map.pointer("/selected_value").unwrap(), &json!(42));
    }

    #[test]
    fn not_select_last() {
        let indexor = Index {};

        let value = json!(43);
        let previous_value = json!(42);
        let previous_index = json!(7);
        let select_index = json!(-1);

        let inputs = vec![value, previous_value, previous_index, select_index];

        let (result, _) = indexor.run(&inputs);

        let output_map = result.unwrap();

        assert_eq!(output_map.pointer("/selected_value"), None);
    }
}
