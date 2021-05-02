use serde_json::json;
use serde_json::Value;

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};

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
    use serde_json::json;

    use flowcore::Implementation;

    use super::Count;

    #[test]
    fn count_returns_value() {
        let data = json!(42);
        let count = json!(0);
        let inputs = vec![data, count];

        let counter = Count {};
        let (result, _) = counter.run(&inputs);
        let output = result.unwrap();

        assert_eq!(output.pointer("/data").unwrap(), &json!(42));
        assert_eq!(output.pointer("/count").unwrap(), &json!(1));
    }
}
