use serde_json::Value::Number;
use serde_json::{json, Value};

use flowcore::errors::{bail, Result};
use flowcore::{RunAgain, RUN_AGAIN};
use flowmacro::flow_function;

#[flow_function]
fn inner_tan(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    if let Number(ref a) = inputs.first().ok_or("Could not get input")? {
        let num = a.as_f64().ok_or("Could not get as f64")?;
        Ok((Some(json!(num.tan())), RUN_AGAIN))
    } else {
        bail!("Input is not a number")
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use super::inner_tan;

    #[test]
    fn tan_zero() {
        let (result, _) = inner_tan(&[json!(0.0)]).expect("failed");
        let val = result.expect("no output").as_f64().expect("not f64");
        assert!((val - 0.0).abs() < 1e-10);
    }

    #[test]
    fn tan_pi_quarter() {
        let (result, _) = inner_tan(&[json!(std::f64::consts::FRAC_PI_4)]).expect("failed");
        let val = result.expect("no output").as_f64().expect("not f64");
        assert!((val - 1.0).abs() < 1e-10);
    }

    #[test]
    fn tan_not_a_number() {
        assert!(inner_tan(&[json!("hello")]).is_err());
    }
}
