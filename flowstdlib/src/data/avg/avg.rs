use flowcore::errors::Result;
use flowcore::flow_output;
use flowcore::RunAgain;
use flowmacro::flow_function;
use serde_json::{json, Value};

#[flow_function]
fn inner_avg(
    value: &Value,
    partial_sum: f64,
    partial_count: f64,
) -> Result<(Option<Value>, RunAgain)> {
    if value.is_null() {
        let avg = if partial_count > 0.0 {
            partial_sum / partial_count
        } else {
            0.0
        };
        return flow_output!(
            "result" => json!(avg),
            "count" => json!(partial_count),
        );
    }

    let v = value.as_f64().ok_or("value not f64")?;
    flow_output!(
        "partial_sum" => json!(partial_sum + v),
        "partial_count" => json!(partial_count + 1.0),
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use super::inner_avg;
    use serde_json::json;

    #[test]
    fn accumulates() {
        let (r, _) = inner_avg(&json!(10), 0.0, 0.0).expect("failed");
        let o = r.unwrap();
        assert_eq!(*o.pointer("/partial_sum").unwrap(), json!(10.0));
        assert_eq!(*o.pointer("/partial_count").unwrap(), json!(1.0));
    }

    #[test]
    fn null_outputs_average() {
        let (r, _) = inner_avg(&json!(null), 30.0, 3.0).expect("failed");
        assert_eq!(*r.unwrap().pointer("/result").unwrap(), json!(10.0));
    }
}
