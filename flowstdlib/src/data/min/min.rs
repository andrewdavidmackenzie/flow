use flowcore::errors::Result;
use flowcore::{RunAgain, RUN_AGAIN};
use flowmacro::flow_function;
use serde_json::{json, Value};

#[flow_function]
fn inner_min(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let value = inputs.first().ok_or("Could not get value")?;
    let running = inputs.get(1).ok_or("Could not get running min")?;

    if value.is_null() {
        let mut m = serde_json::Map::new();
        m.insert("result".into(), running.clone());
        return Ok((Some(Value::Object(m)), RUN_AGAIN));
    }

    let v = value.as_f64().ok_or("value not f64")?;
    let r = running.as_f64().ok_or("running not f64")?;
    let new_min = if v < r { v } else { r };

    let mut m = serde_json::Map::new();
    m.insert("partial".into(), json!(new_min));
    Ok((Some(Value::Object(m)), RUN_AGAIN))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use super::inner_min;
    use serde_json::json;

    #[test]
    fn tracks_minimum() {
        let (r, _) = inner_min(&[json!(50), json!(255)]).expect("failed");
        assert_eq!(*r.unwrap().pointer("/partial").unwrap(), json!(50.0));

        let (r, _) = inner_min(&[json!(200), json!(50)]).expect("failed");
        assert_eq!(*r.unwrap().pointer("/partial").unwrap(), json!(50.0));
    }

    #[test]
    fn null_outputs_result() {
        let (r, _) = inner_min(&[json!(null), json!(42)]).expect("failed");
        assert_eq!(*r.unwrap().pointer("/result").unwrap(), json!(42));
    }
}
