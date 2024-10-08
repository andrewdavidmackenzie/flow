use serde_json::{json, Value};
use serde_json::Value::Number;

use flowcore::{RUN_AGAIN, RunAgain};
use flowcore::errors::{Result, bail};
use flowmacro::flow_function;

#[flow_function]
fn inner_sqrt(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    if let Number(ref a) = inputs.first().ok_or("Could not get a")? {
        let num = a.as_f64().ok_or("Could not get num")?;
        Ok((Some(json!(num.sqrt())), RUN_AGAIN))
    } else {
        bail!("Input is not a number")
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::inner_sqrt;

    #[test]
    fn test_81() {
        let test_81 = json!(81);
        let test_9 = json!(9.0);
        let (root, again) = inner_sqrt(&[test_81]).expect("_sqrt() failed");

        assert!(again);
        assert_eq!(test_9, root.expect("Could not get the value from the output"));
    }

    #[test]
    fn test_not_a_number() {
        let test_invalid_input = json!("Hello");
        assert!(inner_sqrt(&[test_invalid_input]).is_err());
    }
}
