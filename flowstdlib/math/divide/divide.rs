use serde_json::Value;

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};

#[derive(FlowImpl)]
/// Divide one input by another, producing outputs for the dividend, divisor, result and the remainder
#[derive(Debug)]
pub struct Divide;

impl Implementation for Divide {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let dividend = inputs[0].as_f64().unwrap();
        let divisor = inputs[1].as_f64().unwrap();

        let mut output_map = serde_json::Map::new();
        output_map.insert(
            "dividend".into(),
            Value::Number(serde_json::Number::from_f64(dividend).unwrap()),
        );
        output_map.insert(
            "divisor".into(),
            Value::Number(serde_json::Number::from_f64(divisor).unwrap()),
        );
        output_map.insert(
            "result".into(),
            Value::Number(serde_json::Number::from_f64(dividend / divisor).unwrap()),
        );
        output_map.insert(
            "remainder".into(),
            Value::Number(serde_json::Number::from_f64(dividend % divisor).unwrap()),
        );
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

        let outputs = output.unwrap();

        let dividend = outputs.pointer("/dividend").unwrap();
        assert_eq!(
            dividend,
            &Value::Number(serde_json::Number::from_f64(test_data.0 as f64).unwrap())
        );

        let divisor = outputs.pointer("/divisor").unwrap();
        assert_eq!(
            divisor,
            &Value::Number(serde_json::Number::from_f64(test_data.1 as f64).unwrap())
        );

        let result = outputs.pointer("/result").unwrap();
        assert_eq!(
            result,
            &Value::Number(serde_json::Number::from_f64(test_data.2 as f64).unwrap())
        );

        let remainder = outputs.pointer("/remainder").unwrap();
        assert_eq!(
            remainder,
            &Value::Number(serde_json::Number::from_f64(test_data.3 as f64).unwrap())
        );
    }

    #[test]
    fn test_divide() {
        let test_set = vec![(100, 3, 33.333_333_333_333_336_f64, 1), (99, 3, 33f64, 0)];

        for test in test_set {
            do_divide(test);
        }
    }
}
