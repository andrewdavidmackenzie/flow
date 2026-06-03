use super::numeric_json;
use serde_json::Value;

use flowcore::errors::Result;
use flowcore::flow_output;
use flowcore::RunAgain;
use flowmacro::flow_function;

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
