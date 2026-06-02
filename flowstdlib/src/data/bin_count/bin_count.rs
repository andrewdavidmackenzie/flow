use flowcore::errors::Result;
use flowcore::flow_output;
use flowcore::RunAgain;
use flowmacro::flow_function;
use serde_json::{json, Value};

#[flow_function]
fn inner_bin_count(value: &Value, partial: &Value) -> Result<(Option<Value>, RunAgain)> {
    let mut bins = partial
        .as_array()
        .ok_or("Could not get bins as array")?
        .clone();

    if value.is_null() {
        return flow_output!("bins" => Value::Array(bins));
    }

    let idx = usize::try_from(value.as_u64().ok_or("Could not get value as u64")?)
        .map_err(|_| "Value too large")?;

    if let Some(bin) = bins.get_mut(idx) {
        let count = bin.as_u64().unwrap_or(0) + 1;
        *bin = json!(count);
    }

    flow_output!("partial" => Value::Array(bins))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use super::inner_bin_count;

    #[test]
    fn count_single_value() {
        let (result, _) = inner_bin_count(&json!(2), &json!([0, 0, 0, 0])).expect("failed");
        assert_eq!(
            *result
                .expect("no output")
                .pointer("/partial")
                .expect("no partial"),
            json!([0, 0, 1, 0])
        );
    }

    #[test]
    fn count_multiple() {
        let (result, _) = inner_bin_count(&json!(2), &json!([0, 0, 1, 0])).expect("failed");
        assert_eq!(
            *result
                .expect("no output")
                .pointer("/partial")
                .expect("no partial"),
            json!([0, 0, 2, 0])
        );
    }

    #[test]
    fn null_flushes_bins() {
        let (result, _) = inner_bin_count(&json!(null), &json!([3, 1, 2, 0])).expect("failed");
        assert_eq!(
            *result
                .expect("no output")
                .pointer("/bins")
                .expect("no bins"),
            json!([3, 1, 2, 0])
        );
    }
}
