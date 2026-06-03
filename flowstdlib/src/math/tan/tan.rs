use flowcore::numeric_json;
use serde_json::Value;

use flowcore::errors::Result;
use flowcore::flow_output;
use flowcore::RunAgain;
use flowmacro::flow_function;

#[flow_function]
fn inner_tan(a: f64) -> Result<(Option<Value>, RunAgain)> {
    flow_output!(numeric_json(a.tan()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {

    use super::inner_tan;

    #[test]
    fn tan_zero_returns_integer() {
        let (result, _) = inner_tan(0.0).expect("failed");
        assert_eq!(result, Some(serde_json::json!(0)));
    }

    #[test]
    fn tan_pi_quarter() {
        let (result, _) = inner_tan(std::f64::consts::FRAC_PI_4).expect("failed");
        let val = result.expect("no output").as_f64().expect("not f64");
        assert!((val - 1.0).abs() < 1e-10);
    }
}
