use serde_json::json;
use serde_json::Value;

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};

#[derive(FlowImpl)]
/// Multiply one input by another
#[derive(Debug)]
pub struct Multiply;

impl Implementation for Multiply {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let mut output = None;

        if let Some(i1) = inputs[0].as_u64() {
            if let Some(i2) = inputs[1].as_u64() {
                let result = i1 * i2;
                output = Some(json!(result));
            }
        }

        (output, RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use serde_json::{json, Value};

    use flowcore::Implementation;

    use super::Multiply;

    fn do_multiply(test_data: (u32, u32, u32)) {
        let multiplier: &dyn Implementation = &Multiply {} as &dyn Implementation;

        // Create input vector
        let i1 = json!(test_data.0);
        let i2 = json!(test_data.1);
        let inputs: Vec<Value> = vec![i1, i2];

        let (output, run_again) = multiplier.run(&inputs);
        assert!(run_again);

        let value = output.unwrap();
        assert_eq!(value, Value::Number(serde_json::Number::from(test_data.2)));
    }

    #[test]
    fn test_divide() {
        let test_set = vec![(3, 3, 9), (33, 3, 99)];

        for test in test_set {
            do_multiply(test);
        }
    }
}
