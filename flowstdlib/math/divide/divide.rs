use serde_json::{json, Value};

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};

#[derive(FlowImpl)]
/// Divide one input by another, producing outputs for the dividend, divisor, result and the remainder
#[derive(Debug)]
pub struct Divide;

impl Implementation for Divide {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let mut output_map = serde_json::Map::new();

        if let Some(dividend) = inputs[0].as_f64() {
            if let Some(divisor) = inputs[1].as_f64() {
                output_map.insert("dividend".into(), json!(dividend));
                output_map.insert("divisor".into(), json!(divisor));
                output_map.insert("result".into(), json!(dividend / divisor));
                output_map.insert("remainder".into(), json!(dividend % divisor));
            }
        }

        let output = Value::Object(output_map);

        (Some(output), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use serde_json::{json, Value};

    use flowcore::Implementation;

    use super::Divide;

    fn do_divide(test_data: (u32, u32, f64, u32)) {
        let divide: &dyn Implementation = &Divide {} as &dyn Implementation;

        // Create input vector
        let dividend = json!(test_data.0);
        let divisor = json!(test_data.1);
        let inputs: Vec<Value> = vec![dividend, divisor];

        let (output, run_again) = divide.run(&inputs);
        assert!(run_again);

        let outputs = output.expect("Could not get the output value");

        let dividend = outputs
            .pointer("/dividend")
            .expect("Could not get /dividend");
        assert_eq!(dividend, &json!(test_data.0 as f64));

        let divisor = outputs.pointer("/divisor").expect("Could not get /divisor");
        assert_eq!(divisor, &json!(test_data.1 as f64));

        let result = outputs.pointer("/result").expect("Could not get /result");
        assert_eq!(result, &json!(test_data.2 as f64));

        let remainder = outputs
            .pointer("/remainder")
            .expect("Could not get /remainder");
        assert_eq!(remainder, &json!(test_data.3 as f64));
    }

    #[test]
    fn test_divide() {
        let test_set = vec![(100, 3, 33.333_333_333_333_336_f64, 1), (99, 3, 33f64, 0)];

        for test in test_set {
            do_divide(test);
        }
    }
}
