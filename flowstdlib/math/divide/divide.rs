use flowmacro::flow_function;
use serde_json::{json, Value};

#[flow_function]
fn _divide(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let mut output_map = serde_json::Map::new();

    let dividend = inputs[0].as_f64().ok_or("Could not get dividend")?;
    let divisor = inputs[1].as_f64().ok_or("Could not get divisor")?;
    output_map.insert("dividend".into(), json!(dividend));
    output_map.insert("divisor".into(), json!(divisor));
    output_map.insert("result".into(), json!(dividend / divisor));
    output_map.insert("remainder".into(), json!(dividend % divisor));

    Ok((Some(Value::Object(output_map)), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::{json, Value};

    use super::_divide;

    fn do_divide(test_data: (u32, u32, f64, u32)) {
        // Create input vector
        let dividend = json!(test_data.0);
        let divisor = json!(test_data.1);
        let inputs: Vec<Value> = vec![dividend, divisor];

        let (output, run_again) = _divide(&inputs).expect("_divide() failed");
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
