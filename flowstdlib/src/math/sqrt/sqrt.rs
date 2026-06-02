use serde_json::{json, Value};

use flowcore::errors::Result;
use flowcore::flow_output;
use flowcore::RunAgain;
use flowmacro::flow_function;

#[flow_function]
fn inner_sqrt(a: f64) -> Result<(Option<Value>, RunAgain)> {
    flow_output!(json!(a.sqrt()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use super::inner_sqrt;

    #[test]
    fn test_81() {
        let (root, again) = inner_sqrt(81.0).expect("_sqrt() failed");
        assert!(again);
        assert_eq!(root, Some(json!(9.0)));
    }
}
