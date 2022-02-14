use flow_macro::flow_function;
use serde_json::Value;

#[flow_function]
fn _tap(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let mut value = None;
    let data = &inputs[0];
    let control = &inputs[1].as_bool().ok_or("Could not get bool")?;

    if *control {
        value = Some(data.clone());
    }

    Ok((value, RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use flowcore::RUN_AGAIN;
    use serde_json::json;

    use super::_tap;

    #[test]
    fn test_tap_go() {
        let inputs = vec![json!("A"), json!(true)];
        let (output, run_again) = _tap(&inputs).expect("_tap() failed");
        assert_eq!(run_again, RUN_AGAIN);

        assert!(output.is_some());
        let value = output.expect("Could not get output value");
        assert_eq!(value, json!("A"));
    }

    #[test]
    fn test_tap_no_go() {
        let inputs = vec![json!("A"), json!(false)];
        let (output, run_again) = _tap(&inputs).expect("_tap() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert!(output.is_none());
    }
}
