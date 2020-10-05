use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::json;
use serde_json::Value;

#[derive(FlowImpl)]
/// Takes a value on it's input and sends the same value on it's output and adds one to the count
/// received on 'count' input and outputs new count on 'count' output
#[derive(Debug)]
pub struct Count;

impl Implementation for Count {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let data = inputs[0].clone();
        let mut count = inputs[1].as_i64().unwrap();
        count += 1;

        let mut output_map = serde_json::Map::new();

        output_map.insert("data".into(), data);
        output_map.insert("count".into(), json!(count));

        let output = Value::Object(output_map);

        (Some(output), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use flow_impl::Implementation;
    use serde_json::json;

    use super::Count;

    #[test]
    fn count_returns_value() {
        let data = json!(42);
        let count = json!(0);
        let inputs = vec!(data, count);

        let counter = Count {};
        let (result, _) = counter.run(&inputs);
        let output = result.unwrap();

        assert_eq!(output.pointer("/data").unwrap(), &json!(42));
        assert_eq!(output.pointer("/count").unwrap(), &json!(1));
    }
}