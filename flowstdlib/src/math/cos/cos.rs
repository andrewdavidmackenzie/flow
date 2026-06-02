use serde_json::{json, Value};

use flowcore::errors::Result;
use flowcore::flow_output;
use flowcore::RunAgain;
use flowmacro::flow_function;

#[flow_function]
fn inner_cos(a: f64) -> Result<(Option<Value>, RunAgain)> {
    flow_output!(json!(a.cos()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {

    use super::inner_cos;

    #[test]
    fn cos_zero() {
        let (result, _) = inner_cos(0.0).expect("failed");
        let val = result.expect("no output").as_f64().expect("not f64");
        assert!((val - 1.0).abs() < 1e-10);
    }

    #[test]
    fn cos_pi() {
        let (result, _) = inner_cos(std::f64::consts::PI).expect("failed");
        let val = result.expect("no output").as_f64().expect("not f64");
        assert!((val - (-1.0)).abs() < 1e-10);
    }
}
