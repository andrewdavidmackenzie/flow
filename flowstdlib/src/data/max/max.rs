use flowcore::errors::Result;
use flowcore::flow_output;
use flowcore::RunAgain;
use flowmacro::flow_function;
use serde_json::{json, Value};

#[flow_function]
fn inner_max(value: &Value, partial: &Value) -> Result<(Option<Value>, RunAgain)> {
    if value.is_null() {
        return flow_output!("result" => partial.clone());
    }

    let v = value.as_f64().ok_or("value not f64")?;
    let r = partial.as_f64().ok_or("partial not f64")?;
    let new_max = if v > r { v } else { r };

    flow_output!("partial" => json!(new_max))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use super::inner_max;
    use serde_json::json;

    #[test]
    fn tracks_maximum() {
        let (r, _) = inner_max(&json!(200), &json!(0)).expect("failed");
        assert_eq!(*r.unwrap().pointer("/partial").unwrap(), json!(200.0));
    }

    #[test]
    fn null_outputs_result() {
        let (r, _) = inner_max(&json!(null), &json!(200)).expect("failed");
        assert_eq!(*r.unwrap().pointer("/result").unwrap(), json!(200));
    }
}
