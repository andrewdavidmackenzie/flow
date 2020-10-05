use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// Accumulate input values into an array upto the limit specified
#[derive(Debug)]
pub struct Accumulate;

impl Implementation for Accumulate {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let value = inputs[0].clone(); // input value to accumulate in array
        let mut output_map = serde_json::Map::new();

        if value.is_null() {
            output_map.insert("chunk".into(), Value::Null);
        } else {
            let mut partial_input = inputs[1].clone(); // A partial array to append the values to
            let chunk_size = inputs[2].clone(); // how many elements desired in the output array

            let partial = partial_input.as_array_mut().unwrap();
            partial.push(value);

            if partial.len() >= chunk_size.as_u64().unwrap() as usize {
                // TODO could pass on any extra elements beyond chunk size in 'partial'
                // and also force chunk size to be exact....
                output_map.insert("chunk".into(), Value::Array(partial.clone()));
                output_map.insert("partial".into(), Value::Array(vec!()));
            } else {
                output_map.insert("partial".into(), Value::Array(partial.clone()));
            }

            output_map.insert("chunk_size".into(), chunk_size);
        }

        (Some(Value::Object(output_map)), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use flow_impl::Implementation;
    use serde_json::json;

    #[test]
    fn accumulate_start_and_finish() {
        let value = json!(1);
        let partial = json!([]);
        let chunk_size = json!(1);

        let accumulator = super::Accumulate {};
        let (result, _) = accumulator.run(&[value, partial, chunk_size]);
        let output = result.unwrap();
        assert_eq!(output.pointer("/chunk").unwrap(), &json!([1]));
    }

    #[test]
    fn accumulate_start_not_finish() {
        let value = json!(1);
        let partial = json!([]);
        let chunk_size = json!(2);

        let accumulator = super::Accumulate {};
        let (result, _) = accumulator.run(&[value, partial, chunk_size]);
        let output = result.unwrap();
        assert_eq!(output.pointer("/chunk"), None);
        assert_eq!(output.pointer("/partial").unwrap(), &json!([1]));
    }

    #[test]
    fn accumulate_started_then_finish() {
        let value = json!(2);
        let partial = json!([1]);
        let chunk_size = json!(2);

        let accumulator = super::Accumulate {};
        let (result, _) = accumulator.run(&[value, partial, chunk_size]);
        let output = result.unwrap();
        assert_eq!(output.pointer("/chunk").unwrap(), &json!([1, 2]));
    }
}