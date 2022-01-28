use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RUN_AGAIN, RunAgain};
use serde_json::json;
use serde_json::Value;

#[derive(FlowImpl)]
/// Takes a value on it's input and sends the same value on it's output and adds one to the count
/// received on 'count' input and outputs new count on 'count' output
#[derive(Debug)]
pub struct Count;

impl Implementation for Count {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let mut output_map = serde_json::Map::new();
        output_map.insert("data".into(), inputs[0].clone());

        if let Some(mut count) = inputs[1].as_i64() {
            count += 1;
            output_map.insert("count".into(), json!(count));
        }

        let output = Value::Object(output_map);

        (Some(output), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use flowcore::Implementation;
    use serde_json::json;

    use super::Count;

    #[test]
    fn count_returns_value() {
        let data = json!(42);
        let count = json!(0);
        let inputs = vec![data, count];

        let counter = Count {};
        let (result, _) = counter.run(&inputs);
        let output = result.expect("Could not get the Value from the output");

        assert_eq!(output.pointer("/data").expect("Could not get the /data from the output"), &json!(42));
        assert_eq!(output.pointer("/count").expect("Could not get the /count from the output"), &json!(1));
    }
}
