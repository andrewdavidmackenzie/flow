use flowmacro::flow_function;
use serde_json::json;
use serde_json::Value;
use serde_json::Value::String as JsonString;

#[flow_function]
fn _reverse(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let mut value = None;

    let input = &inputs[0];
    if let JsonString(ref s) = input {
        value = Some(json!({
            "reversed" : s.chars().rev().collect::<String>(),
            "original": s
        }));
    }

    Ok((value, RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use flowcore::RUN_AGAIN;
    use serde_json::json;

    use super::_reverse;

    #[test]
    fn test_reverse() {
        let inputs = vec![json!("Hello"), json!(true)];
        let (output, run_again) = _reverse(&inputs).expect("_reverse() failed");
        assert_eq!(run_again, RUN_AGAIN);

        assert!(output.is_some());
        let value = output.expect("No value was returned in the output");
        let map = value.as_object().expect("Expected a Map json object");
        assert_eq!(
            map.get("original").expect("No 'original' value in map"),
            &json!("Hello")
        );
        assert_eq!(
            map.get("reversed").expect("No 'reversed' value in map"),
            &json!("olleH")
        );
    }
}
