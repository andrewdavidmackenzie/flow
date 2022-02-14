use flow_macro::flow_function;
use serde_json::{json, Value};
use serde_json::Value::Number;

#[flow_function]
fn _sqrt(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let input = &inputs[0];
    let mut value = None;

    if let Number(ref a) = input {
        let num = a.as_f64().ok_or("Could not get num")?;
        value = Some(json!(num.sqrt()));
    };

    Ok((value, RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::_sqrt;

    #[test]
    fn test_81() {
        let test_81 = json!(81);
        let test_9 = json!(9.0);
        let (root, again) = _sqrt(&[test_81]).expect("_sqrt() failed");

        assert!(again);
        assert_eq!(test_9, root.expect("Could not get the value from the output"));
    }
}
