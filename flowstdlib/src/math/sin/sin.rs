use serde_json::{json, Value};

use flowcore::errors::Result;
use flowcore::flow_output;
use flowcore::RunAgain;
use flowmacro::flow_function;

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::float_cmp
)]
fn numeric_json(f: f64) -> Value {
    if f.fract() == 0.0 && f.abs() < i64::MAX as f64 {
        let i = f as i64;
        if (i as f64) == f {
            return json!(i);
        }
    }
    json!(f)
}

#[flow_function]
fn inner_sin(a: f64) -> Result<(Option<Value>, RunAgain)> {
    flow_output!(numeric_json(a.sin()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use super::inner_sin;

    #[test]
    fn sin_zero() {
        let (result, _) = inner_sin(0.0).expect("failed");
        assert_eq!(result, Some(json!(0)));
    }

    #[test]
    fn sin_pi_half() {
        let (result, _) = inner_sin(std::f64::consts::FRAC_PI_2).expect("failed");
        let val = result.expect("no output").as_f64().expect("not f64");
        assert!((val - 1.0).abs() < 1e-10);
    }
}
