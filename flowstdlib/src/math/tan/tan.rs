use serde_json::{json, Value};

use flowcore::errors::Result;
use flowcore::{RunAgain, RUN_AGAIN};
use flowmacro::flow_function;

#[allow(clippy::unnecessary_wraps)]
#[flow_function]
fn inner_tan(a: f64) -> Result<(Option<Value>, RunAgain)> {
    Ok((Some(json!(a.tan())), RUN_AGAIN))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use super::inner_tan;

    #[test]
    fn tan_zero() {
        let (result, _) = inner_tan(0.0).expect("failed");
        let val = result.expect("no output").as_f64().expect("not f64");
        assert!(val.abs() < 1e-10);
    }

    #[test]
    fn tan_pi_quarter() {
        let (result, _) = inner_tan(std::f64::consts::FRAC_PI_4).expect("failed");
        let val = result.expect("no output").as_f64().expect("not f64");
        assert!((val - 1.0).abs() < 1e-10);
    }
}
