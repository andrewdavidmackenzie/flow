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
fn inner_sqrt(a: f64) -> Result<(Option<Value>, RunAgain)> {
    flow_output!(numeric_json(a.sqrt()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use super::inner_sqrt;

    #[test]
    fn sqrt_integer_result() {
        let (root, again) = inner_sqrt(81.0).expect("sqrt failed");
        assert!(again);
        assert_eq!(root, Some(json!(9)));
    }

    #[test]
    fn sqrt_float_result() {
        let (root, _) = inner_sqrt(2.0).expect("sqrt failed");
        let val = root.expect("no output").as_f64().expect("not f64");
        assert!((val - std::f64::consts::SQRT_2).abs() < 1e-10);
    }
}
