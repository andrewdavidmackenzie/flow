use flowmacro::flow_function;
use serde_json::{json, Value};

#[flow_function]
fn _accumulate(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let value = inputs[0].clone(); // input value to accumulate in array
    let mut output_map = serde_json::Map::new();

    if value.is_null() {
        output_map.insert("chunk".into(), Value::Null);
    } else {
        let mut partial_input = inputs[1].clone(); // A partial array to append the values to
                                                   // how many elements desired in the output array
        let chunk_size = inputs[2].as_u64().ok_or("Could not get chunk_size")?;

        let partial = partial_input.as_array_mut().ok_or("Could not get partial")?;
        partial.push(value);

        if partial.len() >= chunk_size as usize {
            // TODO could pass on any extra elements beyond chunk size in 'partial'
            // and also force chunk size to be exact....
            output_map.insert("chunk".into(), Value::Array(partial.clone()));
            output_map.insert("partial".into(), Value::Array(vec![]));
        } else {
            output_map.insert("partial".into(), Value::Array(partial.clone()));
        }

        output_map.insert("chunk_size".into(), json!(chunk_size));
    }

    Ok((Some(Value::Object(output_map)), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::_accumulate;

    #[test]
    fn accumulate_start_and_finish() {
        let value = json!(1);
        let partial = json!([]);
        let chunk_size = json!(1);

        let (result, _) = _accumulate(&[value, partial, chunk_size]).expect("_accumulate() failed");
        let output = result.expect("Could not get the Value from the output");
        assert_eq!(output.pointer("/chunk").expect("Could not get the /chunk from the output"), &json!([1]));
    }

    #[test]
    fn accumulate_start_not_finish() {
        let value = json!(1);
        let partial = json!([]);
        let chunk_size = json!(2);

        let (result, _) = _accumulate(&[value, partial, chunk_size]).expect("_accumulate() failed");
        let output = result.expect("Could not get the Value from the output");
        assert_eq!(output.pointer("/chunk"), None);
        assert_eq!(output.pointer("/partial").expect("Could not get the /partial from the output"), &json!([1]));
    }

    #[test]
    fn accumulate_started_then_finish() {
        let value = json!(2);
        let partial = json!([1]);
        let chunk_size = json!(2);

        let (result, _) = _accumulate(&[value, partial, chunk_size]).expect("_accumulate() failed");
        let output = result.expect("Could not get the Value from the output");
        assert_eq!(output.pointer("/chunk").expect("Could not get the /chunk from the output"), &json!([1, 2]));
    }
}
