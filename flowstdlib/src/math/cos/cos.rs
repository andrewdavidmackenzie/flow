use flowcore::numeric_json;
use serde_json::Value;

use flowcore::errors::Result;
use flowcore::flow_output;
use flowcore::RunAgain;
use flowmacro::flow_function;

#[flow_function]
fn inner_cos(a: f64) -> Result<(Option<Value>, RunAgain)> {
    flow_output!(numeric_json(a.cos()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {

    use super::inner_cos;

    #[test]
    fn cos_zero_returns_integer() {
        let (result, _) = inner_cos(0.0).expect("failed");
        assert_eq!(result, Some(serde_json::json!(1)));
    }

    #[test]
    fn cos_pi_returns_integer() {
        let (result, _) = inner_cos(std::f64::consts::PI).expect("failed");
        assert_eq!(result, Some(serde_json::json!(-1)));
    }
}
