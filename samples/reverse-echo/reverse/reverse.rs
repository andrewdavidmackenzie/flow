use flow_macro::flow_function;
use serde_json::json;
use serde_json::Value;

#[flow_function]
fn _reverse(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let mut value = None;

    if inputs.len() == 1 {
        let input = &inputs[0];
        if let Value::String(ref s) = input {
            value = Some(json!(s.chars().rev().collect::<String>()));
        }
        Ok((value, RUN_AGAIN))
    } else {
        Err(Error::from("Incorrect number of inputs for reverse()"))
    }
}

#[cfg(test)]
mod test {
    use flowcore::RUN_AGAIN;
    use serde_json::json;

    use super::_reverse;

    #[test]
    fn invalid_input() {
        assert!(_reverse(&[]).is_err());
    }

    #[test]
    fn send_string() {
        let string = "string of text";
        let value = json!(string);
        let (value, run_again) = _reverse(&[value]).expect("_reverse() failed");

        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, Some(json!("txet fo gnirts")));
    }
}
