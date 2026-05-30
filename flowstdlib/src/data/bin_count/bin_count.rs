use flowcore::errors::Result;
use flowcore::{RunAgain, RUN_AGAIN};
use flowmacro::flow_function;
use serde_json::{json, Value};

#[flow_function]
fn inner_bin_count(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let value = inputs.first().ok_or("Could not get value")?;
    let mut bins = inputs
        .get(1)
        .ok_or("Could not get bins")?
        .as_array()
        .ok_or("Could not get bins as array")?
        .clone();

    if value.is_null() {
        let mut output_map = serde_json::Map::new();
        output_map.insert("bins".into(), Value::Array(bins));
        return Ok((Some(Value::Object(output_map)), RUN_AGAIN));
    }

    let idx = usize::try_from(value.as_u64().ok_or("Could not get value as u64")?)
        .map_err(|_| "Value too large")?;

    if let Some(bin) = bins.get_mut(idx) {
        let count = bin.as_u64().unwrap_or(0) + 1;
        *bin = json!(count);
    }

    let mut output_map = serde_json::Map::new();
    output_map.insert("partial".into(), Value::Array(bins));
    Ok((Some(Value::Object(output_map)), RUN_AGAIN))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use super::inner_bin_count;

    #[test]
    fn count_single_value() {
        let bins = json!([0, 0, 0, 0]);
        let (result, _) = inner_bin_count(&[json!(2), bins]).expect("failed");
        let output = result.expect("no output");
        assert_eq!(
            *output.pointer("/partial").expect("no partial"),
            json!([0, 0, 1, 0])
        );
    }

    #[test]
    fn count_multiple() {
        let bins = json!([0, 0, 1, 0]);
        let (result, _) = inner_bin_count(&[json!(2), bins]).expect("failed");
        let output = result.expect("no output");
        assert_eq!(
            *output.pointer("/partial").expect("no partial"),
            json!([0, 0, 2, 0])
        );
    }

    #[test]
    fn null_flushes_bins() {
        let bins = json!([3, 1, 2, 0]);
        let (result, _) = inner_bin_count(&[json!(null), bins]).expect("failed");
        let output = result.expect("no output");
        assert_eq!(
            *output.pointer("/bins").expect("no bins"),
            json!([3, 1, 2, 0])
        );
    }
}
