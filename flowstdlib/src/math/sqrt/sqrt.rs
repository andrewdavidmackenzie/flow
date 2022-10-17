use serde_json::{json, Value};
use serde_json::Value::Number;

use flowmacro::flow_function;

#[flow_function]
fn _sqrt(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    if let Number(ref a) = &inputs[0] {
        let num = a.as_f64().ok_or("Could not get num")?;
        Ok((Some(json!(num.sqrt())), RUN_AGAIN))
    } else {
        bail!("Input is not a number")
    }
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

    #[test]
    fn test_not_a_number() {
        let test_invalid_input = json!("Hello");
        assert!(_sqrt(&[test_invalid_input]).is_err());
    }
}
