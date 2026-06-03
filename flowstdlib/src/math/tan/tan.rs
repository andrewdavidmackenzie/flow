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
fn inner_tan(a: f64) -> Result<(Option<Value>, RunAgain)> {
    flow_output!(numeric_json(a.tan()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {

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
