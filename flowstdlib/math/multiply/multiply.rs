use flow_macro::flow_function;
use serde_json::json;
use serde_json::Value;

#[flow_function]
fn _multiply(inputs: &[Value]) -> (Option<Value>, RunAgain) {
    let mut output = None;

    if let Some(i1) = inputs[0].as_u64() {
        if let Some(i2) = inputs[1].as_u64() {
            let result = i1 * i2;
            output = Some(json!(result));
        }
    }

    (output, RUN_AGAIN)
}

#[cfg(test)]
mod test {
    use serde_json::{json, Value};
    use super::_multiply;

    fn do_multiply(test_data: (u32, u32, u32)) {
        // Create input vector
        let i1 = json!(test_data.0);
        let i2 = json!(test_data.1);
        let inputs: Vec<Value> = vec![i1, i2];

        let (output, run_again) = _multiply(&inputs);
        assert!(run_again);

        let value = output.expect("Could not get the value from the output");
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
