use serde_json::{json, Value};

use flowcore::errors::Result;
use flowcore::{RunAgain, RUN_AGAIN};
use flowmacro::flow_function;

#[flow_function]
fn inner_accumulate(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let value = inputs.first().ok_or("Could not get value")?.clone(); // input value to accumulate in array
    let mut output_map = serde_json::Map::new();

    if value.is_null() {
        let partial_input = inputs.get(1).ok_or("Could not get partial_input")?.clone();
        let partial = partial_input.as_array().ok_or("Could not get partial")?;
        if partial.is_empty() {
            output_map.insert("chunk".into(), Value::Null);
        } else {
            output_map.insert("chunk".into(), Value::Array(partial.clone()));
        }
    } else {
        let mut partial_input = inputs.get(1).ok_or("Could not get partial_input")?.clone();
        let chunk_size = inputs
            .get(2)
            .ok_or("Could not get chunk_size")?
            .as_u64()
            .filter(|&s| s > 0);

        let partial = partial_input
            .as_array_mut()
            .ok_or("Could not get partial")?;
        partial.push(value);

        if let Some(size) = chunk_size {
            if partial.len() >= usize::try_from(size)? {
                output_map.insert("chunk".into(), Value::Array(partial.clone()));
                output_map.insert("partial".into(), Value::Array(vec![]));
            } else {
                output_map.insert("partial".into(), Value::Array(partial.clone()));
            }
            output_map.insert("chunk_size".into(), json!(size));
        } else {
            output_map.insert("partial".into(), Value::Array(partial.clone()));
            output_map.insert("chunk_size".into(), json!(0));
        }
    }

    Ok((Some(Value::Object(output_map)), RUN_AGAIN))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use super::inner_accumulate;

    #[test]
    fn accumulate_start_and_finish() {
        let value = json!(1);
        let partial = json!([]);
        let chunk_size = json!(1);

        let (result, _) =
            inner_accumulate(&[value, partial, chunk_size]).expect("_accumulate() failed");
        let output = result.expect("Could not get the Value from the output");
        assert_eq!(
            output
                .pointer("/chunk")
                .expect("Could not get the /chunk from the output"),
            &json!([1])
        );
    }

    #[test]
    fn accumulate_start_not_finish() {
        let value = json!(1);
        let partial = json!([]);
        let chunk_size = json!(2);

        let (result, _) =
            inner_accumulate(&[value, partial, chunk_size]).expect("_accumulate() failed");
        let output = result.expect("Could not get the Value from the output");
        assert_eq!(output.pointer("/chunk"), None);
        assert_eq!(
            output
                .pointer("/partial")
                .expect("Could not get the /partial from the output"),
            &json!([1])
        );
    }

    #[test]
    fn accumulate_started_then_finish() {
        let value = json!(2);
        let partial = json!([1]);
        let chunk_size = json!(2);

        let (result, _) =
            inner_accumulate(&[value, partial, chunk_size]).expect("_accumulate() failed");
        let output = result.expect("Could not get the Value from the output");
        assert_eq!(
            output
                .pointer("/chunk")
                .expect("Could not get the /chunk from the output"),
            &json!([1, 2])
        );
    }
}
