use flowcore::errors::Result;
use flowcore::flow_output;
use flowcore::RunAgain;
use flowmacro::flow_function;
use serde_json::{json, Value};

#[flow_function]
fn inner_count(data: &Value, count: i64) -> Result<(Option<Value>, RunAgain)> {
    let _ = data;
    flow_output!(json!(count + 1))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use super::inner_count;

    #[test]
    fn count_returns_value() {
        let (result, _) = inner_count(&json!(42), 0).expect("_count() failed");
        assert_eq!(result, Some(json!(1)));
    }
}
