use flowcore::errors::Result;
use flowcore::{RunAgain, RUN_AGAIN};
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
        let mut m = serde_json::Map::new();
        m.insert("result".into(), json!(avg));
        m.insert("count".into(), json!(partial_count));
        return Ok((Some(Value::Object(m)), RUN_AGAIN));
    }

    let v = value.as_f64().ok_or("value not f64")?;
    let mut m = serde_json::Map::new();
    m.insert("partial_sum".into(), json!(partial_sum + v));
    m.insert("partial_count".into(), json!(partial_count + 1.0));
    Ok((Some(Value::Object(m)), RUN_AGAIN))
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
