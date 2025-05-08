use serde_json::{json, Value};

use flowcore::{RUN_AGAIN, RunAgain};
use flowcore::errors::Result;
use flowmacro::flow_function;

#[flow_function]
fn inner_index(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let value = inputs.first().ok_or("Could not get value")?.clone();

    let mut output_map = serde_json::Map::new();

    if let Some(previous_index) = inputs.get(2).ok_or("Could not get previous_index")?.as_i64() {
        let index = previous_index + 1;

        // Always output the 'value" and its index
        output_map.insert("index".into(), json!(index));

        if let Some(select_index) = inputs.get(3).ok_or("COuld not get selected_index")?.as_i64() {
            match select_index {
                // A 'select_index' value of -1 indicates to output the last value before the null
                -1 if value.is_null() => {
                    let _ = output_map.insert("selected_value".into(), inputs.get(1)
                        .ok_or("COuld not get selected_value")?.clone());
                }
                // If 'select_value' is not -1 then see if it matches the current index
                _ if select_index == index => {
                    let _ = output_map.insert("selected_value".into(), value);
                }
                _ => {}
            }
        }
    }

    Ok((Some(Value::Object(output_map)), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::{json, Value};

    use super::inner_index;

    #[test]
    fn select_index_0() {
        let value = json!(42);
        let previous_value = Value::Null;
        let previous_index = json!(-1);
        let select_index = json!(0);

        let inputs = vec![value, previous_value, previous_index, select_index];

        let (result, _) = inner_index(&inputs).expect("_index() failed");

        let output_map = result.expect("No output map");

        assert_eq!(
            output_map
                .pointer("/selected_value")
                .expect("Could not select route"),
            &json!(42)
        );
    }

    #[test]
    fn not_select_index_0() {
        let value = json!(42);
        let previous_value = Value::Null;
        let previous_index = json!(1);
        let select_index = json!(1);

        let inputs = vec![value, previous_value, previous_index, select_index];

        let (result, _) = inner_index(&inputs).expect("_index() failed");

        let output_map = result.expect("No output map");

        assert_eq!(output_map.pointer("/selected_value"), None);
    }

    #[test]
    fn select_index_1() {
        let value = json!(42);
        let previous_value = Value::Null;
        let previous_index = json!(0);
        let select_index = json!(1);

        let inputs = vec![value, previous_value, previous_index, select_index];

        let (result, _) = inner_index(&inputs).expect("_index() failed");

        let output_map = result.expect("No output map");

        assert_eq!(
            output_map
                .pointer("/selected_value")
                .expect("Could not select route"),
            &json!(42)
        );
    }

    #[test]
    fn not_select_index_1() {
        let value = json!(42);
        let previous_value = Value::Null;
        let previous_index = json!(1);
        let select_index = json!(0);

        let inputs = vec![value, previous_value, previous_index, select_index];

        let (result, _) = inner_index(&inputs).expect("_index() failed");

        let output_map = result.expect("No output map");

        assert_eq!(output_map.pointer("/selected_value"), None);
    }

    #[test]
    fn select_last() {
        let value = Value::Null;
        let previous_value = json!(42);
        let previous_index = json!(7);
        let select_index = json!(-1);

        let inputs = vec![value, previous_value, previous_index, select_index];

        let (result, _) = inner_index(&inputs).expect("_index() failed");

        let output_map = result.expect("No output map");

        assert_eq!(
            output_map
                .pointer("/selected_value")
                .expect("Could not select route"),
            &json!(42)
        );
    }

    #[test]
    fn not_select_last() {
        let value = json!(43);
        let previous_value = json!(42);
        let previous_index = json!(7);
        let select_index = json!(-1);

        let inputs = vec![value, previous_value, previous_index, select_index];

        let (result, _) = inner_index(&inputs).expect("_index() failed");

        let output_map = result.expect("No output map");

        assert_eq!(output_map.pointer("/selected_value"), None);
    }
}
